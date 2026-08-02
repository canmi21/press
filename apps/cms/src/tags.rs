//! `data/tags.yaml`: what each tag is called.
//!
//! An image links to a raw name and nothing else. What a reader sees lives here, so renaming
//! `terminal` to `Terminal` for English and `终端` for Chinese never touches a single image,
//! and one concept cannot drift into three spellings across a library.
//!
//! The raw form is constrained -- lower case, digits, hyphens -- because it is an identifier
//! that happens to be readable. Technical names have one correctly cased display form;
//! ordinary names have a translated display form per locale. See spec/architecture.md.

use crate::i18n::store::Translation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Tag {
	/// A proper noun, brand, tool, format, protocol or organisation. Its name is not translated.
	Technical { display: String },
	/// A common noun that readers expect to see in their own language.
	Ordinary {
		#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
		display: BTreeMap<String, Translation>,
	},
}

impl Tag {
	pub fn ordinary() -> Self {
		Self::Ordinary {
			display: BTreeMap::new(),
		}
	}

	pub fn translations(&self) -> Option<&BTreeMap<String, Translation>> {
		match self {
			Self::Technical { .. } => None,
			Self::Ordinary { display } => Some(display),
		}
	}

	pub fn translations_mut(&mut self) -> Option<&mut BTreeMap<String, Translation>> {
		match self {
			Self::Technical { .. } => None,
			Self::Ordinary { display } => Some(display),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
	pub version: u32,
	#[serde(default)]
	pub tags: BTreeMap<String, Tag>,
}

impl Default for Registry {
	fn default() -> Self {
		Self {
			version: VERSION,
			tags: BTreeMap::new(),
		}
	}
}

pub fn path_for(repo: &Path) -> PathBuf {
	repo.join("data").join("tags.yaml")
}

pub fn load(path: &Path) -> Registry {
	std::fs::read_to_string(path)
		.ok()
		.and_then(|text| serde_yaml_ng::from_str(&text).ok())
		.unwrap_or_default()
}

pub fn save(path: &Path, registry: &Registry) -> std::io::Result<()> {
	let text =
		serde_yaml_ng::to_string(registry).map_err(|error| std::io::Error::other(error.to_string()))?;
	crate::image::store::write(path, text.as_bytes())
}

/// Every tag currently in use, for showing a model what already exists.
///
/// The whole list goes into the prompt. Left to itself a model invents `terminal-window`
/// beside `terminal` and `cli` beside both, and no instruction to "be consistent" fixes that
/// -- it has nothing to be consistent *with* unless it is shown. A hundred tags is nothing to
/// include; if this ever reaches thousands it becomes a retrieval problem rather than a
/// listing one.
pub fn known(registry: &Registry) -> Vec<&str> {
	registry.tags.keys().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn display(text: &str) -> Translation {
		Translation {
			text: text.into(),
			provider: "anthropic".into(),
			model: "claude-sonnet-5".into(),
			at: "2026-08-01T00:00:00Z".into(),
			seconds: 0.0,
			tokens: 0,
			review: false,
		}
	}

	#[test]
	fn technical_and_ordinary_tags_have_distinct_shapes() {
		// The point of the split: a reader never has to guess whether an empty map means a name
		// is universal or merely has not been translated yet.
		let mut registry = Registry::default();
		registry.tags.insert(
			"typescript".to_owned(),
			Tag::Technical {
				display: "TypeScript".to_owned(),
			},
		);
		registry.tags.insert(
			"terminal".to_owned(),
			Tag::Ordinary {
				display: BTreeMap::from([
					("en-US".to_owned(), display("Terminal")),
					("zh-CN".to_owned(), display("终端")),
				]),
			},
		);

		assert_eq!(
			registry.tags["typescript"],
			Tag::Technical {
				display: "TypeScript".to_owned()
			}
		);
		assert_eq!(
			registry.tags["terminal"].translations().expect("ordinary")["zh-CN"].text,
			"终端"
		);
		assert_eq!(known(&registry), vec!["terminal", "typescript"]);
	}

	#[test]
	fn the_registry_round_trips() {
		let mut registry = Registry::default();
		registry.tags.insert(
			"wasm".to_owned(),
			Tag::Technical {
				display: "Wasm".to_owned(),
			},
		);
		registry.tags.insert("terminal".to_owned(), Tag::ordinary());
		let text = serde_yaml_ng::to_string(&registry).expect("yaml");
		let back: Registry = serde_yaml_ng::from_str(&text).expect("parse");
		assert!(back.tags.contains_key("wasm"));
		assert!(
			back.tags["terminal"]
				.translations()
				.expect("ordinary")
				.is_empty()
		);
		assert!(text.contains("kind: technical"));
		assert!(text.contains("kind: ordinary"));
	}
}
