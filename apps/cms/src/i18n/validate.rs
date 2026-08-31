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
	AuthorNoteCountChanged,
	UnresolvedMarker,
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::MalformedTranslatorNote => formatter.write_str("malformed translator's note"),
			Self::MalformedAuthorNote => formatter.write_str("malformed author's note"),
			Self::TranslatorNoteInFrontmatter => {
				formatter.write_str("translator's note is not allowed in frontmatter")
			}
			Self::AuthorNoteCountChanged => formatter.write_str(
				"the translation does not carry the author's notes its source does -- it is \
				 probably a translation of the neighbouring block",
			),
			Self::UnresolvedMarker => formatter.write_str(
				"the translation carries a marker that stands for nothing -- text copied from \
				 the neighbouring context",
			),
		}
	}
}

/// Whether a translation is a plausible size for the block it translates.
///
/// See `width::SIZE_FACTOR` for the numbers and what they are drawn from. This is deliberately
/// generous: it is not a style rule about length, it is the last check that catches a reply which
/// translated the wrong block -- the neighbouring paragraph carried in the request as context.
/// Nothing else sees that. A short source has no code markers to lose, one line against one line
/// passes the line count, and the shape is perfectly valid; the answer is simply about something
/// else, and its size is the only trace.
pub fn size_plausible(source: &str, text: &str) -> bool {
	width::raw(text) <= width::raw(source) * width::SIZE_FACTOR + width::SIZE_ALLOWANCE
}

/// Whether the translation carries exactly the author's notes its source does.
///
/// `:fn` is the author's, never the translator's -- a model has `:tn` for its own remarks -- so
/// the count is fixed by the source and is not a matter of judgement. That makes it the one
/// cheap invariant that catches a reply about a different block: measured over 2744 stored
/// translations it never fired on a correct one, and it named every locale of the block whose
/// answer was its neighbour's. Size could not: the neighbour was twice the source, and twice is
/// what an ordinary German paragraph does. See spec/i18n.md.
pub fn author_notes_preserved(source: &str, text: &str) -> bool {
	source.matches(":fn[").count() == text.matches(":fn[").count()
}

/// Whether the translation is free of markers that stand for something it cannot have.
///
/// Every `⟦tk:N⟧` in the block has been put back by `restore` before this runs, so a surviving
/// `⟦` came from somewhere else -- and the only other place one exists is the neighbouring
/// context, where inline code is folded to a placeholder that deliberately restores to nothing.
/// Text copied out of the context therefore arrives holding proof of where it came from.
pub fn markers_resolved(text: &str) -> bool {
	!text.contains(super::segment::OPEN)
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

/// The checks that read a translation against its source, whether or not code is masked in it.
///
/// **`markers_resolved` is deliberately not among them.** A fresh reply is validated before its
/// markers are put back, so at that point it is *supposed* to be full of `⟦tk:N⟧`; running the
/// check here refused every block containing inline code, which cost a whole article's worth of
/// paid requests before the message -- the generic `no locale survived` -- gave any hint why.
/// The marker check belongs where the text is final: after `restore` in the reply path, and on
/// stored text in `sidecar` below.
pub fn translation(region: Region, source: &str, text: &str) -> Result<(), Error> {
	if region == Region::Frontmatter && text.contains(":tn") {
		return Err(Error::TranslatorNoteInFrontmatter);
	}
	if !notes_well_formed(text) {
		return Err(Error::MalformedTranslatorNote);
	}
	if !author_notes_well_formed(text) {
		return Err(Error::MalformedAuthorNote);
	}
	if !author_notes_preserved(source, text) {
		return Err(Error::AuthorNoteCountChanged);
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
			// Stored text is final -- its markers were restored before it was written -- so the
			// marker check applies here, where it cannot apply to a reply still holding them.
			let checked = translation(segment.region, &segment.source, &stored.text).and_then(|()| {
				if markers_resolved(&stored.text) { Ok(()) } else { Err(Error::UnresolvedMarker) }
			});
			if let Err(error) = checked {
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
		assert!(!spacing_intact(r#"## ¿Modelo:fn[en la petición]{is="el modelo de ejecución"}?"#));
		assert!(!spacing_intact(r#"## Modell zur:fn[Anfragezeit]{is="das Ausführungsmodell"}?"#));
	}

	#[test]
	fn a_script_that_does_not_space_its_words_keeps_the_source_shape() {
		assert!(spacing_intact(r#"## 请求时的:fn[模型]{is="指执行模型"}?"#));
		assert!(spacing_intact(r#"## リクエスト時:fn[モデル]{is="実行モデル"}?"#));
	}

	#[test]
	fn punctuation_is_already_a_boundary() {
		// French elision and a hyphenated term both read correctly without a space.
		assert!(spacing_intact(r#"L':fn[AST]{is="the syntax tree"} visible"#));
		assert!(spacing_intact(r#"pseudo-:fn[SSR]{is="not the traditional kind"} here"#));
	}

	#[test]
	fn a_spaced_directive_passes_on_both_sides() {
		assert!(spacing_intact(r#"## Request-time :fn[model]{is="the execution model"}?"#));
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
			translation(Region::Frontmatter, "source", ":tn[translated]{is=\"a gloss\"}"),
			Err(Error::TranslatorNoteInFrontmatter)
		);
		assert!(translation(Region::Body, "source", ":tn[translated]{is=\"a gloss\"}").is_ok());
	}

	#[test]
	fn a_reply_that_is_the_neighbouring_block_is_refused_by_its_author_notes() {
		// The measured case: three locales of a block with no author's note came back holding the
		// previous block's translation, which has one. Every other check passed -- the shape was
		// valid, the line count matched, and at twice the source's width it was an ordinary
		// German-shaped expansion.
		let source = "只可惜之前我 UI 选了 React 开始动刀";
		let neighbour =
			"就像上一篇写过的，Seam 首先是一套协议，`if` 也是:fn[协议节点]{is=\"结构单元\"}";
		assert!(!author_notes_preserved(source, neighbour));
		assert_eq!(translation(Region::Body, source, neighbour), Err(Error::AuthorNoteCountChanged));
		// A real translation of the same source keeps the count at zero and passes.
		assert!(author_notes_preserved(
			source,
			"The pity is that I had already picked React for the UI"
		));
		// And a block that does carry a note keeps exactly that many.
		let noted = "请求时的:fn[模型]{is=\"执行模型\"}";
		assert!(author_notes_preserved(
			noted,
			"the :fn[model]{is=\"the execution one\"} at request time"
		));
	}

	#[test]
	fn a_reply_still_holding_its_markers_is_not_refused_for_holding_them() {
		// The reply path validates before `restore`, so a block with inline code arrives full of
		// `⟦tk:N⟧` and that is correct. Running the marker check here refused every such block --
		// two thirds of an article -- and reported it as the generic shape failure.
		let masked = crate::i18n::segment::mask("run `cargo build` twice");
		assert!(masked.text.contains('⟦'));
		assert!(translation(Region::Body, &masked.text, &masked.text).is_ok());
	}

	#[test]
	fn a_marker_that_stands_for_nothing_is_refused() {
		// Inline code in the context is folded to a placeholder that restores to nothing, so text
		// copied out of the context arrives still carrying it.
		assert!(!markers_resolved("the ⟦code⟧ was copied from the context"));
		assert!(markers_resolved("ordinary prose with `code` restored into it"));
		// Refused by the caller that owns the final text, not by `translation`.
		assert!(translation(Region::Body, "source", "carries a ⟦code⟧ marker").is_ok());
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
		assert!(author_notes_well_formed("The :fn[model]{is=\"execution, not data\"} here"));
		assert!(author_notes_well_formed("no notes at all"));
		// The shape a straight quote leaves behind: the attribute ends early, the rest is loose.
		assert!(!author_notes_well_formed(":fn[said]{is=\"quote \"hi\" here\"}"));
		assert!(!author_notes_well_formed(":fn[]{is=\"a note\"}"));
		assert!(!author_notes_well_formed(":fn[word]{is=\"\"}"));
		assert!(!author_notes_well_formed(":fn[word]{is=\"unclosed brace\""));
		// A marker with no words is the shape this replaced, and is refused as incomplete.
		assert!(!author_notes_well_formed(":fn{is=\"a note\"}"));
	}

	#[test]
	fn a_translation_carrying_a_broken_author_note_is_refused() {
		assert_eq!(
			translation(Region::Body, ":fn[model]{is=\"source\"}", ":fn[model]{is=\"broken}"),
			Err(Error::MalformedAuthorNote)
		);
		assert!(
			translation(Region::Body, ":fn[model]{is=\"source\"}", ":fn[model]{is=\"fine\"}").is_ok()
		);
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
				BTreeMap::from([("en-US".to_owned(), stored(":tn[Translated]{is=\"a gloss\"}"))]),
			)]),
		};
		let error = sidecar(Path::new("contents/example.i18n.yaml"), &live, &stored_sidecar)
			.expect_err("invalid content must stop the build record");

		let message = error.to_string();
		assert!(message.contains("contents/example.i18n.yaml"));
		assert!(message.contains(id));
		assert!(message.contains("en-US"));
		assert_eq!(segment.region, Region::Frontmatter);
	}
}
