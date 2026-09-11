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

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Columns the rail fits on one line: its cap is 192px at 13px type, so fourteen Han characters
/// or twenty-eight Latin ones. A number about the layout, kept beside the rule that reads it.
pub const ONE_LINE: usize = 28;

/// Two lines, where the label is clamped. Past this the end of the heading is not shown at all,
/// which is the one outcome that is a loss rather than a judgement -- see `validate`.
pub const CLAMP: usize = ONE_LINE * 2;

/// What a string draws to, in pixels, at the 16px type the card list and the article title use.
///
/// Columns are the right unit for the table of contents, where the type is one size and the rail
/// one width. They are the wrong unit here, because these budgets come from three different places
/// in the layout and a column is not the same number of pixels in every script. Measured in the
/// rendered page at 16px: a Han character or a kana draws exactly 16.00px, a Hangul syllable
/// 13.84, full-width punctuation 12.57, and a Latin character averages 7.72 over the real corpus
/// with the widest single string reaching 8.55.
///
/// Han and kana draw a full square and Hangul draws 13.84 of one, so those two are constants.
/// Latin is a table, because a blunt average there is not a small error: `i` advances 3.88px and
/// `W` 16.14, and a single figure chosen safely above both charges an ordinary sentence about a
/// tenth more than it draws. That tenth is not free -- it is a tenth of every budget, taken from
/// the copy -- and it is what left Spanish with three pixels of room on a line that needed four.
///
/// The advances are measured, not looked up in the font file: each character is drawn twenty
/// times on a canvas at the size and weight a card title uses -- the heavier of the two faces
/// these budgets cover -- and the run divided by twenty, which is the
/// shaped advance rather than the nominal one. Against whole strings the sum lands within 3px of
/// what the browser renders, and always above it -- kerning only ever brings real text in
/// narrower than the sum of its advances, so the estimate errs the one way it may.
pub const PX_WIDE: f32 = 16.0;
pub const PX_HANGUL: f32 = 14.0;
/// A character outside the table, which for the nine locales here means none of them.
pub const PX_UNKNOWN: f32 = 10.0;

/// The advance of one Latin character at 16px, measured in the rendered page.
fn latin(c: char) -> f32 {
	match c {
		'W' => 16.4,
		'%' => 15.89,
		'@' => 15.72,
		'M' => 14.6,
		'…' => 14.56,
		'm' => 14.21,
		'w' => 13.26,
		'Q' => 12.3,
		'O' | 'Ó' => 12.27,
		'N' | 'Ñ' => 12.1,
		'G' => 11.96,
		'H' => 11.91,
		'U' | 'Ú' | 'Ü' => 11.84,
		'C' | 'Ç' => 11.74,
		'A' | 'V' | 'À' | 'Á' => 11.56,
		'D' => 11.55,
		'X' => 11.21,
		'Y' => 11.14,
		'K' => 11.0,
		'+' | '<' | '=' | '>' | '~' => 10.68,
		'B' => 10.51,
		'4' => 10.5,
		'&' | 'T' => 10.45,
		'R' => 10.37,
		'$' | 'S' => 10.34,
		'0' => 10.32,
		'P' => 10.27,
		'Z' => 10.25,
		'#' => 10.22,
		'6' | '9' | 'ß' => 10.08,
		'8' => 10.07,
		'3' => 10.03,
		'g' => 9.91,
		'b' | 'd' | 'p' | 'q' => 9.89,
		'2' => 9.86,
		'o' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 9.67,
		'5' | 'E' | 'È' | 'É' | 'Ê' => 9.65,
		'h' | 'u' | 'ù' | 'ú' | 'û' | 'ü' => 9.63,
		'n' | 'ñ' => 9.62,
		'7' | 'F' => 9.43,
		'e' | 'è' | 'é' | 'ê' | 'ë' => 9.4,
		'c' | 'ç' => 9.23,
		'J' | 'y' => 9.2,
		'v' => 9.19,
		'a' | 'à' | 'á' | 'â' | 'ã' | 'ä' => 9.09,
		'L' => 9.05,
		'k' => 8.95,
		'z' => 8.94,
		'x' => 8.92,
		's' => 8.62,
		'?' => 8.44,
		'*' => 8.32,
		'"' => 7.91,
		'^' => 7.62,
		'“' => 7.58,
		'”' => 7.53,
		'-' | '_' => 7.4,
		'{' | '}' => 7.05,
		'1' => 6.64,
		'ï' => 6.51,
		'r' => 6.38,
		'/' => 5.91,
		'(' | ')' | '[' | ']' => 5.9,
		'|' => 5.53,
		'f' => 5.46,
		'`' | 't' => 5.39,
		'\\' => 5.17,
		';' => 5.05,
		'\'' => 5.0,
		'!' => 4.87,
		',' | '.' | ':' | '·' => 4.85,
		'‘' | '’' => 4.44,
		'I' | 'Í' => 4.36,
		' ' => 4.26,
		'i' | 'j' | 'l' | 'ì' | 'í' | 'î' => 4.03,
		_ => PX_UNKNOWN,
	}
}

/// Hangul syllables, plus the jamo blocks a decomposed syllable is written with.
fn is_hangul(c: char) -> bool {
	matches!(c, '\u{AC00}'..='\u{D7A3}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{A960}'..='\u{A97F}' | '\u{D7B0}'..='\u{D7FF}')
}

pub fn pixels(text: &str) -> f32 {
	text.trim()
		.chars()
		.map(|c| {
			if is_hangul(c) {
				PX_HANGUL
			} else if UnicodeWidthChar::width(c) == Some(2) {
				PX_WIDE
			} else {
				latin(c)
			}
		})
		.sum()
}

/// Characters of `locale`'s own script that fit in a budget.
///
/// The prompt speaks to the model in characters, because that is the unit it can count as it
/// writes; the check below speaks in pixels, because that is what the layout is. The two are the
/// same rule read from either end, and the character figure is deliberately the stricter of them:
/// a model told the exact limit writes to it and lands on the boundary, where one wide letter
/// decides the outcome.
pub fn characters(budget: f32, locale: &str) -> usize {
	// The average of a real sentence rather than of the alphabet: measured over the corpus, Latin
	// prose runs 7.7px a character and the figure is rounded up so the count it yields is one a
	// writer can actually land.
	const PX_LATIN_PROSE: f32 = 8.0;
	let per = match locale.split('-').next().unwrap_or(locale) {
		"zh" | "ja" => PX_WIDE,
		"ko" => PX_HANGUL,
		_ => PX_LATIN_PROSE,
	};
	(budget / per).floor() as usize
}

/// The heading level of a source block, or `None` if it is not a heading.
///
/// Only a level-2 heading is listed in the rail, so only it is bound by the rail's width -- see
/// spec/styling.md. A subsection is reached by arriving at its parent, and how long its own
/// heading runs is a question about the prose, not about a column 8.5rem wide.
pub fn level(source: &str) -> Option<usize> {
	let marks = source.trim_start().chars().take_while(|c| *c == '#').count();
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

/// Where a title and a subtitle are drawn, and how much room each has.
///
/// Measured in Safari on an iPhone 17 Pro simulator and an iPad mini at the article column's cap,
/// not in an emulated viewport -- the two engines agree on geometry but disagree by 12% on
/// Japanese, which falls to a different font in each. See spec/i18n.md.
///
/// The phone's card row spends 107px before any text: 48 of page padding, a 47px thumbnail and
/// the 12px beside it. The title gives up 12 more for the leader's clearance and 97 for the date,
/// which is the same English short form in every locale. The article page spends only the 48 and
/// gives its title 85% of the 354px that leaves.
pub mod budget {
	/// A card title on a phone, where the row clips with an ellipsis.
	pub const PHONE_TITLE: f32 = 186.0;
	/// A card subtitle on a phone, which has the column to itself.
	pub const PHONE_SUBTITLE: f32 = 295.0;
	/// An article title on a phone, which is a different question from the card title above.
	///
	/// A card title shares its row with a dotted leader and a date; an article title has the whole
	/// column. So the figure is not taken from what is left over but from what the column is: 354px
	/// inside the page padding on an iPhone 17 Pro, of which the title may take 85%.
	///
	/// Not a clip but a decision. A title wider than this is replaced by the short one, which
	/// always fits, rather than being allowed to wrap to a second line.
	pub const ARTICLE_TITLE: f32 = 300.0;
	/// A card title once the article column is at its 45rem cap, which is every window past 768px.
	pub const DESKTOP_TITLE: f32 = 504.0;
	/// A card subtitle at that same cap.
	pub const DESKTOP_SUBTITLE: f32 = 613.0;
}

/// The share of a budget a string may take and still be left alone.
///
/// A translation that lands inside its budget with nothing to spare is one edit away from not
/// fitting, and the longest title in the corpus today draws 503px against a 504px cap. So a fifth
/// is held back: what fits comfortably is kept, and what merely fits is written again.
pub const HEADROOM: f32 = 0.8;

pub fn fits(text: &str, budget: f32) -> bool {
	pixels(text) <= budget
}

pub fn comfortable(text: &str, budget: f32) -> bool {
	pixels(text) <= budget * HEADROOM
}

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

	/// Measured in the rendered page; the estimate must never come in under these.
	const MEASURED: [(&str, f32); 10] = [
		("朋友只是同行一程", 128.0),
		("将渲染视为协议", 112.0),
		("プロトコルとしての描画", 176.0),
		("친구는 한 시절의 인연", 137.0),
		("Freunde auf Zeit", 125.0),
		("Se acabó holgazanear", 170.0),
		("Cargo stops slacking", 160.0),
		("El renderizado como protocolo", 234.0),
		("Le rendu, un protocole", 172.0),
		("Friends are only there for a season", 265.0),
	];

	#[test]
	fn the_estimate_never_comes_in_under_the_rendered_width() {
		for (text, measured) in MEASURED {
			assert!(
				pixels(text) >= measured,
				"{text}: estimated {} under the measured {measured}",
				pixels(text)
			);
		}
	}

	#[test]
	fn the_estimate_stays_within_a_fiftieth_of_the_rendered_width() {
		// A per-character table rather than an average, so the tolerance is what kerning and
		// rounding leave rather than what a blunt constant needed. It was a sixth.
		for (text, measured) in MEASURED {
			let over = (pixels(text) - measured) / measured;
			assert!(over <= 0.02, "{text}: estimated {over:.3} over the measured {measured}");
		}
	}

	#[test]
	fn a_han_character_is_charged_the_full_square_it_draws() {
		assert_eq!(pixels("朋友只是同行一程"), PX_WIDE * 8.0);
		assert_eq!(pixels("プロトコル"), PX_WIDE * 5.0);
	}

	#[test]
	fn hangul_is_charged_less_than_its_width_class_would_say() {
		// Two columns each by East Asian Width, but 13.84px on the page. Charging the full square
		// would take a fifth of Korean's budget away for nothing.
		assert!(pixels("인연") < PX_WIDE * 2.0);
		assert_eq!(pixels("인연"), PX_HANGUL * 2.0);
	}

	#[test]
	fn an_article_title_has_more_room_than_a_card_title() {
		// The card shares its row with a leader and a date; the article title has the column.
		assert!(budget::ARTICLE_TITLE > budget::PHONE_TITLE);
		// So a short title, written to the card, always clears the article page.
		assert!(fits("Freunde auf Zeit", budget::ARTICLE_TITLE));
		// And the longest title in the corpus does not.
		assert!(!fits("Freundschaften gehören immer nur zu bestimmten Lebensphasen", budget::ARTICLE_TITLE));
	}

	#[test]
	fn the_character_budget_follows_the_script_the_locale_writes_in() {
		assert_eq!(characters(budget::PHONE_TITLE, "zh-CN"), 11);
		assert_eq!(characters(budget::PHONE_TITLE, "ja-JP"), 11);
		assert_eq!(characters(budget::PHONE_TITLE, "ko-KR"), 13);
		assert_eq!(characters(budget::PHONE_TITLE, "de-DE"), 23);
	}

	#[test]
	fn a_title_that_merely_fits_is_not_comfortable() {
		// The German title in the corpus today, against the desktop cap it is one pixel under.
		let tight = "Freundschaften gehören immer nur zu bestimmten Lebensphasen";
		assert!(!comfortable(tight, budget::DESKTOP_TITLE));
		assert!(comfortable("Rendering as a Protocol", budget::DESKTOP_TITLE));
	}

	#[test]
	fn the_short_versions_written_by_hand_fit_the_phone() {
		for (text, _) in MEASURED.iter().take(7) {
			assert!(fits(text, budget::PHONE_TITLE), "{text} does not fit the phone title budget");
		}
	}
}
