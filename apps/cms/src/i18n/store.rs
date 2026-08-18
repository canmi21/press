//! The sidecar: `article.md` beside `article.i18n.yaml`.
//!
//! A map, never a document. It holds segment id to locale to translation and nothing about
//! order, because order lives in the article and two files with an opinion about it would
//! eventually disagree.
//!
//! YAML rather than JSON, and this one is meant to be edited. Translations are prose, which
//! JSON turns into a single escaped line that cannot be reviewed or diffed; `review` is a flag
//! a person sets by hand after reading. See spec/architecture/data.md.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;
pub const SOURCE_SIMILARITY: f64 = 0.9;

fn normalise_similarity(text: &str) -> Vec<char> {
	let mut normalised = String::new();
	let mut space = false;
	for character in text.to_lowercase().chars() {
		let character = match character {
			'“' | '”' => '"',
			'‘' | '’' => '\'',
			other => other,
		};
		if character.is_whitespace() {
			space = !normalised.is_empty();
		} else {
			if space {
				normalised.push(' ');
				space = false;
			}
			normalised.push(character);
		}
	}
	normalised.chars().collect()
}

pub fn similarity(left: &str, right: &str) -> f64 {
	let left = normalise_similarity(left);
	let right = normalise_similarity(right);
	if left == right {
		return 1.0;
	}
	if left.len() < 2 || right.len() < 2 {
		return 0.0;
	}
	let mut left_pairs: HashMap<(char, char), usize> = HashMap::new();
	for pair in left.windows(2) {
		*left_pairs.entry((pair[0], pair[1])).or_default() += 1;
	}
	let mut shared = 0usize;
	for pair in right.windows(2) {
		if let Some(count) = left_pairs.get_mut(&(pair[0], pair[1]))
			&& *count > 0
		{
			*count -= 1;
			shared += 1;
		}
	}
	(2 * shared) as f64 / (left.len() + right.len() - 2) as f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Translation {
	pub text: String,
	/// Who made it. `anthropic`, `openai`, `alibaba`, `deepseek`, or local `source` copy.
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

/// The sidecar beside an article, empty when the article has none yet.
///
/// A missing sidecar is an untranslated article, which is ordinary. A sidecar that does not
/// parse is not, and must never be read as an empty one: every locale would report as missing,
/// `cms i18n` would buy the whole article again, and the save at the end would write the result
/// over the file. This one is meant to be edited by hand -- see spec/i18n.md -- so a stray colon
/// is the expected way to break it, and the translations and `review` flags underneath are what
/// a silent overwrite would destroy.
pub fn load(path: &Path) -> std::io::Result<Sidecar> {
	Ok(load_checked(path)?.unwrap_or_else(|| Sidecar {
		version: VERSION,
		segments: BTreeMap::new(),
	}))
}

/// As `load`, but tells a missing sidecar apart from an empty one.
pub fn load_checked(path: &Path) -> std::io::Result<Option<Sidecar>> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
		Err(error) => return Err(error),
	};
	serde_yaml_ng::from_str(&text)
		.map(Some)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
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
/// timestamp, which reads the same fact the instruction states instead of keeping a second
/// record of when each was written and trusting the two to stay in step.
///
/// The note is what is looked for, and nothing else. The source wording is deliberately absent
/// from a finished translation -- it lives inside the note, where a reader meets it by choice --
/// so requiring the original phrase to appear would fail exactly the translations that got it
/// right.
pub fn missing(
	sidecar: &Sidecar,
	live: &BTreeMap<String, super::segment::Segment>,
	locales: &[&str],
	source_locale: Option<&str>,
	glosses: &super::tn::Table,
) -> BTreeMap<String, Vec<String>> {
	let mut wanted = BTreeMap::new();
	for (id, segment) in live.iter() {
		let have = sidecar.segments.get(id);
		let required = if segment.region == super::segment::Region::Body {
			glosses
				.find(id)
				.map(|entry| entry.spans.as_slice())
				.unwrap_or_default()
		} else {
			&[]
		};
		let absent: Vec<String> = locales
			.iter()
			.filter(|locale| match have.and_then(|map| map.get(**locale)) {
				None => true,
				Some(translation) => {
					let exact_source = source_locale == Some(**locale);
					let same_language = source_locale
						.is_some_and(|source| source.split('-').next() == (**locale).split('-').next());
					(exact_source && similarity(&segment.source, &translation.text) < SOURCE_SIMILARITY)
						|| (!same_language && !required.is_empty() && !translation.text.contains(":tn["))
				}
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
		let live = segment::translatable("a new paragraph").expect("segments");
		assert_eq!(orphans(&sidecar, &live), vec!["old"]);
	}

	#[test]
	fn only_the_absent_locales_are_asked_for() {
		let live = segment::translatable("hello world").expect("segments");
		let id = live.keys().next().expect("segment").clone();
		let mut sidecar = Sidecar::default();
		sidecar
			.segments
			.insert(id.clone(), BTreeMap::from([("ja-JP".to_owned(), entry())]));

		let want = missing(
			&sidecar,
			&live,
			&["ja-JP", "de-DE", "fr-FR"],
			Some("en-US"),
			&crate::i18n::tn::Table::default(),
		);
		assert_eq!(want[&id], vec!["de-DE", "fr-FR"]);
	}

	#[test]
	fn an_existing_source_view_that_was_rewritten_is_still_missing() {
		let live = segment::translatable("作者原本的句子和语气应该完整保留。这里还有第二句话。")
			.expect("segments");
		let id = live.keys().next().expect("segment").clone();
		let mut rewritten = entry();
		rewritten.text = "这段文字主要说明翻译应当尊重作者。".to_owned();
		let mut sidecar = Sidecar::default();
		sidecar.segments.insert(
			id.clone(),
			BTreeMap::from([("zh-CN".to_owned(), rewritten)]),
		);

		let wanted = missing(
			&sidecar,
			&live,
			&["zh-CN"],
			Some("zh-CN"),
			&crate::i18n::tn::Table::default(),
		);
		assert_eq!(wanted[&id], vec!["zh-CN"]);
	}

	#[test]
	fn a_recorded_note_request_outdates_the_translations_that_predate_it() {
		// The closing link in the chain. Without it, agreeing that a phrase needs a note changes
		// nothing until somebody reruns the whole article with --force, which costs every other
		// segment as well and is the kind of thing nobody does for one paragraph.
		let live = segment::translatable("古法 programming").expect("segments");
		let id = live.keys().next().expect("segment").clone();

		// The shape a correct translation has: wholly in its own language, with the original
		// inside the note rather than on the page.
		let mut noted = entry();
		noted.text =
			":tn[old-school]{is=\"the source reads 古法, an antique-craft word\"} programming".into();
		let mut plain = entry();
		plain.text = "old-school programming".into();

		let mut sidecar = Sidecar::default();
		sidecar.segments.insert(
			id.clone(),
			BTreeMap::from([("ja-JP".to_owned(), noted), ("de-DE".to_owned(), plain)]),
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

		// The annotated one is left alone; the one that lost the note is asked for again.
		let want = missing(
			&sidecar,
			&live,
			&["ja-JP", "de-DE"],
			Some("zh-CN"),
			&glosses,
		);
		assert_eq!(want[&id], vec!["de-DE"]);
	}

	#[test]
	fn a_note_does_not_outdate_same_language_views() {
		let live = segment::translatable("古法 programming").expect("segments");
		let id = live.keys().next().expect("segment").clone();
		let mut same_language = entry();
		same_language.text = "古法 programming".to_owned();
		let mut sidecar = Sidecar::default();
		sidecar.segments.insert(
			id.clone(),
			BTreeMap::from([
				("zh-CN".to_owned(), same_language.clone()),
				("zh-TW".to_owned(), same_language),
			]),
		);
		let mut glosses = crate::i18n::tn::Table::default();
		glosses.articles.insert(
			"article.md".to_owned(),
			crate::i18n::tn::Article {
				provider: "openai".to_owned(),
				model: "gpt-5-6-sol".to_owned(),
				at: "2026-08-02T00:00:00Z".to_owned(),
				tokens: 0,
				segments: BTreeMap::from([(
					id,
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

		assert!(
			missing(
				&sidecar,
				&live,
				&["zh-CN", "zh-TW"],
				Some("zh-CN"),
				&glosses,
			)
			.is_empty()
		);
	}

	#[test]
	fn a_broken_sidecar_is_an_error_rather_than_an_empty_one() {
		// The whole point of the checked read. Treated as empty, every locale reports missing,
		// `cms i18n` buys the article again, and the save at the end writes over the file a
		// person was hand-editing when they left the colon out. See spec/i18n.md.
		let path = std::env::temp_dir().join(format!("cms-sidecar-{}.i18n.yaml", std::process::id()));
		std::fs::write(&path, "segments: [this is not a map\n").expect("write");
		let error = load(&path).expect_err("a broken sidecar must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
		let _ = std::fs::remove_file(&path);
	}

	#[test]
	fn a_missing_sidecar_is_an_untranslated_article() {
		let sidecar = load(Path::new("/nonexistent/a.i18n.yaml")).expect("missing is not an error");
		assert!(sidecar.segments.is_empty());
	}

	#[test]
	fn review_is_never_assumed() {
		// A machine may write a translation; only a person may vouch for one.
		assert!(!entry().review);
		let text = serde_yaml_ng::to_string(&entry()).expect("yaml");
		assert!(text.contains("review: false"));
	}
}
