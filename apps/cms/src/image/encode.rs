//! Turning decoded pixels into the formats that get served.
//!
//! Quality settings were measured rather than guessed, on real articles' images: at 1272px a
//! screenshot is 46 KB as AVIF q68 against 508 KB as optimised PNG, and a photo at 1624px is
//! 364 KB against 3094 KB. Lossless formats lost by five to eleven times on every sample, so
//! there is no lossless tier. See spec/architecture.md.

use image::DynamicImage;

/// Measured to hold text edges without ringing, which is the failure mode that shows on the
/// screenshots this site is mostly made of. Photos would tolerate less; the difference in
/// bytes between q52 and q68 was small enough that one setting for both is worth the
/// simplicity of having no content classifier to be wrong.
const AVIF_QUALITY: f32 = 68.0;
/// Only ever reached by a browser without AVIF, so it is tuned for safety over size.
const WEBP_QUALITY: f32 = 80.0;
/// rav1e trades encode time for size. 6 is the middle; this runs once per image locally.
const AVIF_SPEED: u8 = 6;

/// How few distinct colours an image may have before lossy coding is the wrong tool.
///
/// Pixel art, sprites and flat diagrams have hard edges and a tiny palette: PNG stores them
/// exactly and small, while a lossy codec spends bytes inventing gradients across edges that
/// were meant to be sharp. Note the screenshots measured above are *not* this -- Retina
/// capture and antialiasing make them continuous-tone, which is why they compress like
/// photographs.
const FLAT_COLOUR_LIMIT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
	Avif,
	Webp,
	Png,
}

impl Format {
	pub fn extension(self) -> &'static str {
		match self {
			Self::Avif => "avif",
			Self::Webp => "webp",
			Self::Png => "png",
		}
	}

	/// The quality this format is encoded at, normalised to 0..1. PNG is exact, so it has no
	/// quality to report and answers 1.
	pub fn quality(self) -> f32 {
		match self {
			Self::Avif => AVIF_QUALITY / 100.0,
			Self::Webp => WEBP_QUALITY / 100.0,
			Self::Png => 1.0,
		}
	}

	pub fn mime(self) -> &'static str {
		match self {
			Self::Avif => "image/avif",
			Self::Webp => "image/webp",
			Self::Png => "image/png",
		}
	}
}

#[derive(Debug)]
pub enum Error {
	Encode(Format),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Encode(format) => write!(f, "could not encode {}", format.extension()),
		}
	}
}

/// Which formats to emit for an image, in the order a browser should prefer them.
///
/// Flat-colour images get PNG alone: it is both smaller and exact for them, and offering a
/// lossy alternative would only invite a browser to choose the worse one.
pub fn formats_for(image: &DynamicImage) -> Vec<Format> {
	if is_flat_colour(image) {
		vec![Format::Png]
	} else {
		vec![Format::Avif, Format::Webp]
	}
}

pub fn encode(image: &DynamicImage, format: Format) -> Result<Vec<u8>, Error> {
	match format {
		Format::Avif => avif(image),
		Format::Webp => webp(image),
		Format::Png => png(image),
	}
}

fn avif(image: &DynamicImage) -> Result<Vec<u8>, Error> {
	let rgba = image.to_rgba8();
	let (width, height) = (rgba.width() as usize, rgba.height() as usize);
	let pixels: Vec<rgb::RGBA8> = rgba
		.pixels()
		.map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3]))
		.collect();
	ravif::Encoder::new()
		.with_quality(AVIF_QUALITY)
		.with_speed(AVIF_SPEED)
		.encode_rgba(ravif::Img::new(&pixels[..], width, height))
		.map(|encoded| encoded.avif_file)
		.map_err(|_| Error::Encode(Format::Avif))
}

fn webp(image: &DynamicImage) -> Result<Vec<u8>, Error> {
	webp::Encoder::from_image(image)
		.map(|encoder| encoder.encode(WEBP_QUALITY).to_vec())
		.map_err(|_| Error::Encode(Format::Webp))
}

fn png(image: &DynamicImage) -> Result<Vec<u8>, Error> {
	let mut out = Vec::new();
	image
		.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
		.map_err(|_| Error::Encode(Format::Png))?;
	Ok(out)
}

/// Whether an image has few enough distinct colours to be palette art rather than a
/// photograph. Counting stops early, so a photograph costs a few thousand pixels to reject
/// rather than a full scan.
fn is_flat_colour(image: &DynamicImage) -> bool {
	use std::collections::HashSet;
	let rgba = image.to_rgba8();
	let mut seen: HashSet<[u8; 4]> = HashSet::new();
	for pixel in rgba.pixels() {
		seen.insert(pixel.0);
		if seen.len() > FLAT_COLOUR_LIMIT {
			return false;
		}
	}
	true
}

#[cfg(test)]
mod tests {
	use super::*;
	use image::{Rgba, RgbaImage};

	fn solid(width: u32, height: u32, colours: u8) -> DynamicImage {
		let mut buffer = RgbaImage::new(width, height);
		for (x, _y, pixel) in buffer.enumerate_pixels_mut() {
			let shade = ((x % u32::from(colours.max(1))) * 8) as u8;
			*pixel = Rgba([shade, shade, shade, 255]);
		}
		DynamicImage::ImageRgba8(buffer)
	}

	fn noisy(width: u32, height: u32) -> DynamicImage {
		let mut buffer = RgbaImage::new(width, height);
		let mut state: u32 = 0x1234_5678;
		for pixel in buffer.pixels_mut() {
			state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
			let [r, g, b, _] = state.to_le_bytes();
			*pixel = Rgba([r, g, b, 255]);
		}
		DynamicImage::ImageRgba8(buffer)
	}

	#[test]
	fn sends_palette_art_to_png_alone() {
		assert_eq!(formats_for(&solid(64, 64, 8)), vec![Format::Png]);
	}

	#[test]
	fn sends_continuous_tone_to_the_lossy_pair() {
		assert_eq!(
			formats_for(&noisy(64, 64)),
			vec![Format::Avif, Format::Webp]
		);
	}

	#[test]
	fn counting_colours_stops_early_on_a_photograph() {
		// A large noisy image would be expensive to scan fully; the early exit is what makes
		// this check affordable to run on every image.
		assert!(!is_flat_colour(&noisy(2000, 2000)));
	}

	#[test]
	fn encodes_each_format_to_non_empty_bytes() {
		let image = noisy(64, 64);
		for format in [Format::Avif, Format::Webp, Format::Png] {
			let bytes = encode(&image, format).expect("encode");
			assert!(!bytes.is_empty(), "{format:?} produced nothing");
		}
	}

	#[test]
	fn png_output_really_is_png() {
		let bytes = encode(&solid(16, 16, 4), Format::Png).expect("encode");
		assert_eq!(&bytes[1..4], b"PNG");
	}
}
