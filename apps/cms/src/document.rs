//! An article file split into the frontmatter block and the prose after it.
//!
//! One reading, because there were five. Every consumer wrote its own
//! `strip_prefix("---\n")` and they disagreed about the same question -- what a file with no
//! frontmatter, or with a fence it never closes, actually is. Three answers were in the tree at
//! once: the whole text is body, there is no article here, and the body starts after the opening
//! fence. None of them was wrong; nothing said which was the rule.
//!
//! Byte offsets are part of what this returns, not an extra. Translations are spliced back into
//! the file by range, so a consumer that knows where the frontmatter *is* cannot be served by one
//! that only knows what it says. See spec/i18n.md.

use std::collections::BTreeMap;

/// A document whose frontmatter cannot be read.
///
/// Returned rather than panicked on: a person writes this by hand, so a stray colon is an
/// ordinary event and not a broken invariant. The article's path is deliberately absent -- this
/// is handed text and does not know one, while every caller read a file and does, so the path is
/// attached where it is already known rather than threaded through as a parameter the work never
/// uses. See spec/code.md.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Malformed {
	#[error("frontmatter opens with `---` and never closes")]
	Unterminated,
	#[error("frontmatter is not valid YAML: {0}")]
	NotYaml(String),
	#[error("frontmatter is not a mapping of keys to values")]
	NotAMapping,
	#[error("frontmatter `{0}` must be text")]
	NotText(String),
}

impl From<Malformed> for std::io::Error {
	fn from(error: Malformed) -> Self {
		std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
	}
}

/// The text fields a frontmatter block declares.
pub type Fields = BTreeMap<String, String>;

/// The two halves of an article file, and where each begins in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Document<'a> {
	/// The YAML between the fences, absent when the file opens with none.
	pub frontmatter: Option<&'a str>,
	/// Where that YAML begins. Meaningless when there is none, and zero then.
	pub frontmatter_start: usize,
	pub body: &'a str,
	pub body_start: usize,
}

/// Split a file into its frontmatter and its prose.
///
/// **A file with no frontmatter is a document whose body is all of it.** That is the reading the
/// character counters already used, and the one that lets a page without metadata still be read
/// as prose.
///
/// **A fence that never closes is an error, not a document without frontmatter.** The two are
/// indistinguishable by shape and opposite in meaning: one is a file that declares nothing, the
/// other is a file whose declarations were swallowed. Reading the second as the first is how an
/// article silently loses its title, which is the failure this module exists to stop. Nothing in
/// `contents/` relies on the lenient reading -- checked when this was written.
pub fn split(text: &str) -> Result<Document<'_>, Malformed> {
	let Some(rest) = text.strip_prefix("---\n") else {
		return Ok(Document { frontmatter: None, frontmatter_start: 0, body: text, body_start: 0 });
	};
	let Some(end) = rest.find("\n---") else {
		return Err(Malformed::Unterminated);
	};

	// `---\n` is four bytes, and the closing `\n---` another four.
	Ok(Document {
		frontmatter: Some(&rest[..end]),
		frontmatter_start: 4,
		body: &rest[end + 4..],
		body_start: end + 8,
	})
}

/// Every text field the frontmatter declares, keyed by name.
///
/// Read once for the whole block rather than once per key. The version this replaced parsed the
/// YAML again for every field asked for, so reading an article's `lang`, `title` and `subtitle`
/// parsed the same six lines three times -- and swallowed a parse failure into "no such field",
/// which made a broken article indistinguishable from a page that declares nothing. `cms
/// articles` skipped those silently.
///
/// Non-text values are dropped rather than refused. A frontmatter key holding a list or a date is
/// legitimate and simply not something a caller asking for text wants; the one place that must
/// insist on text is the translator, which says so itself.
pub fn fields(text: &str) -> Result<Fields, Malformed> {
	let Some(frontmatter) = split(text)?.frontmatter else {
		return Ok(Fields::new());
	};
	let values: serde_yaml_ng::Value =
		serde_yaml_ng::from_str(frontmatter).map_err(|error| Malformed::NotYaml(error.to_string()))?;
	let Some(values) = values.as_mapping() else {
		return Err(Malformed::NotAMapping);
	};
	Ok(
		values
			.iter()
			.filter_map(|(key, value)| Some((key.as_str()?.to_owned(), value.as_str()?.to_owned())))
			.collect(),
	)
}

/// `fields`, with the article named in whatever goes wrong.
///
/// The error type carries no path on purpose, and every caller that reads a file has one, so this
/// is the join between them -- written once because otherwise it is written at each of the six
/// places that read an article, and one of them would word it differently.
pub fn fields_of(text: &str, path: &std::path::Path) -> std::io::Result<Fields> {
	fields(text).map_err(|error| {
		std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {error}", path.display()))
	})
}

impl<'a> Document<'a> {
	/// The prose with the blank lines after the closing fence dropped.
	///
	/// What a model is shown, and what a character count measures: the gap between the fence and
	/// the first paragraph is layout rather than content.
	///
	/// Borrowed from the original text rather than from `self`, so a caller may drop the document
	/// and keep the prose.
	pub fn prose(&self) -> &'a str {
		self.body.trim_start_matches(['\n', '\r'])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_file_without_frontmatter_is_all_body() {
		let document = split("Just prose.\n").expect("split");
		assert_eq!(document.frontmatter, None);
		assert_eq!(document.body, "Just prose.\n");
		assert_eq!(document.body_start, 0);
	}

	#[test]
	fn the_offsets_locate_both_halves_in_the_original() {
		let text = "---\ntitle: A\n---\n\nBody.\n";
		let document = split(text).expect("split");
		assert_eq!(document.frontmatter, Some("title: A"));
		assert_eq!(&text[document.frontmatter_start..document.frontmatter_start + 8], "title: A");
		assert_eq!(&text[document.body_start..], document.body);
	}

	/// The disagreement this module was written to settle. Read leniently, a file whose fence
	/// never closes looks like a file with no metadata, and every title in it disappears without
	/// a word.
	#[test]
	fn a_fence_that_never_closes_is_refused_rather_than_read_as_empty() {
		let error = split("---\ntitle: A\n\nBody with no closing fence.\n").expect_err("refused");
		assert_eq!(error, Malformed::Unterminated);
	}

	#[test]
	fn prose_drops_the_blank_lines_after_the_fence() {
		let document = split("---\ntitle: A\n---\n\n\nBody.\n").expect("split");
		assert_eq!(document.prose(), "Body.\n");
	}
}
