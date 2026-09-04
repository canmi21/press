//! Deriving everything an article needs from one original image.
//!
//! The original never leaves this machine by default. What gets published are the derived
//! variants, each addressed by the hash of its own bytes, so a key can never denote different
//! content than it did before. See spec/architecture/data.md.

pub mod encode;
pub mod exif;
pub mod geo;
pub mod ladder;
pub mod manifest;
pub mod run;
pub mod store;

use encode::Format;
use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{PixelType, ResizeOptions, Resizer};
use image::DynamicImage;
use ladder::Size;
use manifest::Media;
use std::path::Path;

/// A content id: BLAKE3 truncated to 128 bits, hex encoded.
///
/// Truncation leaves roughly 64-bit collision resistance, which is far more than addressing
/// a lifetime of personal assets needs and deliberately not a tamper-evidence claim. Unrelated
/// to an IPFS CID, which is a structured multihash rather than a bare digest.
pub fn cid(bytes: &[u8]) -> String {
	let digest = blake3::hash(bytes);
	digest.to_hex()[..32].to_string()
}

#[derive(Debug, Clone)]
pub struct Variant {
	pub cid: String,
	pub bytes: Vec<u8>,
	pub width: u32,
	pub height: u32,
	pub format: Format,
}

#[derive(Debug)]
pub struct Derived {
	/// Content id of the original bytes, and the identity of the asset as a whole.
	pub cid: String,
	pub width: u32,
	pub height: u32,
	/// Thumbhash bytes, 19 of them for any image.
	pub thumb: Vec<u8>,
	pub variants: Vec<Variant>,
}

/// One image after every decision has been made and before anything is written.
#[derive(Debug)]
pub struct Prepared {
	pub derived: Derived,
	pub media: Media,
}

#[derive(Debug)]
pub enum Error {
	Read(std::io::Error),
	Decode,
	Encode(encode::Error),
	Serialize(serde_json::Error),
	Write(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Read(error) => write!(f, "could not read: {error}"),
			Self::Decode => write!(f, "not a readable image"),
			Self::Encode(error) => write!(f, "{error}"),
			Self::Serialize(error) => write!(f, "could not encode record: {error}"),
			Self::Write(error) => write!(f, "could not write: {error}"),
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Read(error) | Self::Write(error) => Some(error),
			Self::Serialize(error) => Some(error),
			Self::Decode | Self::Encode(_) => None,
		}
	}
}

/// Decode whatever was handed over.
///
/// The `image` crate covers everything but HEIC, which is HEVC inside a HEIF container --
/// the same container AVIF uses with a different codec inside, so support for one says
/// nothing about the other. That path goes through a pure-Rust decoder rather than bindings
/// to libheif: this runs on one machine and in CI, and a system library is a thing to install
/// in both. Measured at 249ms for a 4032x2268 frame, which is nothing against the AV1 encode
/// that follows.
///
/// Only the primary image. An iPhone HEIC may also carry a depth map, gain map and the frames
/// of a live photo; none of those are wanted yet.
fn load(original: &[u8]) -> Result<DynamicImage, Error> {
	if is_heic(original) {
		let out = heic::DecoderConfig::new()
			.decode(original, heic::PixelLayout::Rgba8)
			.map_err(|_| Error::Decode)?;
		return image::RgbaImage::from_raw(out.width, out.height, out.data)
			.map(DynamicImage::ImageRgba8)
			.ok_or(Error::Decode);
	}
	image::load_from_memory(original).map_err(|_| Error::Decode)
}

/// Whether this is HEIF carrying HEVC, by its `ftyp` brand.
///
/// Read from the container rather than the filename: the extension is whatever the file was
/// called when it arrived, and `data/image` holds files named by hash.
fn is_heic(bytes: &[u8]) -> bool {
	if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
		return false;
	}
	matches!(
		&bytes[8..12],
		b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"hevm" | b"hevs" | b"mif1"
	)
}

/// Everything published for one original: its identity, its placeholder, and one variant per
/// size and format.
pub fn derive(original: &[u8], keep_original: bool) -> Result<Derived, Error> {
	let image = load(original)?;
	let size = Size::new(image.width(), image.height());
	let formats = encode::formats_for(&image);

	let mut variants = Vec::new();
	for target in ladder::ladder(size, keep_original) {
		let resized = resize(&image, target);
		for &format in &formats {
			let bytes = encode::encode(&resized, format).map_err(Error::Encode)?;
			variants.push(Variant {
				cid: cid(&bytes),
				width: target.width,
				height: target.height,
				format,
				bytes,
			});
		}
	}

	// Only the hash is kept. The decoded form the site inlines is produced at build time from
	// this, so storing one here would be the same picture written twice.
	let thumb = placeholder(&image)?;
	Ok(Derived { cid: cid(original), width: size.width, height: size.height, thumb, variants })
}

/// Derive one source into a value ready to write, without touching the published tree.
pub fn derive_for(
	original: &[u8],
	source_mime: &str,
	previous: Option<&Media>,
	keep_original: bool,
	gazetteer: Option<&geo::Gazetteer>,
) -> Result<Prepared, Error> {
	let derived = derive(original, keep_original)?;
	// Read once, at import. The published variants are stripped, so this is the only place the
	// camera's account of the picture survives.
	let mut metadata = exif::read(original);
	// The address is derived from the recorded position rather than read from the file.
	if let Some(found) = metadata.as_mut()
		&& let Some(location) = found.location.clone()
		&& let (Some(lat), Some(lon)) = (location.latitude, location.longitude)
		&& let Some(gazetteer) = gazetteer
	{
		found.address = gazetteer.lookup(lat, lon);
	}
	let media = manifest::media_for(
		&derived,
		source_mime,
		original.len() as u64,
		previous.map(|media| media.created.as_str()),
		metadata,
	);
	Ok(Prepared { derived, media })
}

/// Write a completed derivation. No decoding or record decisions happen here.
pub fn write_derived(public: &Path, prepared: &Prepared) -> Result<(), Error> {
	for variant in &prepared.derived.variants {
		let target = store::variant_path(public, &variant.cid, variant.format.extension());
		store::write(&target, &variant.bytes).map_err(Error::Write)?;
	}

	let document = manifest::Document { version: manifest::VERSION, media: prepared.media.clone() };
	let json = serde_json::to_string_pretty(&document).map_err(Error::Serialize)?;
	store::write(&store::meta_path(public, &prepared.derived.cid), json.as_bytes())
		.map_err(Error::Write)
}

/// Derive and publish one image, preserving its first-seen timestamp when it already exists.
pub fn publish(
	original: &[u8],
	source_mime: &str,
	public: &Path,
	previous: Option<&Media>,
	keep_original: bool,
	gazetteer: Option<&geo::Gazetteer>,
) -> Result<Media, Error> {
	let prepared = derive_for(original, source_mime, previous, keep_original, gazetteer)?;
	let media = prepared.media.clone();
	write_derived(public, &prepared)?;
	Ok(media)
}

/// Derive and store one source synchronously, returning the content id an editor should insert.
pub fn store_one(repository: &Path, source: &Path, keep_original: bool) -> Result<String, Error> {
	let bytes = std::fs::read(source).map_err(Error::Read)?;
	let id = cid(&bytes);
	let merged_path = repository.join(run::MERGED);
	let mut merged = run::load(&merged_path).map_err(Error::Read)?;

	// The returned id may be inserted into an article immediately. Published bytes and records
	// must exist first so a crash can only leave an unreferenced image. See spec/tasks.md.
	let media = publish(
		&bytes,
		mime_of(source),
		&repository.join("data/public"),
		merged.media.get(&id),
		keep_original,
		geo::Gazetteer::open(repository).as_ref(),
	)?;
	merged.media.insert(id.clone(), media);
	merged.updated = manifest::now();
	let json = serde_json::to_string_pretty(&merged).map_err(Error::Serialize)?;
	store::write(&merged_path, format!("{json}\n").as_bytes()).map_err(Error::Write)?;

	Ok(id)
}

pub(crate) fn mime_of(path: &Path) -> &'static str {
	match path
		.extension()
		.and_then(|extension| extension.to_str())
		.unwrap_or_default()
		.to_ascii_lowercase()
		.as_str()
	{
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"webp" => "image/webp",
		"avif" => "image/avif",
		"gif" => "image/gif",
		"heic" | "heif" => "image/heic",
		_ => "application/octet-stream",
	}
}

fn resize(image: &DynamicImage, target: Size) -> DynamicImage {
	if target.width == image.width() && target.height == image.height() {
		return image.clone();
	}
	let mut destination = FirImage::new(target.width, target.height, PixelType::U8x4);
	if Resizer::new().resize(image, &mut destination, &ResizeOptions::new()).is_err() {
		return image.clone();
	}
	image::RgbaImage::from_raw(target.width, target.height, destination.into_vec())
		.map(DynamicImage::ImageRgba8)
		.unwrap_or_else(|| image.clone())
}

/// The thumbhash and a tiny image decoded from it.
///
/// Both are kept: the hash is the compact canonical form, and the decoded image is what gets
/// inlined into an article so a page paints its placeholder with no request, no decoder
/// script, and no dependence on JavaScript having run.
/// The compact hash a page paints before any image arrives.
///
/// Only the hash. It used to also return a decoded, re-encoded copy for inlining, which the
/// site build now produces from this -- one picture, one stored form.
fn placeholder(image: &DynamicImage) -> Result<Vec<u8>, Error> {
	// thumbhash reads a small input by design; anything larger is wasted work.
	let small = resize(image, Size::new(image.width(), image.height()).scaled_to_long_edge(100));
	let rgba = small.to_rgba8();
	Ok(thumbhash::rgba_to_thumb_hash(rgba.width() as usize, rgba.height() as usize, rgba.as_raw()))
}

#[cfg(test)]
mod tests {
	use super::*;
	use image::{Rgba, RgbaImage};

	/// A directory that removes itself, however the test ends.
	///
	/// `TempDir` deletes on drop, which the hand-rolled predecessor could not: a panicking test
	/// left its directory behind, and the name carried the process id because two tests choosing
	/// the same one would otherwise share a directory. Both problems belonged to the workaround.
	fn temp() -> tempfile::TempDir {
		tempfile::tempdir().expect("temp")
	}

	fn photo(width: u32, height: u32) -> Vec<u8> {
		let mut buffer = RgbaImage::new(width, height);
		let mut state: u32 = 0x9e37_79b9;
		for pixel in buffer.pixels_mut() {
			state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
			let [r, g, b, _] = state.to_le_bytes();
			*pixel = Rgba([r, g, b, 255]);
		}
		let mut out = Vec::new();
		DynamicImage::ImageRgba8(buffer)
			.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
			.expect("encode fixture");
		out
	}

	#[test]
	fn a_cid_is_thirty_two_hex_characters() {
		let value = cid(b"anything");
		assert_eq!(value.len(), 32);
		assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
	}

	#[test]
	fn identical_bytes_give_identical_cids() {
		// This is what makes a key immutable by construction rather than by promise.
		assert_eq!(cid(b"same"), cid(b"same"));
		assert_ne!(cid(b"same"), cid(b"other"));
	}

	#[test]
	fn rejects_something_that_is_not_an_image() {
		assert!(matches!(derive(b"not an image", false), Err(Error::Decode)));
	}

	#[test]
	fn derives_every_tier_below_the_original_plus_the_original() {
		let derived = derive(&photo(1500, 1000), false).expect("derive");
		let mut widths: Vec<u32> = derived.variants.iter().map(|v| v.width).collect();
		widths.sort_unstable();
		widths.dedup();
		assert_eq!(widths, vec![640, 1280, 1500]);
	}

	#[test]
	fn stores_one_format_per_tier() {
		// Only AVIF is kept. A browser without it gets a conversion at the edge, which costs
		// one transformation rather than a second permanent copy of every image.
		let derived = derive(&photo(1500, 1000), false).expect("derive");
		for width in [640, 1280, 1500] {
			let at_tier: Vec<encode::Format> =
				derived.variants.iter().filter(|v| v.width == width).map(|v| v.format).collect();
			assert_eq!(at_tier, vec![encode::Format::Avif], "wrong formats at {width}");
		}
	}

	#[test]
	fn keeps_the_aspect_ratio_of_a_portrait_original() {
		let derived = derive(&photo(600, 1200), false).expect("derive");
		for variant in &derived.variants {
			assert!(variant.height > variant.width, "orientation flipped");
		}
	}

	#[test]
	fn re_encodes_a_small_image_without_resizing_it() {
		let derived = derive(&photo(200, 150), false).expect("derive");
		for variant in &derived.variants {
			assert_eq!((variant.width, variant.height), (200, 150));
		}
	}

	#[test]
	fn every_variant_is_addressed_by_its_own_bytes() {
		let derived = derive(&photo(700, 500), false).expect("derive");
		for variant in &derived.variants {
			assert_eq!(variant.cid, cid(&variant.bytes));
		}
		// The asset id is the original's hash, never a variant's.
		assert!(derived.variants.iter().all(|v| v.cid != derived.cid));
	}

	#[test]
	fn produces_a_placeholder_small_enough_to_inline() {
		let derived = derive(&photo(1500, 1000), false).expect("derive");
		// Thumbhash length is not fixed: the payload carries more or fewer coefficients
		// depending on the aspect ratio and whether alpha is present. What matters is that it
		// stays small enough to sit in a manifest without thought, so the bound is asserted
		// rather than a value that happened to come out of one image.
		assert!((16..=32).contains(&derived.thumb.len()), "thumbhash is {} bytes", derived.thumb.len());
		// The decoded form the site inlines is no longer produced here, so there is nothing
		// else to assert: the hash is the whole output.
	}

	#[test]
	fn deriving_prepares_the_record_without_writing_public_data() {
		let temporary = temp();
		let root = temporary.path();
		let public = root.join("data/public");
		let original = photo(20, 12);
		let prepared = derive_for(&original, "image/png", None, false, None).expect("derive for write");

		assert!(!public.exists());
		assert_eq!(prepared.derived.cid, cid(&original));
		assert_eq!(prepared.media.blake3, prepared.derived.cid);

		write_derived(&public, &prepared).expect("write derivation");
		for variant in &prepared.derived.variants {
			assert!(store::variant_path(&public, &variant.cid, variant.format.extension()).is_file());
		}
		let document: manifest::Document = serde_json::from_str(
			&std::fs::read_to_string(store::meta_path(&public, &prepared.derived.cid)).expect("record"),
		)
		.expect("document");
		assert_eq!(document.media, prepared.media);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn a_single_image_returns_only_after_its_bytes_and_records_exist() {
		let temporary = temp();
		let root = temporary.path();
		let source = root.join("source.png");
		let original = photo(20, 12);
		std::fs::write(&source, &original).expect("source");

		let id = store_one(&root, &source, false).expect("store one");

		assert_eq!(id, cid(&original));
		let merged = run::load(&root.join(run::MERGED)).expect("merged");
		let media = merged.media.get(&id).expect("merged record");
		assert!(store::meta_path(&root.join("data/public"), &id).is_file());
		for (variant, record) in &media.variants {
			assert!(
				store::variant_path(
					&root.join("data/public"),
					variant,
					record.mime.strip_prefix("image/").expect("image mime"),
				)
				.is_file()
			);
		}
		std::fs::remove_dir_all(root).ok();
	}
}
