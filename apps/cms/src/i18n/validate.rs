//! Acceptance checks shared by fresh model replies and translations already on disk.

use super::segment::{Kind, Region, Segment};
use super::store::Sidecar;
use super::width;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
	MalformedTranslatorNote,
	MalformedAuthorNote,
	TranslatorNoteInFrontmatter,
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MalformedTranslatorNote => formatter.write_str("malformed translator's note"),
			Self::MalformedAuthorNote => formatter.write_str("malformed author's note"),
			Self::TranslatorNoteInFrontmatter => {
				formatter.write_str("translator's note is not allowed in frontmatter")
			}
		}
	}
}

/// Whether a note directive is separated from the words around it as that script requires.
///
/// The source can write `请求时的:fn[模型]` because Han script does not space its words. Copied
/// into a language that does, the same shape renders `Modeloen la petición` -- one word that
/// exists in no language, produced without any rule being broken on the way. The reply passed
/// shape, the marker came back, and the sentence is wrong.
///
/// The test is the character, not the locale: a directive needs air when what it touches is a
/// narrow letter or digit. A wide glyph does not space its words, and punctuation -- the `l'` of
/// French elision, a hyphen, an opening `¿` -- is already a boundary the eye reads.
pub fn spacing_intact(text: &str) -> bool {
	let needs_space = |c: char| c.is_alphanumeric() && UnicodeWidthChar::width(c) == Some(1);
	for name in [":fn[", ":tn["] {
		let mut at = 0;
		while let Some(found) = text[at..].find(name) {
			let start = at + found;
			if text[..start].chars().next_back().is_some_and(needs_space) {
				return false;
			}
			// The far side: whatever follows the directive's closing brace.
			let after = &text[start..];
			let close = after.find("\"}").map(|i| start + i + 2);
			if let Some(end) = close {
				if text[end..].chars().next().is_some_and(needs_space) {
					return false;
				}
			}
			at = start + name.len();
		}
	}
	true
}

/// Whether a body section heading still fits the rail it becomes a navigation label in.
///
/// **Only a section is bound by this.** A subsection is not listed in the rail at all, so the
/// rail's width says nothing about it; its length is a question about the prose, answered by the
/// prompt and reported by `audit`, not refused here. See spec/styling.md.
///
/// For a section, only the clamp is refused, not the one-line budget. A heading occupying two
/// lines is a legitimate outcome -- some languages cannot say it shorter, and the rail is built
/// for it -- so rejecting that would buy the same answer again and eventually take a worse one.
/// Past two lines the end is not shown at all, which is a loss rather than a judgement, and a
/// loss is what this boundary is for. Everything between the two is reported by `audit` for a
/// person. See spec/i18n.md.
pub fn heading_fits(kind: Kind, region: Region, level: Option<usize>, text: &str) -> bool {
	kind != Kind::Heading
		|| region != Region::Body
		|| level != Some(2)
		|| width::of(text) <= width::CLAMP
}

/// Whether every `:tn` is one complete `:tn[words]{is="explanation"}` directive.
///
/// An ASCII quote cannot appear inside the explanation because the directive syntax has no
/// escape for it. Empty words and explanations are rejected because neither can communicate a
/// note even though a markdown parser can represent them.
pub fn notes_well_formed(text: &str) -> bool {
	well_formed(text, ":tn")
}

/// The shape both notes share: `:name[words]{is="explanation"}`, neither half empty.
fn well_formed(text: &str, name: &str) -> bool {
	let mut rest = text;
	while let Some(at) = rest.find(name) {
		rest = &rest[at + name.len()..];
		let Some(after_open) = rest.strip_prefix('[') else {
			return false;
		};
		let Some(close) = after_open.find(']') else {
			return false;
		};
		if after_open[..close].trim().is_empty()
			|| after_open[..close].contains('\n')
			|| !after_open[close + 1..].starts_with("{is=\"")
		{
			return false;
		}
		rest = &after_open[close + 6..];
		let Some(end) = rest.find('"') else {
			return false;
		};
		if rest[..end].trim().is_empty() || !rest[end + 1..].starts_with('}') {
			return false;
		}
		rest = &rest[end + 2..];
	}
	true
}

/// Whether every `:fn` is one complete `:fn[words]{is="explanation"}` directive.
///
/// The same shape as a translator's note, and checked for the same reason: an ASCII quote cannot
/// appear inside the explanation because the directive syntax has no escape for one, and a note
/// whose text was eaten by one still parses -- as words with a marker that says nothing.
///
/// The site refuses it when it compiles the source. This is the other end: an author's note
/// travels inside its paragraph to be translated, so a model can hand back a directive the author
/// never wrote.
pub fn author_notes_well_formed(text: &str) -> bool {
	well_formed(text, ":fn")
}

pub fn translation(region: Region, text: &str) -> Result<(), Error> {
	if region == Region::Frontmatter && text.contains(":tn") {
		return Err(Error::TranslatorNoteInFrontmatter);
	}
	if !notes_well_formed(text) {
		return Err(Error::MalformedTranslatorNote);
	}
	if !author_notes_well_formed(text) {
		return Err(Error::MalformedAuthorNote);
	}
	Ok(())
}

/// Validate every live stored translation before CMS emits a build record.
///
/// Orphans cannot be assigned a region after their source segment disappears, so they remain
/// reviewable history and are intentionally outside this build acceptance boundary.
pub fn sidecar(
	path: &Path,
	live: &BTreeMap<String, Segment>,
	sidecar: &Sidecar,
) -> std::io::Result<()> {
	for (id, segment) in live {
		let Some(locales) = sidecar.segments.get(id) else {
			continue;
		};
		for (locale, stored) in locales {
			if let Err(error) = translation(segment.region, &stored.text) {
				return Err(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					format!("{}: segment {id} locale {locale}: {error}", path.display()),
				));
			}
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::i18n::store::Translation;

	#[test]
	fn a_directive_glued_to_a_latin_word_is_refused() {
		// What the model wrote when it copied the source's spacing into Spanish.
		assert!(!spacing_intact(
			r#"## ¿Modelo:fn[en la petición]{is="el modelo de ejecución"}?"#
		));
		assert!(!spacing_intact(
			r#"## Modell zur:fn[Anfragezeit]{is="das Ausführungsmodell"}?"#
		));
	}

	#[test]
	fn a_script_that_does_not_space_its_words_keeps_the_source_shape() {
		assert!(spacing_intact(r#"## 请求时的:fn[模型]{is="指执行模型"}?"#));
		assert!(spacing_intact(
			r#"## リクエスト時:fn[モデル]{is="実行モデル"}?"#
		));
	}

	#[test]
	fn punctuation_is_already_a_boundary() {
		// French elision and a hyphenated term both read correctly without a space.
		assert!(spacing_intact(
			r#"L':fn[AST]{is="the syntax tree"} visible"#
		));
		assert!(spacing_intact(
			r#"pseudo-:fn[SSR]{is="not the traditional kind"} here"#
		));
	}

	#[test]
	fn a_spaced_directive_passes_on_both_sides() {
		assert!(spacing_intact(
			r#"## Request-time :fn[model]{is="the execution model"}?"#
		));
		assert!(spacing_intact(r#"the :fn[model]{is="a gloss"} matters"#));
	}

	#[test]
	fn the_word_after_a_directive_needs_air_too() {
		assert!(!spacing_intact(r#"the :fn[model]{is="a gloss"}matters"#));
	}

	fn stored(text: &str) -> Translation {
		Translation {
			text: text.to_owned(),
			provider: "openai".to_owned(),
			model: "gpt-5-6-luna-medium".to_owned(),
			at: "2026-08-03T00:00:00Z".to_owned(),
			seconds: 1.0,
			tokens: 10,
			review: false,
		}
	}

	#[test]
	fn frontmatter_never_accepts_a_translator_note() {
		assert_eq!(
			translation(Region::Frontmatter, ":tn[translated]{is=\"a gloss\"}"),
			Err(Error::TranslatorNoteInFrontmatter)
		);
		assert!(translation(Region::Body, ":tn[translated]{is=\"a gloss\"}").is_ok());
	}

	#[test]
	fn malformed_note_shapes_are_rejected() {
		assert!(!notes_well_formed(":tn[word]{is=\"missing brace\""));
		assert!(!notes_well_formed(":tn[]{is=\"a gloss\"}"));
		assert!(!notes_well_formed(":tn[word]{is=\"\"}"));
		assert!(!notes_well_formed(":tn{is=\"a gloss\"}"));
	}

	#[test]
	fn malformed_author_note_shapes_are_rejected() {
		assert!(author_notes_well_formed(
			"The :fn[model]{is=\"execution, not data\"} here"
		));
		assert!(author_notes_well_formed("no notes at all"));
		// The shape a straight quote leaves behind: the attribute ends early, the rest is loose.
		assert!(!author_notes_well_formed(
			":fn[said]{is=\"quote \"hi\" here\"}"
		));
		assert!(!author_notes_well_formed(":fn[]{is=\"a note\"}"));
		assert!(!author_notes_well_formed(":fn[word]{is=\"\"}"));
		assert!(!author_notes_well_formed(":fn[word]{is=\"unclosed brace\""));
		// A marker with no words is the shape this replaced, and is refused as incomplete.
		assert!(!author_notes_well_formed(":fn{is=\"a note\"}"));
	}

	#[test]
	fn a_translation_carrying_a_broken_author_note_is_refused() {
		assert_eq!(
			translation(Region::Body, ":fn[model]{is=\"broken}"),
			Err(Error::MalformedAuthorNote)
		);
		assert!(translation(Region::Body, ":fn[model]{is=\"fine\"}").is_ok());
	}

	#[test]
	fn stored_frontmatter_failure_names_the_sidecar_segment_and_locale() {
		let live =
			super::super::segment::translatable("---\ntitle: Source\n---\n\nBody").expect("segments");
		let (id, segment) = live
			.iter()
			.find(|(_, segment)| segment.region == Region::Frontmatter)
			.expect("frontmatter segment");
		let stored_sidecar = Sidecar {
			version: 1,
			segments: BTreeMap::from([(
				id.clone(),
				BTreeMap::from([(
					"en-US".to_owned(),
					stored(":tn[Translated]{is=\"a gloss\"}"),
				)]),
			)]),
		};
		let error = sidecar(
			Path::new("contents/example.i18n.yaml"),
			&live,
			&stored_sidecar,
		)
		.expect_err("invalid content must stop the build record");

		let message = error.to_string();
		assert!(message.contains("contents/example.i18n.yaml"));
		assert!(message.contains(id));
		assert!(message.contains("en-US"));
		assert_eq!(segment.region, Region::Frontmatter);
	}
}
