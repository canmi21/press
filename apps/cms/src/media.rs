//! `data/media.yaml`: what a picture is about, as opposed to what it is made of.
//!
//! The manifest beside it holds variants, dimensions and a thumbhash -- all of which `cms
//! image` can rebuild from the original at any time. Nothing here can be rebuilt. A
//! description cost money to produce and a person's time to check; tags are curated. Keeping
//! the two apart is what stops `cms image --force` from spending a rebuild's worth of pixels
//! and taking the words with it, which is exactly what it did while they shared a struct.
//!
//! YAML, and meant to be edited. See spec/architecture/data.md.

use crate::i18n::store::Translation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

/// What kind of image this is.
///
/// Closed, unlike tags. A distinction that can be drawn with a tag does not earn a category:
/// a terminal capture, a browser capture and an editor capture are all screenshots, and
/// letting each become its own kind would grow this list for every application that exists.
/// The category answers what sort of thing it is; tags answer what is in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
	/// A camera pointed at the world.
	Photograph,
	/// A capture of a screen.
	Screenshot,
	/// Drawn to explain something: charts, flowcharts, architecture.
	Diagram,
	/// Text is the subject: a scan, a slide, a page.
	Document,
	/// Illustration, rendering, generated imagery.
	Artwork,
}

/// Not yet read by anything: the command that asks a model to classify an image is the
/// next step, and this is the list it will be given. Kept whole so that step is a caller to
/// write rather than a taxonomy to argue about twice.
#[allow(dead_code)]
impl Category {
	pub fn parse(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().as_str() {
			"photograph" | "photo" => Some(Self::Photograph),
			"screenshot" => Some(Self::Screenshot),
			"diagram" => Some(Self::Diagram),
			"document" => Some(Self::Document),
			"artwork" => Some(Self::Artwork),
			_ => None,
		}
	}

	pub fn all() -> [&'static str; 5] {
		["photograph", "screenshot", "diagram", "document", "artwork"]
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub category: Option<Category>,
	/// What the image shows, per locale. The same shape a translated paragraph has, so both
	/// go through one pipeline.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub description: BTreeMap<String, Translation>,
	/// Raw tag names only. What each one is called in a given language lives in the registry,
	/// so renaming a tag for readers never touches the images that carry it.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
	pub version: u32,
	#[serde(default)]
	pub media: BTreeMap<String, Entry>,
}

impl Default for Media {
	fn default() -> Self {
		Self {
			version: VERSION,
			media: BTreeMap::new(),
		}
	}
}

pub fn path_for(repo: &Path) -> PathBuf {
	repo.join("data").join("media.yaml")
}

/// The descriptions and categories held against each asset, empty when the repository has none yet.
///
/// A parse failure is an error, never an empty set: every writer loads the whole file,
/// edits a few entries and saves it back, so reading a broken one as empty would replace descriptions bought one model call at a time
/// with nothing. Same rule as the sidecar and the image manifest.
pub fn load(path: &Path) -> std::io::Result<Media> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Media::default()),
		Err(error) => return Err(error),
	};
	serde_yaml_ng::from_str(&text)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

pub fn save(path: &Path, media: &Media) -> std::io::Result<()> {
	let text =
		serde_yaml_ng::to_string(media).map_err(|error| std::io::Error::other(error.to_string()))?;
	crate::image::store::write(path, text.as_bytes())
}

/// A tag as it may be written: lower case, digits and hyphens.
///
/// Constrained because a tag is an identifier that happens to be readable. `TypeScript`,
/// `typescript` and `type-script` would otherwise be three tags for one thing, and no amount
/// of care at the point of writing prevents that forever. What a reader sees comes from the
/// registry, where it may be capitalised, branded or translated freely.
///
/// Enforced once tagging exists; written now because the constraint is the reason the shape
/// works, not a detail of whoever gets round to calling it.
#[allow(dead_code)]
pub fn is_valid_tag(name: &str) -> bool {
	!name.is_empty()
		&& name
			.chars()
			.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
		&& !name.starts_with('-')
		&& !name.ends_with('-')
}

#[cfg(test)]
mod tests {
	#[test]
	fn a_broken_media_file_is_an_error_rather_than_an_empty_one() {
		// data/media.yaml holds descriptions bought one model call at a time, and every writer
		// saves the whole file back. Read as empty, the next save erases the lot.
		let path = std::env::temp_dir().join(format!("cms-media-{}.yaml", std::process::id()));
		std::fs::write(&path, "media: [not a map\n").expect("write");
		let error = super::load(&path).expect_err("a broken media file must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
		let _ = std::fs::remove_file(&path);
	}

	use super::*;

	#[test]
	fn a_tag_is_an_identifier_that_happens_to_be_readable() {
		assert!(is_valid_tag("terminal"));
		assert!(is_valid_tag("gpt-oss"));
		assert!(is_valid_tag("wasm2"));
		// Case and spacing are exactly how one concept becomes three tags.
		assert!(!is_valid_tag("TypeScript"));
		assert!(!is_valid_tag("shell terminal"));
		assert!(!is_valid_tag("-leading"));
		assert!(!is_valid_tag("trailing-"));
		assert!(!is_valid_tag(""));
	}

	#[test]
	fn a_category_is_closed_and_a_tag_is_not() {
		assert_eq!(Category::parse("Screenshot"), Some(Category::Screenshot));
		assert_eq!(Category::parse("photo"), Some(Category::Photograph));
		// Terminal is a thing in a screenshot, not a kind of image.
		assert_eq!(Category::parse("terminal"), None);
		assert_eq!(Category::all().len(), 5);
	}

	#[test]
	fn an_empty_entry_writes_nothing_but_itself() {
		// Absent fields are omitted, so an image with only a description does not carry an
		// empty tag list and a null category to say it has neither.
		let text = serde_yaml_ng::to_string(&Entry::default()).expect("yaml");
		assert_eq!(text.trim(), "{}");
	}

	#[test]
	fn categories_serialise_as_the_words_a_person_would_type() {
		let text = serde_yaml_ng::to_string(&Category::Photograph).expect("yaml");
		assert!(text.contains("photograph"), "{text}");
	}
}
