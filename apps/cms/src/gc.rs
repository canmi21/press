//! The `cms gc` command: dropping what no article asks for any more.
//!
//! Kept apart from every other command and never run as a side effect. Deriving an image and
//! deleting one are opposite risks: the first can be repeated until it is right, the second
//! is only safe because `data/image` still holds the originals and `cms image` can rebuild
//! from a content id alone. That safety is a property of this repository, not of the
//! algorithm, so the deletion waits to be asked for. See spec/architecture.md.
//!
//! Dry by default, like `mise run sync`, and for the same reason: the output is the review.

use crate::image::manifest::Merged;
use crate::image::run::MERGED;
use crate::refs;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Sweep {
	/// Files no reachable asset claims.
	pub orphans: Vec<PathBuf>,
	/// Manifest entries for assets no article references any more.
	pub entries: Vec<String>,
	pub bytes: u64,
}

/// Everything in `data/public` that nothing reachable from an article accounts for.
pub fn plan(repo: &Path, public: &Path, articles: &Path) -> std::io::Result<Sweep> {
	let scan = refs::scan(articles)?;
	let merged = load(&repo.join(MERGED));
	let wanted = scan.cids();

	// An article names the original; the objects on disk are its variants and its record. The
	// manifest is the only thing that connects the two, so a cid missing from it keeps nothing
	// alive -- which is correct, because the site could not resolve it either.
	let mut keep: BTreeSet<String> = wanted.clone();
	for cid in &wanted {
		if let Some(media) = merged.assets.get(cid) {
			keep.extend(media.variants.keys().cloned());
		}
	}

	let mut sweep = Sweep {
		entries: merged
			.assets
			.keys()
			.filter(|cid| !wanted.contains(*cid))
			.cloned()
			.collect(),
		..Sweep::default()
	};

	for path in files_under(&public.join("image"))?
		.into_iter()
		.chain(files_under(&public.join("meta"))?)
	{
		if !keep.contains(&stem_of(&path)) {
			sweep.bytes += path.metadata().map(|meta| meta.len()).unwrap_or_default();
			sweep.orphans.push(path);
		}
	}

	// Icons are swept by domain rather than by content: the directory existing is the record
	// that the domain was checked, so removing one file inside it would claim the site was
	// asked and had no icon.
	let wanted_domains: BTreeSet<String> =
		scan.wanted().into_iter().map(|icon| icon.domain).collect();
	for directory in directories_under(&public.join("favicon"))? {
		let name = directory
			.file_name()
			.and_then(|n| n.to_str())
			.unwrap_or_default()
			.to_owned();
		if !wanted_domains.contains(&name) {
			sweep.bytes += files_under(&directory)?
				.iter()
				.filter_map(|path| path.metadata().ok())
				.map(|meta| meta.len())
				.sum::<u64>();
			sweep.orphans.push(directory);
		}
	}

	sweep.orphans.sort();
	Ok(sweep)
}

/// Carry out a plan, and rewrite the manifest without the entries it dropped.
pub fn apply(repo: &Path, sweep: &Sweep) -> std::io::Result<()> {
	for path in &sweep.orphans {
		if path.is_dir() {
			std::fs::remove_dir_all(path)?;
		} else {
			std::fs::remove_file(path)?;
		}
	}

	if sweep.entries.is_empty() {
		return Ok(());
	}
	let merged_path = repo.join(MERGED);
	let mut merged = load(&merged_path);
	for cid in &sweep.entries {
		merged.assets.remove(cid);
	}
	merged.generated = crate::image::manifest::now();
	crate::image::store::write(
		&merged_path,
		format!(
			"{}\n",
			serde_json::to_string_pretty(&merged).unwrap_or_default()
		)
		.as_bytes(),
	)
}

fn load(path: &Path) -> Merged {
	std::fs::read_to_string(path)
		.ok()
		.and_then(|text| serde_json::from_str(&text).ok())
		.unwrap_or_else(|| Merged {
			version: crate::image::manifest::VERSION,
			generated: crate::image::manifest::now(),
			assets: std::collections::BTreeMap::new(),
		})
}

/// The content id a stored file is named by, whatever it is nested under.
fn stem_of(path: &Path) -> String {
	path
		.file_stem()
		.and_then(|stem| stem.to_str())
		.unwrap_or_default()
		.to_owned()
}

fn files_under(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
	let mut found = Vec::new();
	if !directory.is_dir() {
		return Ok(found);
	}
	for entry in std::fs::read_dir(directory)?.filter_map(Result::ok) {
		let path = entry.path();
		if path.is_dir() {
			found.extend(files_under(&path)?);
		} else {
			found.push(path);
		}
	}
	Ok(found)
}

fn directories_under(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
	if !directory.is_dir() {
		return Ok(Vec::new());
	}
	Ok(
		std::fs::read_dir(directory)?
			.filter_map(Result::ok)
			.map(|entry| entry.path())
			.filter(|path| path.is_dir())
			.collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::image::manifest::{Media, Source, VariantRecord};
	use std::collections::BTreeMap;

	fn temp(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("cms-gc-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	fn media(variant: &str) -> Media {
		let mut variants = BTreeMap::new();
		variants.insert(
			variant.to_owned(),
			VariantRecord {
				mime: "image/avif".into(),
				width: 640,
				height: 360,
				quality: 0.68,
				bytes: 1,
			},
		);
		Media {
			kind: "image".into(),
			created: "2026-07-31T00:00:00Z".into(),
			updated: "2026-07-31T00:00:00Z".into(),
			blake3: String::new(),
			thumbhash: String::new(),
			source: Source {
				mime: "image/png".into(),
				width: 640,
				height: 360,
				ratio: "16:9".into(),
				bytes: 1,
			},
			metadata: None,
			variants,
		}
	}

	/// A repository with one referenced asset and one abandoned one.
	fn scenario(name: &str) -> (PathBuf, String, String) {
		let root = temp(name);
		let kept = "44b6081deaf0242ca3bf83d62a3b6c95".to_owned();
		let dropped = "12faaa76365814de1195d6bdf1e5ba05".to_owned();
		let kept_variant = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
		let dropped_variant = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();

		std::fs::create_dir_all(root.join("contents")).expect("dir");
		std::fs::write(root.join("contents/a.md"), format!("![]({kept}.avif)")).expect("write");

		let mut assets = BTreeMap::new();
		assets.insert(kept.clone(), media(&kept_variant));
		assets.insert(dropped.clone(), media(&dropped_variant));
		let merged = Merged {
			version: 1,
			generated: "2026-07-31T00:00:00Z".into(),
			assets,
		};
		// Through the same writer production uses, which creates the parent. The manifest sits
		// at `data/metadata.json` now, so a bare write lands in a directory that is not there.
		crate::image::store::write(
			&root.join(MERGED),
			serde_json::to_string(&merged).expect("json").as_bytes(),
		)
		.expect("write");

		let public = root.join("public");
		for (cid, variant) in [(&kept, &kept_variant), (&dropped, &dropped_variant)] {
			let object = crate::image::store::variant_path(&public, variant, "avif");
			crate::image::store::write(&object, b"bytes").expect("write");
			crate::image::store::write(&crate::image::store::meta_path(&public, cid), b"{}")
				.expect("write");
		}
		(root, kept_variant, dropped_variant)
	}

	#[test]
	fn keeps_the_variants_of_a_referenced_asset() {
		let (root, kept_variant, dropped_variant) = scenario("keep");
		let sweep = plan(&root, &root.join("public"), &root.join("contents")).expect("plan");

		let names: Vec<String> = sweep.orphans.iter().map(|p| stem_of(p)).collect();
		assert!(!names.contains(&kept_variant), "swept a live variant");
		assert!(names.contains(&dropped_variant), "kept a dead variant");
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn drops_the_manifest_entry_along_with_the_bytes() {
		// Leaving the record behind would make the manifest grow forever and would let a
		// later reference resolve to variants that are no longer there.
		let (root, _, _) = scenario("entries");
		let sweep = plan(&root, &root.join("public"), &root.join("contents")).expect("plan");
		assert_eq!(sweep.entries, vec!["12faaa76365814de1195d6bdf1e5ba05"]);

		apply(&root, &sweep).expect("apply");
		let merged = load(&root.join(MERGED));
		assert_eq!(merged.assets.len(), 1);
		assert!(
			merged
				.assets
				.contains_key("44b6081deaf0242ca3bf83d62a3b6c95")
		);
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn planning_alone_deletes_nothing() {
		let (root, _, _) = scenario("dry");
		let sweep = plan(&root, &root.join("public"), &root.join("contents")).expect("plan");
		assert!(!sweep.orphans.is_empty());
		for path in &sweep.orphans {
			assert!(path.exists(), "planning removed {}", path.display());
		}
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn sweeps_an_icon_directory_no_article_links_to() {
		let root = temp("icons");
		std::fs::create_dir_all(root.join("contents")).expect("dir");
		std::fs::write(
			root.join("contents/a.md"),
			r#"::linkcard{url="https://kept.com"}"#,
		)
		.expect("write");

		let public = root.join("public");
		for domain in ["kept.com", "gone.com"] {
			let directory = public.join("favicon").join(domain);
			std::fs::create_dir_all(&directory).expect("dir");
			std::fs::write(directory.join("light.png"), b"icon").expect("write");
		}

		let sweep = plan(&root, &public, &root.join("contents")).expect("plan");
		assert_eq!(sweep.orphans.len(), 1);
		assert!(sweep.orphans[0].ends_with("gone.com"));
		std::fs::remove_dir_all(&root).ok();
	}
}
