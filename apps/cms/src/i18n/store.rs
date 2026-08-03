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

/// What still needs translating: every (segment, locale) with no entry, or with an outdated one.
///
/// A recorded note request makes an existing translation outdated, because it was produced
/// before anyone decided the phrase had to survive. Detected from the text rather than from a
/// timestamp: the request says a phrase is kept verbatim, so a translation that does not contain
/// it cannot be carrying the note. That reads the same fact the instruction states, instead of
/// keeping a second record of when each was written and trusting the two to stay in step.
pub fn missing(
	sidecar: &Sidecar,
	live: &BTreeMap<String, super::segment::Segment>,
	locales: &[&str],
	glosses: &super::tn::Table,
) -> BTreeMap<String, Vec<String>> {
	let mut wanted = BTreeMap::new();
	for (id, _) in live.iter() {
		let have = sidecar.segments.get(id);
		let required = glosses
			.find(id)
			.map(|entry| entry.spans.as_slice())
			.unwrap_or_default();
		let absent: Vec<String> = locales
			.iter()
			.filter(|locale| match have.and_then(|map| map.get(**locale)) {
				None => true,
				Some(translation) => required
					.iter()
					.any(|span| !translation.text.contains(&span.phrase)),
			})
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

		let want = missing(
			&sidecar,
			&live,
			&["ja-JP", "de-DE", "fr-FR"],
			&crate::i18n::tn::Table::default(),
		);
		assert_eq!(want[&id], vec!["de-DE", "fr-FR"]);
	}

	#[test]
	fn a_recorded_note_request_outdates_the_translations_that_predate_it() {
		// The closing link in the chain. Without it, agreeing that a phrase needs a note changes
		// nothing until somebody reruns the whole article with --force, which costs every other
		// segment as well and is the kind of thing nobody does for one paragraph.
		let live = segment::translatable("古法 programming");
		let id = live.keys().next().expect("segment").clone();

		let mut kept = entry();
		kept.text = "古法 :tn[古法]{is=\"the old method\"} programming".into();
		let mut lost = entry();
		lost.text = "old-school programming".into();

		let mut sidecar = Sidecar::default();
		sidecar.segments.insert(
			id.clone(),
			BTreeMap::from([
				("ja-JP".to_owned(), kept),
				("de-DE".to_owned(), lost.clone()),
			]),
		);

		let mut glosses = crate::i18n::tn::Table::default();
		glosses.articles.insert(
			"milestone/a.md".to_owned(),
			crate::i18n::tn::Article {
				provider: "openai".to_owned(),
				model: "gpt-5-6-sol".to_owned(),
				at: "2026-08-02T00:00:00Z".to_owned(),
				tokens: 0,
				segments: BTreeMap::from([(
					id.clone(),
					crate::i18n::tn::Entry {
						source: "古法 programming".to_owned(),
						spans: vec![crate::i18n::tn::Gloss {
							phrase: "古法".to_owned(),
							guidance: "the old method".to_owned(),
						}],
					},
				)]),
			},
		);

		// Only the one that translated the phrase away is asked for again.
		let want = missing(&sidecar, &live, &["ja-JP", "de-DE"], &glosses);
		assert_eq!(want[&id], vec!["de-DE"]);

	}

	#[test]
	fn review_is_never_assumed() {
		// A machine may write a translation; only a person may vouch for one.
		assert!(!entry().review);
		let text = serde_yaml_ng::to_string(&entry()).expect("yaml");
		assert!(text.contains("review: false"));
	}
}
