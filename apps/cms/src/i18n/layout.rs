//! The ordered segment layout consumed by the site build.
//!
//! Rust alone decides block boundaries and ids. The committed artifact carries only the byte
//! ranges of translatable blocks, so the TypeScript build can assemble translations without
//! learning how segmentation works or requiring Rust in CI.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FILE: &str = "data/article-segments.json";
pub const VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
	pub id: String,
	pub start: usize,
	pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
	pub version: u8,
	pub articles: BTreeMap<String, Vec<Span>>,
}

pub fn path_for(root: &Path) -> PathBuf {
	root.join(FILE)
}

pub fn build(root: &Path) -> std::io::Result<Layout> {
	let contents = root.join("contents");
	let mut articles = BTreeMap::new();
	for path in crate::refs::markdown_under(&contents)? {
		let article = std::fs::read_to_string(&path)?;
		let relative = path
			.strip_prefix(&contents)
			.map_err(|error| std::io::Error::other(error.to_string()))?
			.to_string_lossy()
			.replace('\\', "/")
			.trim_start_matches('/')
			.to_owned();
		let spans = super::segment::split(&article)
			.into_iter()
			.filter(|segment| segment.kind.translatable())
			.map(|segment| Span {
				id: segment.id,
				start: segment.start,
				end: segment.end,
			})
			.collect();
		articles.insert(relative, spans);
	}
	Ok(Layout {
		version: VERSION,
		articles,
	})
}

#[cfg(test)]
pub fn load(path: &Path) -> std::io::Result<Layout> {
	let text = std::fs::read_to_string(path)?;
	serde_json::from_str(&text).map_err(|error| std::io::Error::other(error.to_string()))
}

/// Rewrite only when the derived record changed, returning whether anything was written.
pub fn sync(root: &Path) -> std::io::Result<bool> {
	let path = path_for(root);
	let mut text = serde_json::to_string_pretty(&build(root)?)
		.map_err(|error| std::io::Error::other(error.to_string()))?;
	text.push('\n');
	if std::fs::read_to_string(&path).is_ok_and(|existing| existing == text) {
		return Ok(false);
	}
	std::fs::write(path, text)?;
	Ok(true)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_committed_layout_matches_the_rust_splitter() {
		let root = crate::paths::repo_root().expect("repository root");
		let committed = load(&path_for(&root)).expect("run `cms segments` to create the layout");
		let current = build(&root).expect("derive current layout");
		assert_eq!(
			committed, current,
			"article segmentation changed; run `cms segments` and commit the result"
		);
	}
}
