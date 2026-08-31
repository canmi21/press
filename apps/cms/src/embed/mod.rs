//! Data the articles embed but do not contain: a crate's dependency tree, a repository's state.
//!
//! Fetched once at build time and written under `data/build/`, so a page renders from a
//! checkout with no proxy route, no request per reader, and no key. Both records are
//! rebuildable from what git already holds -- an article names the crate and the repo, and both
//! services answer for free -- which is what puts them there rather than beside the curated
//! records. See spec/architecture/data.md.

pub mod crates;
pub mod fetch;
pub mod repos;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

/// What the articles ask to embed, found by scanning them.
#[derive(Debug, Default, PartialEq)]
pub struct Wanted {
	pub crates: Vec<String>,
	pub repos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crates {
	pub version: u32,
	#[serde(default)]
	pub crates: BTreeMap<String, crates::Crate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repos {
	pub version: u32,
	#[serde(default)]
	pub repos: BTreeMap<String, repos::Repo>,
}

impl Default for Crates {
	fn default() -> Self {
		Self { version: VERSION, crates: BTreeMap::new() }
	}
}

impl Default for Repos {
	fn default() -> Self {
		Self { version: VERSION, repos: BTreeMap::new() }
	}
}

pub fn crates_path(repo: &Path) -> PathBuf {
	repo.join("data").join("build").join("crates.json")
}

pub fn repos_path(repo: &Path) -> PathBuf {
	repo.join("data").join("build").join("repos.json")
}

/// Every crate and repository the articles name.
///
/// Read off the placeholder directives rather than kept in a list somebody maintains: an
/// article that stops mentioning a crate stops asking for it, with nothing to remember.
pub fn wanted(article: &str) -> Wanted {
	let mut found = Wanted::default();
	for line in article.lines() {
		let trimmed = line.trim_start();
		if trimmed.starts_with("::cargo{") {
			found.crates.extend(attribute(trimmed, "crate"));
		} else if trimmed.starts_with("::github{") {
			found.repos.extend(attribute(trimmed, "repo"));
		}
	}
	found.crates.sort();
	found.crates.dedup();
	found.repos.sort();
	found.repos.dedup();
	found
}

fn attribute(line: &str, name: &str) -> Option<String> {
	let needle = format!("{name}=\"");
	let at = line.find(&needle)? + needle.len();
	let rest = &line[at..];
	let end = rest.find('"')?;
	Some(rest[..end].to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_articles_say_what_to_fetch() {
		// Kept nowhere else. An article that stops naming a crate stops asking for it, which is
		// one fewer list to keep in step with the prose.
		let article = "\n::cargo{crate=\"seam-cli\"}\n\n\
		               ::github{repo=\"canmi21/seam\" ref=\"a7f34cb\"}\n\
		               ::cargo{crate=\"seam-cli\"}\n\
		               ::image{src=\"a.avif\"}\n";
		let found = wanted(article);
		assert_eq!(found.crates, vec!["seam-cli"]);
		assert_eq!(found.repos, vec!["canmi21/seam"]);
	}

	#[test]
	fn a_placeholder_of_another_kind_asks_for_nothing() {
		assert_eq!(wanted("::placeholder{kind=\"tokei\"}"), Wanted::default());
		assert_eq!(wanted("::image{src=\"a.avif\"}"), Wanted::default());
		assert_eq!(wanted("just prose"), Wanted::default());
	}
}
