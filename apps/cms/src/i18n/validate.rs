//! Acceptance checks shared by fresh model replies and translations already on disk.

use super::segment::{Region, Segment};
use super::store::Sidecar;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

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

/// Whether every `:tn` is one complete `:tn[words]{is="explanation"}` directive.
///
/// An ASCII quote cannot appear inside the explanation because the directive syntax has no
/// escape for it. Empty words and explanations are rejected because neither can communicate a
/// note even though a markdown parser can represent them.
pub fn notes_well_formed(text: &str) -> bool {
	let mut rest = text;
	while let Some(at) = rest.find(":tn") {
		rest = &rest[at + 3..];
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

/// Whether every `:fn` is one complete `:fn{is="explanation"}` directive.
///
/// The author's note carries no words of its own -- the marker attaches after what it explains
/// rather than wrapping it -- so its shape is shorter than a translator's note and shares only the
/// quoting rule. An ASCII quote cannot appear inside the explanation because the directive syntax
/// has no escape for one; a note whose text was eaten by one still parses, as a marker that says
/// nothing, which is why this is checked rather than left to render.
///
/// The site refuses the same shape when it compiles the source. This is the other end of that:
/// the note travels inside its paragraph to be translated, so a model can hand back a directive
/// the author never wrote.
pub fn author_notes_well_formed(text: &str) -> bool {
	let mut rest = text;
	while let Some(at) = rest.find(":fn") {
		rest = &rest[at + 3..];
		let Some(after) = rest.strip_prefix("{is=\"") else {
			return false;
		};
		let Some(end) = after.find('"') else {
			return false;
		};
		if after[..end].trim().is_empty() || !after[end + 1..].starts_with('}') {
			return false;
		}
		rest = &after[end + 2..];
	}
	true
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
			"Model:fn{is=\"execution, not data\"} here"
		));
		assert!(author_notes_well_formed("no notes at all"));
		// The shape a straight quote leaves behind: the attribute ends early and the rest is loose.
		assert!(!author_notes_well_formed(":fn{is=\"said \"hi\" then\"}"));
		assert!(!author_notes_well_formed(":fn{is=\"\"}"));
		assert!(!author_notes_well_formed(":fn{is=\"unclosed brace\""));
		// A translator's note is not an author's note, and neither shape accepts the other.
		assert!(!author_notes_well_formed(":fn[words]{is=\"a gloss\"}"));
	}

	#[test]
	fn a_translation_carrying_a_broken_author_note_is_refused() {
		assert_eq!(
			translation(Region::Body, "Model:fn{is=\"broken}"),
			Err(Error::MalformedAuthorNote)
		);
		assert!(translation(Region::Body, "Model:fn{is=\"fine\"}").is_ok());
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
