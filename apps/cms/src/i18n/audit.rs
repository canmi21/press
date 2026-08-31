//! Report-only checks on stored translations: the note policies that shape cannot enforce.
//!
//! Two policies live here, both from spec/i18n.md. An author's note (`:fn`) shows its words and
//! its explanation together at the end of the article as one continuous statement, so an
//! explanation that restates the words reads the word twice. A translator's note (`:tn`) wraps
//! the translated words -- never the source script carried into the sentence -- and quotes the
//! original span, verbatim, inside the note.
//!
//! Both are soft: a target grammar can force a restatement, and a same-script locale can carry
//! the original legitimately. So nothing here rejects or re-asks; `cms i18n --check` prints the
//! findings and a person judges them. A hard gate would fail exactly the defensible minority
//! the policy allows for.

use super::segment::Kind;
use super::tn;
use super::width;

/// The locales whose scripts legitimately contain Han characters, so a `:tn` wrapping them
/// says nothing about whether the words were translated.
const HAN_SCRIPT_LOCALES: [&str; 3] = ["zh-CN", "zh-TW", "ja-JP"];

/// Marks that open something and are followed immediately by what they open. Deliberately
/// without the French guillemet, which does take a space in its own typography.
const OPENING_MARKS: [char; 6] = ['¿', '¡', '(', '[', '\u{201C}', '\u{2018}'];

/// One suspect translation, named precisely enough to find and judge.
#[derive(Debug, PartialEq)]
pub struct Finding {
	pub segment: String,
	pub locale: String,
	pub reason: String,
}

/// Every `name[words]{is="note"}` directive in a text, as (words, note) pairs.
///
/// Tolerant where `validate::well_formed` is strict: this runs over stored translations that
/// already passed validation, and a malformed straggler is simply not a pair to audit.
fn directives(text: &str, name: &str) -> Vec<(String, String)> {
	let mut found = Vec::new();
	let mut rest = text;
	while let Some(at) = rest.find(name) {
		rest = &rest[at + name.len()..];
		let Some(words_end) = rest.find(']') else {
			break;
		};
		let words = &rest[..words_end];
		let after = &rest[words_end + 1..];
		let Some(tail) = after.strip_prefix("{is=\"") else {
			continue;
		};
		let Some(note_end) = tail.find("\"}") else {
			break;
		};
		found.push((words.to_owned(), tail[..note_end].to_owned()));
		rest = &tail[note_end + 2..];
	}
	found
}

/// The stretches of a recorded phrase a note is able to reproduce.
///
/// A note cannot contain a straight double quote -- the directive's attribute has no escape for
/// one -- so a phrase carrying one is cited around it rather than through it. A phrase without
/// one yields exactly itself, which is every phrase but the rare case this exists for.
fn quotable(phrase: &str) -> impl Iterator<Item = &str> {
	phrase.split('"').filter(|piece| !piece.is_empty())
}

fn has_han(text: &str) -> bool {
	text.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// Leading characters that are neither letters nor digits carry no restatement signal.
fn normalised(text: &str) -> String {
	text.trim_start_matches(|c: char| !c.is_alphanumeric()).to_lowercase()
}

/// The policy findings for one stored translation of one body segment.
///
/// `kind` decides whether the navigation-width policy applies; `source` is what the heading was
/// before translation, which is the only honest yardstick for it.
pub fn of(
	segment_id: &str,
	locale: &str,
	translation: &str,
	kind: Kind,
	source: &str,
	glosses: Option<&tn::Entry>,
) -> Vec<Finding> {
	let mut findings = Vec::new();
	let finding =
		|reason: String| Finding { segment: segment_id.to_owned(), locale: locale.to_owned(), reason };

	// A section heading is also a rail label. Two lines are allowed and `validate` refuses only
	// what gets cut off, so the interesting band -- wider than a line, still readable -- is
	// reported here with the source beside it, because whether a language could have said it
	// shorter is the judgement a person makes and a threshold cannot.
	//
	// A subsection is never listed in the rail, so no width can be wrong for it and nothing here
	// is a rule it broke. It is still worth saying when one runs very long: it is read in the
	// prose, where the source headings are short, and a translation several times their length is
	// usually the translator explaining the section rather than naming it. Reported at the clamp
	// -- twice a rail line -- so only the genuinely long ones are mentioned.
	if kind == Kind::Heading {
		let columns = width::of(translation);
		match width::level(source) {
			Some(2) if columns > width::ONE_LINE => findings.push(finding(format!(
				"heading wraps the table of contents ({columns} columns, source is {}; one line is {})",
				width::of(source),
				width::ONE_LINE,
			))),
			Some(level) if level > 2 && columns > width::CLAMP => findings.push(finding(format!(
				"subsection heading runs long ({columns} columns, source is {}); it is not in the \
				 table of contents, so this is a reading judgement rather than a fit",
				width::of(source),
			))),
			_ => {}
		}
	}

	// The two wrong-block checks, reported over what is already stored. `validate` refuses these
	// on arrival now, but everything bought before it did is still on disk, and a person needs
	// the whole list at once rather than one failure per build.
	if !super::validate::author_notes_preserved(source, translation) {
		findings.push(finding(format!(
			"carries {} author's notes where the source has {} -- probably a translation of a \
			 neighbouring block",
			translation.matches(":fn[").count(),
			source.matches(":fn[").count(),
		)));
	}
	if !super::validate::markers_resolved(translation) {
		findings.push(finding(
			"carries a code marker that stands for nothing -- text copied from the neighbouring \
			 context"
				.to_owned(),
		));
	}

	// The same rule `validate` now refuses on arrival, reported over what is already stored.
	if !super::validate::spacing_intact(translation) {
		findings.push(finding("a note directive is glued to the word beside it".to_owned()));
	}

	// The correction overshooting: told to space a directive off the word beside it, a model
	// also spaces it off an opening mark, where the mark is already the boundary and the space
	// is a typographic error -- Spanish `¿ Modelo`. Reported rather than refused: which marks
	// take a space is a per-language typographic convention, and French genuinely spaces its
	// guillemets, so a threshold here would be wrong somewhere.
	if OPENING_MARKS.iter().any(|mark| {
		translation.contains(&format!("{mark} :fn[")) || translation.contains(&format!("{mark} :tn["))
	}) {
		findings.push(finding("a space follows an opening mark before a note directive".to_owned()));
	}

	// An author's note explanation continues from its words; opening by restating them is the
	// double reading the policy exists to avoid.
	for (words, note) in directives(translation, ":fn") {
		if !words.is_empty() && normalised(&note).starts_with(&normalised(&words)) {
			findings.push(finding(format!(":fn explanation restates the words it follows ({words})")));
		}
	}

	let tn_pairs = directives(translation, ":tn");
	// The wrapped words are the translation. Han inside them, in a locale whose script has
	// none, is the source carried into the sentence -- or a romanisation's sibling failure.
	if !HAN_SCRIPT_LOCALES.contains(&locale) {
		for (words, _) in &tn_pairs {
			if has_han(words) {
				findings.push(finding(format!(":tn wraps untranslated source script ({words})")));
			}
		}
	}
	// The note quotes the original span verbatim, in its original script; a reader who has
	// never seen the source meets the original word only here.
	//
	// Checked against the longest run of the phrase a note could actually contain, which for
	// almost every phrase is the whole of it. A straight double quote is the exception: the
	// directive's attribute has no escape for one, so a phrase carrying it -- `"清"字`, the author
	// quoting a single character -- can never be reproduced verbatim, and comparing it literally
	// reported all eight locales of a segment whose notes cite the character correctly. A
	// permanent false positive on a report a person reads is worse than no report: it hides the
	// findings that are real.
	if let Some(entry) = glosses {
		for span in &entry.spans {
			let quoted =
				quotable(&span.phrase).any(|piece| tn_pairs.iter().any(|(_, note)| note.contains(piece)));
			if !tn_pairs.is_empty() && !quoted {
				findings.push(finding(format!(":tn note does not quote the original ({})", span.phrase)));
			}
		}
	}

	findings
}

/// Below this many columns a same-language pair may legitimately differ by half.
///
/// The two views of one language are free to diverge in exactly the way this check looks for --
/// one leaves a six-character fragment alone while the other spells it into a sentence. On a
/// short block that is a style difference and says nothing; on a long one there is no wording
/// choice that halves a paragraph, so the short blocks are simply not asked about. Measured over
/// the corpus, every divergence under this width was benign and the one above it was a block
/// answered with its neighbour's text.
const PAIR_FLOOR: usize = 60;

/// How far apart two views of one language may run before it is worth a person's eye.
const PAIR_RATIO: f64 = 0.75;

/// Findings that only exist when the locales of one segment are read together.
///
/// Every other check here reads one translation on its own, which is why the block whose zh-TW
/// answer was its neighbour's paragraph passed all of them: it was well-formed, the right
/// length for a paragraph, and carried every marker it was given. What it was not was the same
/// length as its own zh-CN sibling -- and two views of one language, translated from one source,
/// have no reason to differ by half. The sibling is the only yardstick that noticed.
///
/// Report-only, like everything else in this file: the pair is a signal, not a rule. See
/// spec/i18n.md.
pub fn across_locales(segment_id: &str, source: &str, texts: &[(&str, &str)]) -> Vec<Finding> {
	let mut findings = Vec::new();
	if width::raw(source) < PAIR_FLOOR {
		return findings;
	}
	for (index, (locale, text)) in texts.iter().enumerate() {
		let language = locale.split('-').next();
		for (other, other_text) in texts.iter().skip(index + 1) {
			if other.split('-').next() != language {
				continue;
			}
			let (a, b) = (width::raw(text), width::raw(other_text));
			let (low, high) = (a.min(b), a.max(b));
			if high == 0 {
				continue;
			}
			#[expect(
				clippy::cast_precision_loss,
				reason = "column counts are far below the range where f64 loses integers"
			)]
			let ratio = low as f64 / high as f64;
			if ratio < PAIR_RATIO {
				findings.push(Finding {
					segment: segment_id.to_owned(),
					locale: format!("{locale}/{other}"),
					reason: format!(
						"two views of one language differ by half ({locale} {a} columns, {other} \
						 {b}, source {}) -- one of them is probably about a different block",
						width::raw(source),
					),
				});
			}
		}
	}
	findings
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Prose whose source is stood in for by one carrying the same author's notes.
	///
	/// The note policies under test read neither the source's words nor its width, but the
	/// wrong-block check counts its `:fn` directives, and a source of `""` would make every
	/// fixture below look like an answer about a different block.
	fn of_prose(id: &str, locale: &str, text: &str, glosses: Option<&tn::Entry>) -> Vec<Finding> {
		let source = ":fn[x]{is=\"y\"}".repeat(text.matches(":fn[").count());
		of(id, locale, text, Kind::Prose, &source, glosses)
	}

	fn gloss(phrase: &str) -> tn::Entry {
		tn::Entry {
			source: String::new(),
			spans: vec![tn::Gloss { phrase: phrase.to_owned(), guidance: String::new() }],
		}
	}

	#[test]
	fn a_restating_explanation_is_reported_and_a_continuing_one_is_not() {
		let restating =
			of_prose("s", "en-US", r#"The :fn[model]{is="model means the runtime"} here"#, None);
		assert_eq!(restating.len(), 1);
		assert!(restating[0].reason.contains("restates"));
		let continuing =
			of_prose("s", "en-US", r#"The :fn[model]{is="the runtime, not the data"} here"#, None);
		assert!(continuing.is_empty());
	}

	#[test]
	fn restatement_ignores_case_and_leading_punctuation() {
		let found = of_prose("s", "en-US", r#":fn[Seam]{is="-- seam is a protocol"} x"#, None);
		assert_eq!(found.len(), 1);
	}

	#[test]
	fn han_inside_a_latin_locales_tn_is_reported() {
		let text = r#"he :tn[鸽了]{is="a note"} it"#;
		assert_eq!(of_prose("s", "en-US", text, None).len(), 1);
		// The same words in a Han-script locale say nothing.
		assert!(of_prose("s", "zh-TW", text, None).is_empty());
	}

	#[test]
	fn a_note_that_skips_the_original_is_reported_and_a_quoting_one_is_not() {
		let glosses = gloss("鸽");
		let skipping = r#"he :tn[put it off]{is="the source said postponing, like a no-show"} it"#;
		assert_eq!(of_prose("s", "en-US", skipping, Some(&glosses)).len(), 1);
		let quoting = r#"he :tn[put it off]{is="the source word 鸽, literally pigeon, means standing someone up"} it"#;
		assert!(of_prose("s", "en-US", quoting, Some(&glosses)).is_empty());
	}

	#[test]
	fn two_views_of_one_language_are_read_against_each_other() {
		// The block that started this: zh-TW held the next paragraph's translation. Well-formed,
		// a plausible paragraph length, every marker present -- and half the length of its own
		// zh-CN sibling, which is the only thing about it that was wrong.
		let source = "今天已经是我 move 到 US 来的整整一个月了；落地 NC 日常安顿好后，周围的地区和比较近的景点也基本上逛完了，那么人就闲下来了。";
		let found = across_locales(
			"s",
			source,
			&[
				(
					"en-US",
					"Today marks exactly one month since I moved to the US; after landing in NC and getting settled in, I have seen most of what is nearby, so now I have time on my hands.",
				),
				(
					"zh-CN",
					"今天正好是我搬到美国满一个月。落地北卡后，日常起居都安顿好了，附近的地区和几个稍近的景点也基本逛遍了，人就闲了下来。",
				),
				("zh-TW", "不過在開始吐槽別人之前，大概還是得先補一點背景知識"),
			],
		);
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].locale, "zh-CN/zh-TW");
		assert!(found[0].reason.contains("differ by half"));

		// Two views that agree are silent, and locales of different languages are never compared:
		// German is simply longer than Japanese and always was.
		assert!(
			across_locales(
				"s",
				source,
				&[
					(
						"zh-CN",
						"今天正好是我搬到美国满一个月。落地北卡后，日常起居都安顿好了，附近的地区也基本逛遍了。"
					),
					(
						"zh-TW",
						"今天正好是我搬到美國滿一個月。落地北卡後，日常起居都安頓好了，附近的地區也基本逛遍了。"
					),
				],
			)
			.is_empty()
		);
	}

	#[test]
	fn a_short_block_may_have_divergent_views_of_one_language() {
		// Under the floor a same-language pair legitimately differs by half: one leaves the
		// fragment alone and the other spells it into a sentence. Both are correct.
		assert!(
			across_locales(
				"s",
				"甚至模式匹配",
				&[("zh-CN", "甚至连模式匹配也不例外。"), ("zh-TW", "甚至模式匹配")],
			)
			.is_empty()
		);
	}

	#[test]
	fn a_wrong_block_answer_is_reported_over_what_is_already_stored() {
		// The same invariant `validate` refuses on arrival: an author's note the source does not
		// have came from somewhere else.
		let found = of(
			"s",
			"ko-KR",
			r#"지난 글에서도 썼듯이 :fn[프로토콜 노드]{is="구조 단위"}"#,
			Kind::Prose,
			"只可惜之前我 UI 选了 React 开始动刀",
			None,
		);
		assert_eq!(found.len(), 1);
		assert!(found[0].reason.contains("neighbouring block"));
	}

	#[test]
	fn a_phrase_that_cannot_be_quoted_verbatim_is_matched_without_its_quotes() {
		// `"清"字` is the author quoting one character. No note can reproduce it: a straight
		// double quote ends the attribute, so a literal comparison fails however well the note
		// cites the character.
		let glosses = gloss("\"清\"字");
		assert_eq!(quotable("\"清\"字").collect::<Vec<_>>(), vec!["清", "字"]);
		// A phrase with no quote in it is compared whole, as before.
		assert_eq!(quotable("两点一线").collect::<Vec<_>>(), vec!["两点一线"]);

		let citing =
			r#"the :tn[clear]{is="The original uses one character, 清, at once fresh and cool"} smell"#;
		assert!(of_prose("s", "en-US", citing, Some(&glosses)).is_empty());

		let silent = r#"the :tn[clear]{is="no single English word carries all three senses"} smell"#;
		assert_eq!(of_prose("s", "en-US", silent, Some(&glosses)).len(), 1);
	}

	#[test]
	fn a_translation_with_no_directives_has_no_findings() {
		assert!(of_prose("s", "en-US", "plain prose", None).is_empty());
		// A recorded gloss without any tn in the text is missing work, not a policy finding --
		// the completeness audit owns that.
		assert!(of_prose("s", "en-US", "plain prose", Some(&gloss("鸽"))).is_empty());
	}
}
