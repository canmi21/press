//! The sidecar: `article.md` beside `article.i18n.yaml`.
//!
//! A map, never a document. It holds segment id to locale to translation and nothing about
//! order, because order lives in the article and two files with an opinion about it would
//! eventually disagree.
//!
//! YAML rather than JSON, and this one is meant to be edited. Translations are prose, which
//! JSON turns into a single escaped line that cannot be reviewed or diffed; `review` is a flag
//! a person sets by hand after reading. See spec/architecture.md.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Translation {
	pub text: String,
	/// Who made it. `anthropic`, `openai`, `alibaba`, `deepseek`.
	pub provider: String,
	/// Normalised model id; see `model::Id`.
	pub model: String,
	/// ISO 8601 UTC, at the moment the request was sent rather than when it returned.
	pub at: String,
	/// Wall clock the request took.
	pub seconds: f64,
	pub tokens: u64,
	/// Whether a person has read this and vouched for it. Never set by a machine.
	#[serde(default)]
	pub review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sidecar {
	#[serde(default = "default_version")]
	pub version: u32,
	/// Segment id to locale to translation.
	#[serde(default)]
	pub segments: BTreeMap<String, BTreeMap<String, Translation>>,
}

fn default_version() -> u32 {
	VERSION
}

/// Where an article's translations live.
pub fn path_for(article: &Path) -> PathBuf {
	article.with_extension("i18n.yaml")
}

pub fn load(path: &Path) -> Sidecar {
	std::fs::read_to_string(path)
		.ok()
		.and_then(|text| serde_yaml_ng::from_str(&text).ok())
		.unwrap_or_else(|| Sidecar {
			version: VERSION,
			segments: BTreeMap::new(),
		})
}

pub fn save(path: &Path, sidecar: &Sidecar) -> std::io::Result<()> {
	let text =
		serde_yaml_ng::to_string(sidecar).map_err(|error| std::io::Error::other(error.to_string()))?;
	crate::image::store::write(path, text.as_bytes())
}

/// Segments present in the sidecar that the article no longer contains.
///
/// Editing a paragraph changes its id, so the old translations stay behind under the old key.
/// They are kept until swept rather than dropped on sight: a corrected typo leaves a
/// translation that is still almost right, and worth reading before it goes.
pub fn orphans(sidecar: &Sidecar, live: &BTreeMap<String, super::segment::Segment>) -> Vec<String> {
	sidecar
		.segments
		.keys()
		.filter(|id| !live.contains_key(*id))
		.cloned()
		.collect()
}

/// What still needs translating: every (segment, locale) with no entry.
pub fn missing(
	sidecar: &Sidecar,
	live: &BTreeMap<String, super::segment::Segment>,
	locales: &[&str],
) -> BTreeMap<String, Vec<String>> {
	let mut wanted = BTreeMap::new();
	for (id, _) in live.iter() {
		let have = sidecar.segments.get(id);
		let absent: Vec<String> = locales
			.iter()
			.filter(|locale| have.is_none_or(|map| !map.contains_key(**locale)))
			.map(|locale| (*locale).to_owned())
			.collect();
		if !absent.is_empty() {
			wanted.insert(id.clone(), absent);
		}
	}
	wanted
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::i18n::segment;

	fn entry() -> Translation {
		Translation {
			text: "こんにちは".into(),
			provider: "anthropic".into(),
			model: "claude-sonnet-5".into(),
			at: "2026-08-01T00:00:00Z".into(),
			seconds: 3.5,
			tokens: 1200,
			review: false,
		}
	}

	#[test]
	fn the_sidecar_sits_beside_the_article() {
		assert_eq!(
			path_for(Path::new("contents/mirror/a.md")),
			PathBuf::from("contents/mirror/a.i18n.yaml")
		);
	}

	#[test]
	fn prose_round_trips_without_becoming_one_escaped_line() {
		let mut sidecar = Sidecar::default();
		let mut locales = BTreeMap::new();
		let mut multi = entry();
		multi.text = "first line\n\nsecond line with \"quotes\" and: colons".into();
		locales.insert("ja-JP".to_owned(), multi.clone());
		sidecar.segments.insert("abc".to_owned(), locales);

		let text = serde_yaml_ng::to_string(&sidecar).expect("serialise");
		let back: Sidecar = serde_yaml_ng::from_str(&text).expect("parse");
		assert_eq!(back.segments["abc"]["ja-JP"], multi);
	}

	#[test]
	fn an_edited_paragraph_leaves_its_old_translation_behind() {
		// Not deleted on sight: after a typo fix the old text is still nearly right, and worth
		// a look before it is swept.
		let mut sidecar = Sidecar::default();
		sidecar.segments.insert(
			"old".to_owned(),
			BTreeMap::from([("ja-JP".to_owned(), entry())]),
		);
		let live = segment::translatable("a new paragraph");
		assert_eq!(orphans(&sidecar, &live), vec!["old"]);
	}

	#[test]
	fn only_the_absent_locales_are_asked_for() {
		let live = segment::translatable("hello world");
		let id = live.keys().next().expect("segment").clone();
		let mut sidecar = Sidecar::default();
		sidecar
			.segments
			.insert(id.clone(), BTreeMap::from([("ja-JP".to_owned(), entry())]));

		let want = missing(&sidecar, &live, &["ja-JP", "de-DE", "fr-FR"]);
		assert_eq!(want[&id], vec!["de-DE", "fr-FR"]);
	}

	#[test]
	fn review_is_never_assumed() {
		// A machine may write a translation; only a person may vouch for one.
		assert!(!entry().review);
		let text = serde_yaml_ng::to_string(&entry()).expect("yaml");
		assert!(text.contains("review: false"));
	}
}
