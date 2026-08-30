//! How wide a heading will draw where it is read as navigation.
//!
//! A heading is also a label in the article's table of contents, and that rail is narrow. What
//! decides whether it fits is not how many characters it has: a Han character occupies two
//! columns where a Latin letter occupies one, so a count says a Chinese heading and a French one
//! of the same length are the same size, and they differ by a factor of two. Measured against the
//! rendered rail, one CJK glyph is 13px and the average Latin character 6.84px -- a ratio of 1.9,
//! which is the East Asian Width table saying the same thing.
//!
//! This is the cheap true measure rather than the exact one. Real text shaping knows that `i` is
//! narrower than `m`; it also needs the font, which the CMS does not have and should not grow a
//! reason to load. The two-to-one split is where nearly all the error is, and it is the half a
//! table can settle. See spec/i18n.md.

use unicode_width::UnicodeWidthStr;

/// Columns the rail fits on one line: its cap is 192px at 13px type, so fourteen Han characters
/// or twenty-eight Latin ones. A number about the layout, kept beside the rule that reads it.
pub const ONE_LINE: usize = 28;

/// Two lines, where the label is clamped. Past this the end of the heading is not shown at all,
/// which is the one outcome that is a loss rather than a judgement -- see `validate`.
pub const CLAMP: usize = ONE_LINE * 2;

/// The heading level of a source block, or `None` if it is not a heading.
///
/// Only a level-2 heading is listed in the rail, so only it is bound by the rail's width -- see
/// spec/styling.md. A subsection is reached by arriving at its parent, and how long its own
/// heading runs is a question about the prose, not about a column 8.5rem wide.
pub fn level(source: &str) -> Option<usize> {
	let marks = source
		.trim_start()
		.chars()
		.take_while(|c| *c == '#')
		.count();
	(2..=6).contains(&marks).then_some(marks)
}

/// What the table of contents will actually print for this heading.
///
/// The stored segment is markdown: the `##` marks, an explicit `{#slug}` anchor, and any note
/// directive around some of the words. None of those reach the rail -- the anchor is an address,
/// and a note's marker is dropped when the heading is flattened to a label -- so measuring the
/// raw segment would charge a heading for text nobody sees.
pub fn label(source: &str) -> String {
	let mut text = source.trim();
	// Heading marks, then the trailing anchor.
	text = text.trim_start_matches('#').trim_start();
	if let Some(open) = text.rfind("{#") {
		if text.trim_end().ends_with('}') {
			text = text[..open].trim_end();
		}
	}

	let mut out = String::with_capacity(text.len());
	let mut rest = text;
	while let Some(at) = rest.find(":fn[").or_else(|| rest.find(":tn[")) {
		out.push_str(&rest[..at]);
		let after = &rest[at + 4..];
		let Some(close) = after.find(']') else {
			break;
		};
		// The wrapped words are what the label shows; the explanation never appears there.
		out.push_str(&after[..close]);
		let tail = &after[close + 1..];
		rest = match tail.strip_prefix("{is=\"") {
			Some(note) => match note.find("\"}") {
				Some(end) => &note[end + 2..],
				None => "",
			},
			None => tail,
		};
	}
	out.push_str(rest);
	out.replace(['`', '*', '_'], "")
}

/// Columns `source` occupies once it is a navigation label.
pub fn of(source: &str) -> usize {
	UnicodeWidthStr::width(label(source).trim())
}

/// Columns a block occupies as written, markdown and all.
///
/// Unlike `of`, nothing is stripped: this is used to compare a translation against its source,
/// where the marks are part of what was asked for and part of what came back.
pub fn raw(text: &str) -> usize {
	UnicodeWidthStr::width(text.trim())
}

/// How much wider than its source a translation may be before it is not a translation.
///
/// A block cannot say several times more than the block it renders, so a reply that does is
/// answering something else -- in practice the neighbouring context, which the request carries
/// and the prompt forbids repeating. The failure is invisible to every other check: the markers
/// are intact because a short source has none, the line count matches because both are one line,
/// and the shape is valid. Only the size gives it away.
///
/// Four times plus forty columns. Measured over 2744 stored translations, the widest legitimate
/// one runs 2.4 times its source, and short blocks need the constant -- `OR` is two columns and
/// its German is four. Nine entries exceeded it, and all nine were the fault this describes:
/// a two-column `OR` answered with five hundred, a horizontal rule with three hundred.
pub const SIZE_FACTOR: usize = 4;
pub const SIZE_ALLOWANCE: usize = 40;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_han_character_is_two_columns_and_a_latin_one_is_one() {
		assert_eq!(of("## 编译期渲染"), 10);
		assert_eq!(of("## Compile time"), 12);
	}

	#[test]
	fn the_anchor_is_an_address_and_is_not_measured() {
		assert_eq!(of("## Adoption {#adoption}"), of("## Adoption"));
	}

	#[test]
	fn a_note_is_measured_by_its_words_alone() {
		assert_eq!(
			of(r#"## Request-time :fn[model]{is="the execution model, not the data model"}"#),
			of("## Request-time model"),
		);
	}

	#[test]
	fn emphasis_and_code_marks_are_not_drawn() {
		assert_eq!(of("## The `slot` protocol"), of("## The slot protocol"));
	}

	#[test]
	fn a_level_is_read_from_the_marks_and_nothing_else_is_a_heading() {
		assert_eq!(level("## Adoption {#adoption}"), Some(2));
		assert_eq!(level("### Local ownership {#local}"), Some(3));
		assert_eq!(level("A paragraph about ## things"), None);
	}

	#[test]
	fn a_heading_that_wraps_is_over_one_line() {
		assert!(of("## Le modèle au moment de la requête ?") > ONE_LINE);
		assert!(of("## Adoption") < ONE_LINE);
	}
}
