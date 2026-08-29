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

use super::tn;

/// The locales whose scripts legitimately contain Han characters, so a `:tn` wrapping them
/// says nothing about whether the words were translated.
const HAN_SCRIPT_LOCALES: [&str; 3] = ["zh-CN", "zh-TW", "ja-JP"];

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

fn has_han(text: &str) -> bool {
	text.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// Leading characters that are neither letters nor digits carry no restatement signal.
fn normalised(text: &str) -> String {
	text
		.trim_start_matches(|c: char| !c.is_alphanumeric())
		.to_lowercase()
}

/// The policy findings for one stored translation of one body segment.
pub fn of(
	segment_id: &str,
	locale: &str,
	translation: &str,
	glosses: Option<&tn::Entry>,
) -> Vec<Finding> {
	let mut findings = Vec::new();
	let finding = |reason: String| Finding {
		segment: segment_id.to_owned(),
		locale: locale.to_owned(),
		reason,
	};

	// An author's note explanation continues from its words; opening by restating them is the
	// double reading the policy exists to avoid.
	for (words, note) in directives(translation, ":fn") {
		if !words.is_empty() && normalised(&note).starts_with(&normalised(&words)) {
			findings.push(finding(format!(
				":fn explanation restates the words it follows ({words})"
			)));
		}
	}

	let tn_pairs = directives(translation, ":tn");
	// The wrapped words are the translation. Han inside them, in a locale whose script has
	// none, is the source carried into the sentence -- or a romanisation's sibling failure.
	if !HAN_SCRIPT_LOCALES.contains(&locale) {
		for (words, _) in &tn_pairs {
			if has_han(words) {
				findings.push(finding(format!(
					":tn wraps untranslated source script ({words})"
				)));
			}
		}
	}
	// The note quotes the original span verbatim, in its original script; a reader who has
	// never seen the source meets the original word only here.
	if let Some(entry) = glosses {
		for span in &entry.spans {
			let quoted = tn_pairs.iter().any(|(_, note)| note.contains(&span.phrase));
			if !tn_pairs.is_empty() && !quoted {
				findings.push(finding(format!(
					":tn note does not quote the original ({})",
					span.phrase
				)));
			}
		}
	}

	findings
}

#[cfg(test)]
mod tests {
	use super::*;

	fn gloss(phrase: &str) -> tn::Entry {
		tn::Entry {
			source: String::new(),
			spans: vec![tn::Gloss {
				phrase: phrase.to_owned(),
				guidance: String::new(),
			}],
		}
	}

	#[test]
	fn a_restating_explanation_is_reported_and_a_continuing_one_is_not() {
		let restating = of(
			"s",
			"en-US",
			r#"The :fn[model]{is="model means the runtime"} here"#,
			None,
		);
		assert_eq!(restating.len(), 1);
		assert!(restating[0].reason.contains("restates"));
		let continuing = of(
			"s",
			"en-US",
			r#"The :fn[model]{is="the runtime, not the data"} here"#,
			None,
		);
		assert!(continuing.is_empty());
	}

	#[test]
	fn restatement_ignores_case_and_leading_punctuation() {
		let found = of(
			"s",
			"en-US",
			r#":fn[Seam]{is="-- seam is a protocol"} x"#,
			None,
		);
		assert_eq!(found.len(), 1);
	}

	#[test]
	fn han_inside_a_latin_locales_tn_is_reported() {
		let text = r#"he :tn[鸽了]{is="a note"} it"#;
		assert_eq!(of("s", "en-US", text, None).len(), 1);
		// The same words in a Han-script locale say nothing.
		assert!(of("s", "zh-TW", text, None).is_empty());
	}

	#[test]
	fn a_note_that_skips_the_original_is_reported_and_a_quoting_one_is_not() {
		let glosses = gloss("鸽");
		let skipping = r#"he :tn[put it off]{is="the source said postponing, like a no-show"} it"#;
		assert_eq!(of("s", "en-US", skipping, Some(&glosses)).len(), 1);
		let quoting = r#"he :tn[put it off]{is="the source word 鸽, literally pigeon, means standing someone up"} it"#;
		assert!(of("s", "en-US", quoting, Some(&glosses)).is_empty());
	}

	#[test]
	fn a_translation_with_no_directives_has_no_findings() {
		assert!(of("s", "en-US", "plain prose", None).is_empty());
		// A recorded gloss without any tn in the text is missing work, not a policy finding --
		// the completeness audit owns that.
		assert!(of("s", "en-US", "plain prose", Some(&gloss("鸽"))).is_empty());
	}
}
