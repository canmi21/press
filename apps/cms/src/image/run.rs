//! The `cms image` command: originals in, published variants and manifests out.

use super::manifest::{self, Media, Merged};
use super::{derive, store};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the merged manifest is committed. Articles reference images by content id, and this
/// is what lets a build resolve one without the images being present at all.
pub const MERGED: &str = "assets.json";

pub struct Outcome {
	pub processed: usize,
	pub skipped: usize,
	pub failed: Vec<(PathBuf, String)>,
	/// Original filename stem to content id, for rewriting references that still name a file.
	pub renamed: BTreeMap<String, String>,
}

/// Process every original under `originals`, publishing into `public` and merging into
/// `repo/assets.json`.
///
/// An asset already present in the merged manifest is skipped unless `force`: deriving is
/// minutes of CPU, and the content id proves nothing changed.
pub fn run(repo: &Path, originals: &Path, public: &Path, force: bool) -> std::io::Result<Outcome> {
	let merged_path = repo.join(MERGED);
	let mut merged = load(&merged_path);
	let mut outcome = Outcome {
		processed: 0,
		skipped: 0,
		failed: Vec::new(),
		renamed: BTreeMap::new(),
	};

	for path in sources(originals)? {
		let bytes = match std::fs::read(&path) {
			Ok(bytes) => bytes,
			Err(error) => {
				outcome.failed.push((path, error.to_string()));
				continue;
			}
		};

		let id = super::cid(&bytes);
		if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
			outcome.renamed.insert(stem.to_owned(), id.clone());
		}

		if !force && merged.assets.contains_key(&id) {
			outcome.skipped += 1;
			continue;
		}

		match publish(&bytes, &path, public, merged.assets.get(&id)) {
			Ok(media) => {
				merged.assets.insert(id, media);
				outcome.processed += 1;
			}
			Err(error) => outcome.failed.push((path, error)),
		}
	}

	merged.generated = manifest::now();
	store::write(
		&merged_path,
		format!(
			"{}\n",
			serde_json::to_string_pretty(&merged).unwrap_or_default()
		)
		.as_bytes(),
	)?;
	Ok(outcome)
}

fn publish(
	bytes: &[u8],
	path: &Path,
	public: &Path,
	previous: Option<&Media>,
) -> Result<Media, String> {
	let derived = derive(bytes).map_err(|error| error.to_string())?;
	let mime = mime_of(path);
	let media = manifest::media_for(
		&derived,
		mime,
		bytes.len() as u64,
		previous.map(|media| media.created.as_str()),
	);

	for variant in &derived.variants {
		let target = store::variant_path(public, &variant.cid, variant.format.extension());
		store::write(&target, &variant.bytes).map_err(|error| error.to_string())?;
	}

	let document = manifest::Document {
		version: manifest::VERSION,
		media: media.clone(),
	};
	let json = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
	store::write(&store::meta_path(public, &derived.cid), json.as_bytes())
		.map_err(|error| error.to_string())?;

	Ok(media)
}

fn load(path: &Path) -> Merged {
	std::fs::read_to_string(path)
		.ok()
		.and_then(|text| serde_json::from_str(&text).ok())
		.unwrap_or_else(|| Merged {
			version: manifest::VERSION,
			generated: manifest::now(),
			assets: BTreeMap::new(),
		})
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
	path
		.file_name()
		.and_then(|name| name.to_str())
		.is_some_and(|name| name.starts_with('.'))
}

fn mime_of(path: &Path) -> &'static str {
	match path
		.extension()
		.and_then(|e| e.to_str())
		.unwrap_or_default()
		.to_ascii_lowercase()
		.as_str()
	{
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"webp" => "image/webp",
		"avif" => "image/avif",
		"gif" => "image/gif",
		_ => "application/octet-stream",
	}
}

/// Replace old filename references in articles with the content ids they now resolve to.
///
/// Returns how many references changed. Matching on the stem rather than the whole filename
/// so a reference keeps working whether or not it carried an extension.
pub fn rewrite_references(
	articles: &Path,
	renamed: &BTreeMap<String, String>,
) -> std::io::Result<usize> {
	let mut changed = 0;
	for path in markdown_under(articles)? {
		let original = std::fs::read_to_string(&path)?;
		let mut text = original.clone();
		for (old, new) in renamed {
			if old == new || !text.contains(old.as_str()) {
				continue;
			}
			changed += text.matches(old.as_str()).count();
			text = text.replace(old.as_str(), new);
		}
		if text != original {
			std::fs::write(&path, text)?;
		}
	}
	Ok(changed)
}

fn markdown_under(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
	let mut found = Vec::new();
	if !directory.is_dir() {
		return Ok(found);
	}
	for entry in std::fs::read_dir(directory)?.filter_map(Result::ok) {
		let path = entry.path();
		if path.is_dir() {
			found.extend(markdown_under(&path)?);
		} else if path.extension().and_then(|e| e.to_str()) == Some("md") {
			found.push(path);
		}
	}
	found.sort();
	Ok(found)
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
		assert!(
			sources(Path::new("/nowhere-at-all"))
				.expect("sources")
				.is_empty()
		);
	}

	#[test]
	fn rewrites_references_across_nested_articles() {
		let root = temp("rewrite");
		std::fs::create_dir_all(root.join("deep")).expect("dir");
		std::fs::write(root.join("a.md"), "![](oldhash.png) and ![](oldhash.png)").expect("write");
		std::fs::write(root.join("deep/b.md"), "<img src=\"oldhash.png\">").expect("write");

		let mut renamed = BTreeMap::new();
		renamed.insert("oldhash".to_owned(), "newcid".to_owned());
		let changed = rewrite_references(&root, &renamed).expect("rewrite");

		assert_eq!(changed, 3);
		assert!(
			std::fs::read_to_string(root.join("a.md"))
				.unwrap()
				.contains("newcid.png")
		);
		assert!(
			std::fs::read_to_string(root.join("deep/b.md"))
				.unwrap()
				.contains("newcid.png")
		);
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn leaves_an_article_alone_when_nothing_matches() {
		let root = temp("nomatch");
		std::fs::write(root.join("a.md"), "no images here").expect("write");
		let mut renamed = BTreeMap::new();
		renamed.insert("oldhash".to_owned(), "newcid".to_owned());
		assert_eq!(rewrite_references(&root, &renamed).expect("rewrite"), 0);
		std::fs::remove_dir_all(&root).ok();
	}
}
