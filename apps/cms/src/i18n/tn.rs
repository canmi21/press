//! `data/tn.yaml`: which passages a translator has to gloss, and roughly how.
//!
//! Whether a passage needs a note is a judgement about the whole article -- does the text
//! around it already make the meaning recoverable? -- while translation happens one block at a
//! time. A per-block model cannot see that, which is why four articles produced no notes at all
//! however the rule was worded. So the judgement is made separately, by a model that reads the
//! article whole, and recorded here for the translator to obey.
//!
//! Keyed by segment id, so a request expires exactly when its paragraph changes. That is the
//! same property that makes a translation go stale, arriving for free rather than as a rule
//! somebody has to remember.
//!
//! Not under `data/build/`. That directory holds records rebuildable from what git already has;
//! this one costs a paid request and a person's agreement, which puts it beside `media.yaml`
//! and `tags.yaml`. See spec/architecture.md.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

/// One passage to keep in its original form and explain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gloss {
	/// The exact text to leave untranslated.
	pub phrase: String,
	/// What it means, for the translator to render in each target language.
	pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
	/// Enough of the block to recognise it without resolving a hash by hand.
	///
	/// This file exists to be read before it is trusted, and a table of hashes against advice is
	/// not reviewable. Never used for matching -- the id does that.
	pub source: String,
	pub spans: Vec<Gloss>,
}

/// One article's scan.
///
/// Grouped by article rather than kept as one flat map of segment ids, for a reason the flat
/// shape could not express: an article scanned and found to need nothing leaves no segments
/// behind, and is then indistinguishable from one never scanned at all. The record of having
/// looked has to exist separately from what was found.
///
/// Provenance sits here rather than on each finding, because one scan reads one article once.
/// Repeating it per segment would store the same four values as many times as the article has
/// suggestions, and invite them to disagree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Article {
	pub provider: String,
	pub model: String,
	pub at: String,
	pub tokens: u64,
	#[serde(default)]
	pub segments: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
	pub version: u32,
	/// Keyed the way `data/build/segments.json` keys its articles, so the two join on the same
	/// string instead of needing a rule for converting one into the other.
	#[serde(default)]
	pub articles: BTreeMap<String, Article>,
}

impl Default for Table {
	fn default() -> Self {
		Self {
			version: VERSION,
			articles: BTreeMap::new(),
		}
	}
}

impl Table {
	/// The findings for a segment, wherever they were recorded.
	///
	/// A segment id is the hash of its own text, so the same block in two articles is one entry
	/// in each and either answers. Translation asks about a segment and does not care which
	/// article carried the scan.
	pub fn find(&self, id: &str) -> Option<&Entry> {
		self.articles
			.values()
			.find_map(|article| article.segments.get(id))
	}

	/// Whether this article has been read, however little came back.
	pub fn scanned(&self, article: &str) -> bool {
		self.articles.contains_key(article)
	}
}

pub fn path_for(repo: &Path) -> PathBuf {
	repo.join("data").join("tn.yaml")
}

pub fn load(path: &Path) -> Table {
	std::fs::read_to_string(path)
		.ok()
		.and_then(|text| serde_yaml_ng::from_str(&text).ok())
		.unwrap_or_default()
}

pub fn save(path: &Path, table: &Table) -> std::io::Result<()> {
	let text =
		serde_yaml_ng::to_string(table).map_err(|error| std::io::Error::other(error.to_string()))?;
	crate::image::store::write(path, text.as_bytes())
}

/// The rule a flagged block adds to its translation prompt.
///
/// Two halves, and only having both is useful. Left alone the translator renders `古法` as
/// "old-school" -- reasonable English with nothing left to annotate -- so a note that glossed it
/// would be explaining a word no longer on the page. Keeping the phrase is what gives the note
/// something to point at.
pub fn rule(entry: &Entry) -> String {
	let mut rule = String::from(
		"- This block contains wording a reader of the target language cannot recover. For each \
		 phrase listed below: reproduce the phrase exactly as written, do not translate it, and \
		 place `:tn[phrase]{is=\"...\"}` immediately after it, with the explanation written in \
		 the target language. One note per phrase listed, and none for anything else.\n",
	);
	for span in &entry.spans {
		rule.push_str(&format!("  - {}: {}\n", span.phrase, span.guidance));
	}
	rule
}

/// What the scanner is asked, given a whole article.
///
/// The article goes in unfenced and unmasked, unlike a translation request: nothing here is
/// written back into content, so a prompt injection in the prose can at worst produce a
/// suggestion a person then declines. The reply is line-anchored for the same reason the
/// translator's is -- one malformed line costs one suggestion.
fn scan_prompt(article: &str) -> String {
	format!(
		"Read this article and find the passages a reader of another language could not recover \
		 from context once it is translated: a quoted idiom, a pun, a culturally local reference, \
		 a phrase whose force depends on how it is written rather than what it says.\n\
		 \n\
		 You are not translating. You are deciding what a translator will have to keep in the \
		 original and explain.\n\
		 \n\
		 Be sparing. A phrase whose meaning the surrounding sentences already carry needs no \
		 note, and a note on something obvious is worse than none. Most articles have very few. \
		 If there are none, say so and stop.\n\
		 \n\
		 For each one, output exactly one line:\n\
		 PHRASE\\tone sentence on what it means and why a translation would lose it\n\
		 \n\
		 The phrase must appear in the article exactly as you write it. Nothing else: no \
		 preamble, no numbering, no code fences.\n\
		 \n\
		 ---\n{article}\n---"
	)
}

/// Read the scanner's reply into phrases and their guidance.
pub fn parse_scan(reply: &str) -> Vec<Gloss> {
	reply
		.lines()
		.filter_map(|line| {
			let (phrase, guidance) = line.split_once('\t')?;
			let phrase = phrase.trim();
			let guidance = guidance.trim();
			(!phrase.is_empty() && !guidance.is_empty()).then(|| Gloss {
				phrase: phrase.to_owned(),
				guidance: guidance.to_owned(),
			})
		})
		.collect()
}

/// Attach each suggested phrase to the segment whose source contains it.
///
/// A suggestion the article does not contain is dropped rather than recorded. The scanner reads
/// the whole article and can paraphrase what it found; a phrase that cannot be located is one
/// no translator could keep verbatim either, so recording it would produce an instruction that
/// is impossible to follow.
pub fn attach(
	segments: &[super::segment::Segment],
	found: &[Gloss],
) -> Vec<(String, String, Vec<Gloss>)> {
	let mut by_segment: BTreeMap<String, (String, Vec<Gloss>)> = BTreeMap::new();
	for gloss in found {
		let Some(segment) = segments
			.iter()
			.find(|segment| segment.source.contains(&gloss.phrase))
		else {
			continue;
		};
		by_segment
			.entry(segment.id.clone())
			.or_insert_with(|| (segment.source.clone(), Vec::new()))
			.1
			.push(gloss.clone());
	}
	by_segment
		.into_iter()
		.map(|(id, (source, spans))| (id, source, spans))
		.collect()
}

/// The strong model reads the article once; the translator never sees the whole of it.
pub async fn scan(
	article: &str,
	runner: super::runner::Runner,
) -> Result<(Vec<Gloss>, String, u64), super::runner::Refusal> {
	let model = runner.model_for_scan();
	let answer = super::runner::ask(runner, &scan_prompt(article), model).await?;
	Ok((parse_scan(&answer.text), answer.model, answer.tokens))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn entry() -> Entry {
		Entry {
			source: "奇怪的是，放手之后质量并没有变差".to_owned(),
			spans: vec![Gloss {
				phrase: "古法".to_owned(),
				guidance: "literally 'the old method': writing every line by hand, before AI"
					.to_owned(),
			}],
		}
	}

	#[test]
	fn the_rule_keeps_the_phrase_as_well_as_explaining_it() {
		// Only asking for the note leaves nothing to annotate: the phrase would already have
		// been translated away by the time the note was written.
		let rule = rule(&entry());
		assert!(rule.contains("do not translate it"));
		assert!(rule.contains(":tn[phrase]"));
		assert!(rule.contains("古法"));
		assert!(rule.contains("the old method"));
	}

	fn article() -> Article {
		Article {
			provider: "openai".to_owned(),
			model: "gpt-5-6-sol".to_owned(),
			at: "2026-08-02T00:00:00Z".to_owned(),
			tokens: 0,
			segments: BTreeMap::from([("abc123".to_owned(), entry())]),
		}
	}

	#[test]
	fn a_table_round_trips_and_finds_a_segment_through_its_article() {
		let mut table = Table::default();
		table.articles.insert("milestone/a.md".to_owned(), article());
		let text = serde_yaml_ng::to_string(&table).expect("yaml");
		let back: Table = serde_yaml_ng::from_str(&text).expect("parse");
		assert_eq!(back.find("abc123"), Some(&entry()));
	}

	#[test]
	fn an_article_that_needed_nothing_still_counts_as_read() {
		// The reason for grouping by article at all. A flat map of segment ids cannot hold this:
		// an article with no findings leaves nothing behind, and a rerun would pay again to
		// learn the same nothing.
		let mut table = Table::default();
		let mut empty = article();
		empty.segments.clear();
		table.articles.insert("milestone/quiet.md".to_owned(), empty);
		assert!(table.scanned("milestone/quiet.md"));
		assert!(!table.scanned("milestone/unread.md"));
	}

	#[test]
	fn a_recorded_suggestion_reaches_the_prompt() {
		// Writing the entry is the decision. `cms tn` prints and only records when asked, so
		// there is no second approval to wait for -- an entry exists because somebody chose it.
		let article = "---\nlang: zh\n---\n\n古法 programming";
		let request = crate::i18n::prompt::build(
			&crate::i18n::segment::split(article)[0],
			"古法 programming",
			None,
			None,
			Some(&entry()),
		);
		assert!(request.text.contains("do not translate it"));
		assert!(request.text.contains("古法"));
	}

	#[test]
	fn a_suggestion_the_article_does_not_contain_is_dropped() {
		// The scanner reads the whole article and can paraphrase what it found. A phrase that
		// cannot be located is one no translator could keep verbatim either, so recording it
		// would produce an instruction impossible to follow.
		let segments = crate::i18n::segment::split("---\nlang: zh\n---\n\n奇怪的是，古法编程");
		let found = vec![
			Gloss {
				phrase: "古法".to_owned(),
				guidance: "the old method".to_owned(),
			},
			Gloss {
				phrase: "never written".to_owned(),
				guidance: "invented by the scanner".to_owned(),
			},
		];
		let attached = attach(&segments, &found);
		assert_eq!(attached.len(), 1);
		assert_eq!(attached[0].2.len(), 1);
		assert_eq!(attached[0].2[0].phrase, "古法");
	}

	#[test]
	fn a_reply_is_read_line_by_line_and_a_bad_line_costs_one_suggestion() {
		let found = parse_scan("古法\tthe old method\nnot a suggestion\n想开了\tcame to terms\n");
		assert_eq!(found.len(), 2);
		assert_eq!(found[1].phrase, "想开了");
	}

	#[test]
	fn a_reader_can_tell_what_an_entry_refers_to() {
		// The file is meant to be read before it steers anything, and hashes against advice is
		// not something a person can agree or disagree with.
		assert!(!entry().source.is_empty());
	}
}
