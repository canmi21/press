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
//! and `tags.yaml`. See spec/architecture/data.md.

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
		Self { version: VERSION, articles: BTreeMap::new() }
	}
}

impl Table {
	/// The findings for a segment, wherever they were recorded.
	///
	/// A segment id is the hash of its own text, so the same block in two articles is one entry
	/// in each and either answers. Translation asks about a segment and does not care which
	/// article carried the scan.
	pub fn find(&self, id: &str) -> Option<&Entry> {
		self.articles.values().find_map(|article| article.segments.get(id))
	}

	/// Whether this article has been read, however little came back.
	pub fn scanned(&self, article: &str) -> bool {
		self.articles.contains_key(article)
	}
}

pub fn path_for(repo: &Path) -> PathBuf {
	repo.join("data").join("tn.yaml")
}

/// The gloss table, empty when nothing has been scanned yet.
///
/// A parse failure is an error rather than an empty table for the reason the sidecar has the
/// same rule: the table is paid for, `cms tn` saves over it at the end of a scan, and reading a
/// broken file as empty would spend the money again and then erase what it replaced.
pub fn load(path: &Path) -> std::io::Result<Table> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Table::default()),
		Err(error) => return Err(error),
	};
	serde_yaml_ng::from_str(&text)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

pub fn save(path: &Path, table: &Table) -> std::io::Result<()> {
	let text =
		serde_yaml_ng::to_string(table).map_err(|error| std::io::Error::other(error.to_string()))?;
	crate::image::store::write(path, text.as_bytes())
}

/// The rule a flagged block adds to its translation prompt.
///
/// The note attaches to the translation, never to a retained original. A translation is expected
/// to be wholly in its own language, and fluent prose has nowhere to put a foreign clause: an
/// earlier version asked for the source wording to be kept verbatim and produced German
/// paragraphs ending in Chinese sentences. The original belongs inside the note, where a reader
/// meets it only if they want to.
///
/// The guidance below is given to the model as findings, not as copy. It was written in whatever
/// language the scan ran in, for a machine, about a source a reader of the target language has
/// not seen -- reproducing it verbatim would put an internal memo on the page.
pub fn rule(entry: &Entry) -> String {
	let mut rule = String::from(
		"- Some wording in this block carries an effect that does not survive localisation, \
		 including a same-language rewrite into more direct prose. Handle each item below like this \
		 for every requested locale:\n\
		 \n\
		 1. Translate or localise the passage naturally, as you would without this rule. The result \
		 must read as the target locale, with no source wording retained merely because its effect \
		 is difficult to carry. A same-language target may of course use the same script, but it \
		 still follows the direct-reading rewrite policy.\n\
		 2. Wrap the translated words that stand where the original effect was in \
		 `:tn[translated words]{is=\"...\"}`. Copy that shape exactly: a closing `]`, then \
		 `{is=\"`, the note, then `\"}` with the closing quote present. Never use a straight \
		 double quote inside the note -- there is no way to escape one there and it ends the \
		 note early. Use curly quotes or none. A marker missing any of this is discarded whole \
		 and the block is asked for again.\n\
		 3. The wrapped words are the translation, in the target language and its own script. \
		 Never a romanisation -- pinyin is neither the original nor a translation -- and never \
		 the source script carried into the sentence.\n\
		 4. Write `is` for someone who reads only the target language and has not seen the \
		 original. Quote the original word exactly as the source wrote it, in its original \
		 script, then say what it did -- the joke, the tone, the register, whatever is actually \
		 lost. One or two sentences.\n\
		 \n\
		 The findings below are notes to you, not text to reproduce. Do not translate them and do \
		 not paste them: they were written about a source your reader has never read. Work out \
		 for yourself what that reader needs in order to feel what the original does, and write \
		 that. A note that explains the joke well in one language may need to be shorter, longer \
		 or differently aimed in another, and that is your judgement to make.\n\
		 \n\
		 One note per item listed, and none for anything else in this block.\n",
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
		 You are not translating. The passage will be translated normally; you are deciding \
		 which word the translation should carry a footnote about, so that a reader learns what \
		 the original did there.\n\
		 \n\
		 So name the shortest span that carries the effect, and never more. If a clause reads \
		 oddly, the word inside it is what you want -- `鸽` and not `从 Next.js 13 鸽到 Next.js \
		 16`, `摸鱼` and not `不许 Cargo 再摸鱼了`. A note attaches to a word; naming a whole \
		 sentence produces a footnote longer than the thing it explains.\n\
		 If that shortest span occurs more than once in the article, include just enough adjacent \
		 characters to identify this occurrence uniquely, while staying within eight characters.\n\
		 \n\
		 Some effects have no word to attach to: a rhythm across two sentences, a callback \
		 between a title and an ending. Those are real losses and there is nothing to point at, \
		 so do not report them. A note explains one word; anything wider is a loss to accept \
		 rather than a note to write.\n\
		 \n\
		 Be sparing. A phrase whose meaning the surrounding sentences already carry needs no \
		 note, and a note on something obvious is worse than none. Most articles have very few. \
		 If there are none, say so and stop.\n\
		 \n\
		 For each one, output exactly one line:\n\
		 WORD\\tone sentence on what it means and why a translation would lose it\n\
		 \n\
		 The word must appear in the article exactly as you write it. Nothing else: no \
		 preamble, no numbering, no code fences. Each explanation is one sentence and at most \
		 180 characters. This is a single-turn text analysis: everything needed is below. Do \
		 not inspect files, repository rules, previous notes or version control, and do not \
		 describe how you will work.\n\
		 \n\
		 ---\n{article}\n---"
	)
}

/// The longest span worth keeping in the original, in characters.
///
/// A note holds one word in place and footnotes it; the sentence around it is still translated.
/// Past a few characters the thing being kept is a clause, and keeping a clause does not
/// annotate a translation -- it cancels it, leaving a German page with a Chinese sentence in the
/// middle of it. Measured on the first run: `果果`, `摆烂了` and `MUSL 厨` read well, while
/// `从 Next.js 13 鸽到 Next.js 16` and `只剩下一个"清"字可以形容` left whole clauses untranslated
/// in all eight languages.
///
/// A limit rather than only an instruction, because the instruction is advice to a model and
/// this is the property the output has to have.
const LONGEST_SPAN: usize = 8;
const LONGEST_GUIDANCE: usize = 180;

/// Read the scanner's reply into phrases and their guidance.
pub fn parse_scan(reply: &str) -> Vec<Gloss> {
	reply
		.lines()
		.filter_map(|line| {
			let (phrase, guidance) = line.split_once('\t')?;
			let phrase = phrase.trim();
			let guidance = guidance.trim();
			let short = phrase.chars().count() <= LONGEST_SPAN;
			let concise = guidance.chars().count() <= LONGEST_GUIDANCE;
			(!phrase.is_empty() && !guidance.is_empty() && short && concise)
				.then(|| Gloss { phrase: phrase.to_owned(), guidance: guidance.to_owned() })
		})
		.collect()
}

/// Whether every occurrence of `phrase` in `source` sits inside an `:fn` note's `is` attribute.
///
/// A phrase there needs no translator's note: the author's note is already an explanation
/// channel, and its translation simply renders an equivalent expression. Worse than needless, a
/// note there is impossible -- `:tn` cannot nest inside the attribute, since the straight quote
/// that would open it ends the attribute instead -- so recording one wedges the segment: every
/// answer either omits the demanded note or breaks the directive's shape, and the run buys the
/// same refusal forever. Found that way: `脱裤子放屁` lived inside an `:fn` explanation, and
/// three model attempts correctly declined to do the impossible.
fn only_inside_note_attributes(source: &str, phrase: &str) -> bool {
	let mut inside: Vec<(usize, usize)> = Vec::new();
	let mut rest = 0;
	while let Some(at) = source[rest..].find("{is=\"") {
		let open = rest + at + 5;
		let Some(len) = source[open..].find("\"}") else {
			break;
		};
		inside.push((open, open + len));
		rest = open + len + 2;
	}
	let mut found_any = false;
	let mut from = 0;
	while let Some(at) = source[from..].find(phrase) {
		let here = from + at;
		found_any = true;
		if !inside.iter().any(|(a, b)| here >= *a && here < *b) {
			return false;
		}
		from = here + phrase.len();
	}
	found_any
}

/// Attach each suggested phrase to the segment whose source contains it.
///
/// A suggestion the article does not contain is dropped rather than recorded. The scanner reads
/// the whole article and can paraphrase what it found; a phrase that cannot be located is one
/// no translator could keep verbatim either, so recording it would produce an instruction that
/// is impossible to follow. A phrase living only inside `:fn` note attributes is dropped for
/// the same shape of reason -- see `only_inside_note_attributes`.
pub fn attach(
	segments: &[super::segment::Segment],
	found: &[Gloss],
) -> Vec<(String, String, Vec<Gloss>)> {
	let mut by_segment: BTreeMap<String, (String, Vec<Gloss>)> = BTreeMap::new();
	for gloss in found {
		let mut matches = segments.iter().filter(|segment| {
			segment.region == super::segment::Region::Body && segment.source.contains(&gloss.phrase)
		});
		let Some(segment) = matches.next() else {
			continue;
		};
		if matches.next().is_some() {
			continue;
		}
		if only_inside_note_attributes(&segment.source, &gloss.phrase) {
			continue;
		}
		by_segment
			.entry(segment.id.clone())
			.or_insert_with(|| (segment.source.clone(), Vec::new()))
			.1
			.push(gloss.clone());
	}
	by_segment.into_iter().map(|(id, (source, spans))| (id, source, spans)).collect()
}

/// The strong model reads the article once; the translator never sees the whole of it.
pub async fn scan(
	article: &str,
	runner: super::runner::Runner,
	model_override: Option<&str>,
) -> Result<(Vec<Gloss>, String, u64), super::runner::Refusal> {
	let model = model_override.unwrap_or_else(|| runner.model_for_scan());
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
				guidance: "literally 'the old method': writing every line by hand, before AI".to_owned(),
			}],
		}
	}

	#[test]
	fn the_rule_annotates_the_translation_rather_than_keeping_the_source() {
		// An earlier version asked for the source wording to be kept verbatim, which produced
		// German paragraphs ending in Chinese sentences. A translation is wholly its own
		// language; the original belongs inside the note.
		let rule = rule(&entry());
		assert!(rule.contains("Translate or localise the passage naturally"));
		assert!(rule.contains("same-language target may of course use the same script"));
		assert!(rule.contains(":tn[translated words]"));
		// The finding still reaches the model, as a finding.
		assert!(rule.contains("古法"));
		assert!(rule.contains("the old method"));
	}

	#[test]
	fn frontmatter_suggestions_are_never_attached() {
		let segments =
			crate::i18n::segment::split("---\ntitle: A local idiom\n---\n\nBody without that phrase.")
				.expect("segments");
		let found = vec![Gloss { phrase: "local idiom".to_owned(), guidance: "context".to_owned() }];

		assert!(attach(&segments, &found).is_empty());
	}

	#[test]
	fn the_findings_are_marked_as_notes_rather_than_copy() {
		// Written for a machine, in whichever language the scan ran, about a source the reader
		// has not seen. Pasted through, they put an internal memo on the page.
		let rule = rule(&entry());
		assert!(rule.contains("notes to you, not text to reproduce"));
		assert!(rule.contains("Work out for yourself"));
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
			&crate::i18n::segment::split(article).expect("segments")[0],
			"古法 programming",
			None,
			None,
			Some(&entry()),
		);
		assert!(request.text.contains("Translate or localise the passage naturally"));
		assert!(request.text.contains("古法"));
	}

	#[test]
	fn a_suggestion_the_article_does_not_contain_is_dropped() {
		// The scanner reads the whole article and can paraphrase what it found. A phrase that
		// cannot be located is one no translator could keep verbatim either, so recording it
		// would produce an instruction impossible to follow.
		let segments =
			crate::i18n::segment::split("---\nlang: zh\n---\n\n奇怪的是，古法编程").expect("segments");
		let found = vec![
			Gloss { phrase: "古法".to_owned(), guidance: "the old method".to_owned() },
			Gloss { phrase: "never written".to_owned(), guidance: "invented by the scanner".to_owned() },
		];
		let attached = attach(&segments, &found);
		assert_eq!(attached.len(), 1);
		assert_eq!(attached[0].2.len(), 1);
		assert_eq!(attached[0].2[0].phrase, "古法");
	}

	#[test]
	fn a_phrase_living_only_inside_a_note_attribute_is_dropped() {
		let segment = super::super::segment::Segment {
			id: "s1".into(),
			kind: super::super::segment::Kind::Prose,
			source: "结果就是 :fn[乱]{is=\"基本上就是脱裤子放屁\"} 而已".into(),
			region: super::super::segment::Region::Body,
			start: 0,
			end: 0,
		};
		let inside = Gloss { phrase: "脱裤子放屁".into(), guidance: "a crude idiom".into() };
		assert!(attach(&[segment.clone()], &[inside]).is_empty());
		// The same phrase in open prose still attaches.
		let open =
			super::super::segment::Segment { source: "这就是脱裤子放屁而已".into(), ..segment };
		let gloss = Gloss { phrase: "脱裤子放屁".into(), guidance: "a crude idiom".into() };
		assert_eq!(attach(&[open], &[gloss]).len(), 1);
	}

	#[test]
	fn an_ambiguous_phrase_is_dropped_instead_of_attached_to_the_first_match() {
		let segments = crate::i18n::segment::split(
			"---\nlang: zh\n---\n\n第一段有一个清字。\n\n第二段也有一个清字。",
		)
		.expect("segments");
		let found = vec![Gloss {
			phrase: "清".to_owned(),
			guidance: "a word whose effect depends on this occurrence".to_owned(),
		}];

		assert!(attach(&segments, &found).is_empty());
	}

	#[test]
	fn a_reply_is_read_line_by_line_and_a_bad_line_costs_one_suggestion() {
		let found = parse_scan("古法\tthe old method\nnot a suggestion\n想开了\tcame to terms\n");
		assert_eq!(found.len(), 2);
		assert_eq!(found[1].phrase, "想开了");
	}

	#[test]
	fn agent_chatter_cannot_hide_inside_a_translation_note() {
		let chatter = "x".repeat(LONGEST_GUIDANCE + 1);
		assert!(parse_scan(&format!("发电\t{chatter}")).is_empty());
	}

	#[test]
	fn a_span_longer_than_a_word_is_not_recorded() {
		// Keeping a clause in the original does not annotate a translation, it cancels it. The
		// first run left `从 Next.js 13 鸽到 Next.js 16` untranslated in all eight languages
		// because the instruction alone did not hold.
		let found = parse_scan(
			"鸽\tto flake on a plan\n从 Next.js 13 鸽到 Next.js 16\tthe same joke, as a clause\n",
		);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].phrase, "鸽");
	}

	#[test]
	fn a_broken_table_is_an_error_rather_than_an_empty_one() {
		// `cms tn` saves the whole table back at the end of a scan, so reading a broken one as
		// empty would pay for every gloss again and then erase what it replaced.
		let path = std::env::temp_dir().join(format!("cms-tn-{}.yaml", std::process::id()));
		std::fs::write(&path, "articles: [not a map\n").expect("write");
		let error = load(&path).expect_err("a broken table must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
		let _ = std::fs::remove_file(&path);
	}

	#[test]
	fn a_reader_can_tell_what_an_entry_refers_to() {
		// The file is meant to be read before it steers anything, and hashes against advice is
		// not something a person can agree or disagree with.
		assert!(!entry().source.is_empty());
	}
}
