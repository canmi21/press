//! The `cms image` command: what the articles ask for, derived and published.
//!
//! Articles drive this, not the contents of a directory. A reference is either finished --
//! `{cid}.{ext}`, a content id and the format it resolved to -- or it still names a file, in
//! which case that file is looked for under `data/image`, derived, published, and the
//! reference rewritten to what it became. Rewriting is what records that the work is done, so
//! the state lives in the article rather than in a log beside it.

use super::manifest::{self, Media, Merged};
use super::{mime_of, store};
use crate::refs::{self, Scan};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the merged manifest is committed, relative to the repository root.
///
/// Inside `data/` because it describes what is there, and tracked anyway because a build
/// resolves every image from it without a byte of `data/` being present. It is the one file
/// under that directory git keeps -- see .gitignore, which says why.
pub const MERGED: &str = "data/metadata.json";

#[derive(Debug, Default)]
pub struct Outcome {
	pub processed: usize,
	pub skipped: usize,
	pub rewritten: usize,
	/// Records rewritten because the manifest moved to a newer shape.
	pub migrated: usize,
	pub failed: Vec<(PathBuf, String)>,
	/// References naming a file that is not under `data/image`.
	///
	/// Not an error: an article may be written before its picture is dropped in, and stopping
	/// the run would leave every other image unprocessed for the sake of one that is late.
	pub missing: Vec<String>,
}

pub struct Options<'a> {
	pub force: bool,
	pub keep_original: bool,
	/// Files named on the command line. Empty means "whatever the articles ask for".
	pub only: &'a [PathBuf],
}

/// Derive and publish everything the articles reference, then rewrite the references.
pub fn run(
	repo: &Path,
	originals: &Path,
	public: &Path,
	articles: &Path,
	options: &Options<'_>,
) -> std::io::Result<Outcome> {
	let merged_path = repo.join(MERGED);
	let mut merged = load(&merged_path)?;
	let mut outcome = Outcome::default();
	let scan = refs::scan(articles)?;
	// Opened once for the whole run, and absent when the data has not been fetched -- which
	// reads the same as a photograph carrying no position.
	let gazetteer = super::geo::Gazetteer::open(repo);

	// Records published under an older shape are rewritten from the merged manifest, which
	// already holds everything they contain. Re-deriving to fix a version number would spend
	// minutes of CPU to produce identical pixels.
	for cid in manifest::migrate(&mut merged, public) {
		if let Some(media) = merged.media.get(&cid) {
			republish(public, &cid, media)?;
			outcome.migrated += 1;
		}
	}

	// What the article wrote, mapped to what it should say now.
	let mut rewrites: BTreeMap<String, String> = BTreeMap::new();

	for (reference, path) in wanted(&scan, originals, public, &merged, options, &mut outcome) {
		let bytes = match std::fs::read(&path) {
			Ok(bytes) => bytes,
			Err(error) => {
				outcome.failed.push((path, error.to_string()));
				continue;
			}
		};

		let id = super::cid(&bytes);
		let previous = merged.media.get(&id);
		let keep = options.keep_original || previous.is_some_and(keeps_full_frame);

		if !options.force && previous.is_some() && published(public, previous) {
			outcome.skipped += 1;
			if let Some(target) = reference.as_deref() {
				note(&mut rewrites, target, &id, previous);
			}
			continue;
		}

		match super::publish(&bytes, mime_of(&path), public, previous, keep, gazetteer.as_ref()) {
			Ok(media) => {
				if let Some(target) = reference.as_deref() {
					note(&mut rewrites, target, &id, Some(&media));
				}
				merged.media.insert(id, media);
				outcome.processed += 1;
			}
			Err(error) => outcome.failed.push((path, error.to_string())),
		}
	}

	// A finished reference can still name the wrong format: an article written when the
	// pipeline stored PNG, or an asset re-derived into something else since. The extension is
	// a claim about what the CDN will serve, so it is corrected from the manifest without
	// deriving anything.
	for image in &scan.images {
		let Some((cid, _)) = image.resolved() else {
			continue;
		};
		if let Some(name) = merged.media.get(cid).and_then(|media| resolved_name(cid, media))
			&& name != image.value
		{
			rewrites.insert(image.value.clone(), name);
		}
	}

	merged.updated = manifest::now();
	let json = serde_json::to_string_pretty(&merged)
		.map_err(|error| std::io::Error::other(error.to_string()))?;
	store::write(&merged_path, format!("{json}\n").as_bytes())?;

	// Bytes and manifest land first because a crash after publishing can only leave a derived
	// image no article references yet; that is harmless and a rerun repairs it. Rewriting first
	// could crash before the bytes land, leaving an article pointing at something nonexistent,
	// and the lost original filename would leave a later run no way to repair it. See spec/tasks.md.
	outcome.rewritten = rewrite_references(articles, &rewrites)?;
	Ok(outcome)
}

/// Every original to look at this run, paired with the reference that asked for it.
///
/// Files named on the command line have no reference to rewrite -- they are being imported
/// ahead of the article that will use them, and `--original` is how that is declared.
fn wanted(
	scan: &Scan,
	originals: &Path,
	public: &Path,
	merged: &Merged,
	options: &Options<'_>,
	outcome: &mut Outcome,
) -> Vec<(Option<String>, PathBuf)> {
	if !options.only.is_empty() {
		return options.only.iter().map(|path| (None, path.clone())).collect();
	}

	let mut found: Vec<(Option<String>, PathBuf)> = Vec::new();

	for image in scan.unresolved() {
		let candidate = originals.join(&image.value);
		if candidate.is_file() {
			found.push((Some(image.value.clone()), candidate));
		} else {
			outcome.missing.push(image.value.clone());
		}
	}

	// A finished reference whose variants are gone -- swept, or never published on this
	// machine. The original is found by hashing, because the id is the hash.
	let unpublished: Vec<String> =
		scan.cids().into_iter().filter(|cid| !published(public, merged.media.get(cid))).collect();
	if !unpublished.is_empty() {
		let by_id = originals_by_id(originals);
		for cid in unpublished {
			match by_id.get(&cid) {
				Some(path) => found.push((None, path.clone())),
				None => outcome.missing.push(cid),
			}
		}
	}

	found
}

/// Content id of every original on hand, so a swept asset can be rebuilt from its id alone.
fn originals_by_id(originals: &Path) -> BTreeMap<String, PathBuf> {
	sources(originals)
		.unwrap_or_default()
		.into_iter()
		.filter_map(|path| {
			let bytes = std::fs::read(&path).ok()?;
			Some((super::cid(&bytes), path))
		})
		.collect()
}

/// Whether a published record was derived with the full frame kept.
///
/// The published variants already answer this: a rung at exactly the source's long edge can only
/// exist because the full frame was kept. Below the cap the top rung is the source either way,
/// so there is nothing to infer and nothing that could be inferred wrong.
///
/// Measured on the long edge, never on width, for the reason `ladder` gives: a portrait
/// photograph is not a small image, it is a tall one. Asking whether the width cleared the cap
/// answered no for every tall image, and re-deriving then dropped a full-frame rung that was
/// already published.
///
/// FIXME: this infers a choice spec/architecture/data.md says is recorded on the record. Recording it
/// is deliberately not done yet -- the input directory, and the retained originals it implies,
/// exist only because the CLI is the one way an image enters. Once the desktop app derives on
/// insert there is no original to re-derive from and nothing left to infer, and the question
/// becomes a metadata version migration instead. Revisit when the app owns image insertion.
fn keeps_full_frame(media: &Media) -> bool {
	let source = super::ladder::Size::new(media.source.width, media.source.height);
	media.variants.values().any(|record| {
		super::ladder::Size::new(record.width, record.height).long_edge() == source.long_edge()
	}) && source.long_edge() > super::ladder::CAP
}

/// Whether every variant a record claims is actually on disk.
///
/// The manifest alone is not evidence: after a sweep it still lists assets whose bytes are
/// gone, and trusting it would leave articles pointing at nothing.
fn published(public: &Path, media: Option<&Media>) -> bool {
	let Some(media) = media else {
		return false;
	};
	media.variants.iter().all(|(cid, record)| {
		store::variant_path(public, cid, crate::extension::for_variant(&record.mime)).is_file()
	})
}

/// What an article should call this asset: its content id and the format it resolved to.
///
/// The largest variant decides the extension. It is the one an article without a srcset falls
/// back to, and every rung of a ladder shares its format.
fn resolved_name(cid: &str, media: &Media) -> Option<String> {
	let extension = media
		.variants
		.values()
		.max_by_key(|record| record.width)
		.map(|record| crate::extension::for_variant(&record.mime))?;
	Some(format!("{cid}.{extension}"))
}

fn note(
	rewrites: &mut BTreeMap<String, String>,
	reference: &str,
	cid: &str,
	media: Option<&Media>,
) {
	if let Some(name) = media.and_then(|media| resolved_name(cid, media)) {
		rewrites.insert(reference.to_owned(), name);
	}
}

/// Write one asset's record again from what the merged manifest already says.
///
/// Used by the migration and by `cms alt`, both of which change a record without touching a
/// single pixel. Re-deriving to publish a changed field would spend minutes producing bytes
/// that are already correct.
pub fn republish(public: &Path, cid: &str, media: &Media) -> std::io::Result<()> {
	let document = manifest::Document { version: manifest::VERSION, media: media.clone() };
	let json = serde_json::to_string_pretty(&document)
		.map_err(|error| std::io::Error::other(error.to_string()))?;
	store::write(&store::meta_path(public, cid), json.as_bytes())
}

/// The merged manifest, fresh and empty when the repository has none yet.
///
/// `data/metadata.json` is committed, a whole site build resolves its images out of it, and the
/// descriptions in it were paid for one model call at a time. Every writer here loads it,
/// removes or adds a few entries, and saves the whole document back, so reading an unparseable
/// file as an empty one would not degrade -- it would replace the manifest with four fields.
pub fn load(path: &Path) -> std::io::Result<Merged> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
			return Ok(Merged {
				version: manifest::VERSION,
				created: manifest::now(),
				updated: manifest::now(),
				media: BTreeMap::new(),
			});
		}
		Err(error) => return Err(error),
	};
	serde_json::from_str(&text)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

fn sources(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
	if !directory.is_dir() {
		return Ok(Vec::new());
	}
	let mut found: Vec<PathBuf> = std::fs::read_dir(directory)?
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_file() && !is_hidden(path))
		.collect();
	// Sorted so a run over the same directory reports in the same order twice, which is what
	// makes a failure reproducible.
	found.sort();
	Ok(found)
}

fn is_hidden(path: &Path) -> bool {
	path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with('.'))
}

/// Point every rewritten reference at what it became.
///
/// Returns how many references changed. Whole-value matching, not substring: a filename like
/// `a.png` occurs inside plenty of prose, and replacing it there would corrupt the text.
pub fn rewrite_references(
	articles: &Path,
	rewrites: &BTreeMap<String, String>,
) -> std::io::Result<usize> {
	if rewrites.is_empty() {
		return Ok(0);
	}
	let mut changed = 0;
	for path in refs::markdown_under(articles)? {
		let original = std::fs::read_to_string(&path)?;
		let mut text = original.clone();
		for (old, new) in rewrites {
			if old == new {
				continue;
			}
			for (from, to) in [
				(format!("]({old})"), format!("]({new})")),
				(format!("src=\"{old}\""), format!("src=\"{new}\"")),
			] {
				changed += text.matches(&from).count();
				text = text.replace(&from, &to);
			}
		}
		if text != original {
			std::fs::write(&path, text)?;
		}
	}
	Ok(changed)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn temp(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("cms-run-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	#[test]
	fn a_broken_manifest_is_an_error_rather_than_an_empty_one() {
		// Every writer loads the whole document, edits a few entries and saves it back. Read as
		// empty, the next save replaces a committed manifest -- and the paid descriptions in it
		// -- with four fields. See spec/architecture/data.md.
		let root = temp("broken-manifest");
		let path = root.join("metadata.json");
		std::fs::write(&path, "{ not json").expect("write");
		let error = load(&path).expect_err("a broken manifest must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
	}

	#[test]
	fn a_missing_manifest_is_a_fresh_repository() {
		let root = temp("missing-manifest");
		let merged = load(&root.join("metadata.json")).expect("missing is not an error");
		assert!(merged.media.is_empty());
	}

	#[test]
	fn reads_mime_from_the_extension() {
		assert_eq!(mime_of(Path::new("a.PNG")), "image/png");
		assert_eq!(mime_of(Path::new("a.jpeg")), "image/jpeg");
		assert_eq!(mime_of(Path::new("a.unknown")), "application/octet-stream");
	}

	#[test]
	fn ignores_hidden_files() {
		let root = temp("hidden");
		std::fs::write(root.join(".DS_Store"), b"x").expect("write");
		std::fs::write(root.join("real.png"), b"x").expect("write");
		let found = sources(&root).expect("sources");
		assert_eq!(found.len(), 1);
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn a_missing_directory_is_empty_rather_than_an_error() {
		assert!(sources(Path::new("/nowhere-at-all")).expect("sources").is_empty());
	}

	#[test]
	fn rewrites_references_across_nested_articles() {
		let root = temp("rewrite");
		std::fs::create_dir_all(root.join("deep")).expect("dir");
		std::fs::write(root.join("a.md"), "![](shot.png) and ![](shot.png)").expect("write");
		std::fs::write(root.join("deep/b.md"), r#"::linkcard{src="shot.png" url="https://a.example"}"#)
			.expect("write");

		let mut rewrites = BTreeMap::new();
		rewrites.insert("shot.png".to_owned(), "newcid.avif".to_owned());
		let changed = rewrite_references(&root, &rewrites).expect("rewrite");

		assert_eq!(changed, 3);
		assert!(std::fs::read_to_string(root.join("a.md")).unwrap().contains("](newcid.avif)"));
		assert!(
			std::fs::read_to_string(root.join("deep/b.md")).unwrap().contains(r#"src="newcid.avif""#)
		);
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn leaves_prose_alone_that_merely_mentions_a_filename() {
		// "shot.png" is a perfectly ordinary thing to write in a sentence. A substring replace
		// would silently edit the text of the article.
		let root = temp("prose");
		std::fs::write(root.join("a.md"), "I saved it as shot.png last week.").expect("write");
		let mut rewrites = BTreeMap::new();
		rewrites.insert("shot.png".to_owned(), "newcid.avif".to_owned());

		assert_eq!(rewrite_references(&root, &rewrites).expect("rewrite"), 0);
		assert!(std::fs::read_to_string(root.join("a.md")).unwrap().contains("shot.png"));
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn leaves_an_article_alone_when_nothing_matches() {
		let root = temp("nomatch");
		std::fs::write(root.join("a.md"), "no images here").expect("write");
		let mut rewrites = BTreeMap::new();
		rewrites.insert("shot.png".to_owned(), "newcid.avif".to_owned());
		assert_eq!(rewrite_references(&root, &rewrites).expect("rewrite"), 0);
		std::fs::remove_dir_all(&root).ok();
	}

	/// A record whose published rungs are `sizes`, derived from a `width` x `height` source.
	fn derived(width: u32, height: u32, sizes: &[(u32, u32)]) -> manifest::Media {
		let mut variants = BTreeMap::new();
		for (index, (w, h)) in sizes.iter().enumerate() {
			variants.insert(
				format!("r{index}"),
				manifest::VariantRecord {
					mime: "image/avif".into(),
					width: *w,
					height: *h,
					quality: 0.68,
					bytes: 1,
				},
			);
		}
		manifest::Media {
			kind: "image".into(),
			created: "2026-07-31T00:00:00Z".into(),
			updated: "2026-07-31T00:00:00Z".into(),
			blake3: String::new(),
			thumbhash: String::new(),
			source: manifest::Source {
				mime: "image/png".into(),
				width,
				height,
				ratio: "x".into(),
				bytes: 1,
			},
			metadata: None,
			variants,
		}
	}

	#[test]
	fn a_tall_image_keeps_the_full_frame_it_published() {
		// 1000x2400 with --original: the ladder caps at 1920 on the long edge, so the full frame
		// is a rung of its own. Re-deriving used to ask whether the *width* cleared the cap --
		// 1000 does not -- and dropped a variant that was already published. See ladder::Size.
		let tall = derived(1000, 2400, &[(267, 640), (533, 1280), (800, 1920), (1000, 2400)]);
		assert!(keeps_full_frame(&tall));
	}

	#[test]
	fn a_wide_image_keeps_the_full_frame_it_published() {
		let wide = derived(2400, 1000, &[(640, 267), (1280, 533), (1920, 800), (2400, 1000)]);
		assert!(keeps_full_frame(&wide));
	}

	#[test]
	fn a_tall_image_derived_without_the_full_frame_does_not_gain_one() {
		let tall = derived(1000, 2400, &[(267, 640), (533, 1280), (800, 1920)]);
		assert!(!keeps_full_frame(&tall));
	}

	#[test]
	fn an_image_under_the_cap_has_no_separate_full_frame_to_keep() {
		// The top rung is the source, so there is nothing --original could have added and
		// nothing to infer from the top rung matching it.
		let small = derived(800, 1200, &[(427, 640), (800, 1200)]);
		assert!(!keeps_full_frame(&small));
	}

	#[test]
	fn the_largest_variant_decides_the_extension() {
		let mut variants = BTreeMap::new();
		variants.insert(
			"small".to_owned(),
			manifest::VariantRecord {
				mime: "image/avif".into(),
				width: 640,
				height: 360,
				quality: 0.68,
				bytes: 1,
			},
		);
		variants.insert(
			"large".to_owned(),
			manifest::VariantRecord {
				mime: "image/png".into(),
				width: 1920,
				height: 1080,
				quality: 1.0,
				bytes: 2,
			},
		);
		let media = manifest::Media {
			kind: "image".into(),
			created: "2026-07-31T00:00:00Z".into(),
			updated: "2026-07-31T00:00:00Z".into(),
			blake3: "44b6081deaf0242ca3bf83d62a3b6c95".into(),
			thumbhash: String::new(),
			source: manifest::Source {
				mime: "image/png".into(),
				width: 1920,
				height: 1080,
				ratio: "16:9".into(),
				bytes: 3,
			},
			metadata: None,
			variants,
		};

		assert_eq!(
			resolved_name("44b6081deaf0242ca3bf83d62a3b6c95", &media).as_deref(),
			Some("44b6081deaf0242ca3bf83d62a3b6c95.png")
		);
	}

	#[test]
	fn a_stale_sidecar_is_republished_when_the_aggregate_is_current() {
		let root = temp("stale-sidecar");
		let public = root.join("public");
		let articles = root.join("contents");
		std::fs::create_dir_all(root.join("data")).expect("data");
		std::fs::create_dir_all(&articles).expect("articles");

		let cid = "44b6081deaf0242ca3bf83d62a3b6c95";
		let media = manifest::Media {
			kind: "image".into(),
			created: "2026-07-31T00:00:00Z".into(),
			updated: "2026-07-31T00:00:00Z".into(),
			blake3: cid.into(),
			thumbhash: "hash".into(),
			source: manifest::Source {
				mime: "image/png".into(),
				width: 1,
				height: 1,
				ratio: "1:1".into(),
				bytes: 1,
			},
			metadata: None,
			variants: BTreeMap::new(),
		};
		let merged = Merged {
			version: manifest::VERSION,
			created: "2026-07-31T00:00:00Z".into(),
			updated: "2026-07-31T00:00:00Z".into(),
			media: BTreeMap::from([(cid.to_owned(), media.clone())]),
		};
		store::write(
			&root.join(MERGED),
			serde_json::to_string_pretty(&merged).expect("merged").as_bytes(),
		)
		.expect("write merged");
		let mut stale = serde_json::to_value(manifest::Document { version: 2, media: media.clone() })
			.expect("stale document");
		stale["media"]["preview"] = "obsolete".into();
		store::write(
			&store::meta_path(&public, cid),
			serde_json::to_string_pretty(&stale).expect("stale json").as_bytes(),
		)
		.expect("write stale sidecar");

		let outcome = run(
			&root,
			&root.join("data/image"),
			&public,
			&articles,
			&Options { force: false, keep_original: false, only: &[] },
		)
		.expect("run");

		// The old aggregate gate returned before seeing this v2 file because the aggregate was
		// already v3. The repair comes entirely from merged metadata: no original or variant
		// directory exists for a pixel pipeline to read or write.
		assert_eq!(outcome.migrated, 1);
		assert_eq!(outcome.processed, 0);
		let rewritten = std::fs::read_to_string(store::meta_path(&public, cid)).expect("sidecar");
		let document: manifest::Document = serde_json::from_str(&rewritten).expect("document");
		assert_eq!(document.version, manifest::VERSION);
		assert_eq!(document.media, media);
		assert!(!rewritten.contains("preview"));
		assert!(!public.join("image").exists());
		std::fs::remove_dir_all(&root).ok();
	}
}
