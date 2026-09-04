//! One article's segments, for an interface that wants to look inside it.
//!
//! The listing answers what an article owes; this answers what it is made of. Two reads rather
//! than one because they are asked at different moments and cost different amounts: the listing
//! is drawn for every article at once, and this is drawn for the one somebody opened.
//!
//! ## Two shapes, because a stale segment has no source
//!
//! A live segment can be shown by its own prose -- the paragraph is in the article, and the
//! translations are what it became. A stale one cannot: its paragraph was edited away, so the only
//! text that still exists anywhere is the translation itself. Nothing here can put the two side by
//! side, and an interface that offered a before-and-after would be inventing the before. So a
//! stale segment carries a preview of what it says and a live one carries its source.
//!
//! ## Bodies are not sent until they are asked for
//!
//! The largest sidecar here is 609 KB across 1128 translations. Sending all of it to draw a list
//! of a hundred and forty rows spends the whole file to render the first line of each. [outline]
//! carries what a row needs and [detail] carries one segment's translations, which is the
//! granularity the interface opens at anyway.

use serde::Serialize;
use std::path::Path;

use crate::i18n;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Row {
	pub id: String,
	/// Whether the article has since dropped the paragraph this belongs to.
	pub stale: bool,
	/// The paragraph, for a live segment. Absent for a stale one, which no longer has one.
	pub source: Option<String>,
	/// What a stale segment says, in the source locale where there is one. Absent for a live
	/// segment, whose `source` is the better line to show.
	pub preview: Option<String>,
	/// Which locales hold a translation, in the order the interface shows them.
	pub locales: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Outline {
	pub article: String,
	/// Live segments in the order they appear in the article, then the stale ones.
	pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rendering {
	pub locale: String,
	pub text: String,
	pub provider: String,
	pub model: String,
	/// ISO 8601 UTC, when the request was sent.
	pub at: String,
	pub tokens: u64,
	pub review: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
	pub id: String,
	pub stale: bool,
	pub source: Option<String>,
	pub renderings: Vec<Rendering>,
}

/// Cut a preview to something a row can hold without the caller measuring bytes.
///
/// Characters rather than bytes, so a Chinese translation is not cut mid-codepoint -- every
/// article here has at least one locale where that is the normal case.
fn preview_of(text: &str) -> String {
	const LIMIT: usize = 160;
	if text.chars().count() <= LIMIT {
		return text.to_owned();
	}
	text.chars().take(LIMIT).collect::<String>() + "…"
}

fn read(contents: &Path, article: &str) -> std::io::Result<(String, i18n::store::Sidecar)> {
	let path = contents.join(article);
	let source = std::fs::read_to_string(&path)?;
	let sidecar = i18n::store::load(&i18n::store::path_for(&path))?;
	Ok((source, sidecar))
}

/// Every segment an article has or still holds a translation for.
///
/// Live ones keep the article's order, because that is the order somebody reading it would expect.
/// Stale ones have no place in the article any more, so they follow, sorted by id -- an arbitrary
/// order stated rather than whatever the map iterated in.
pub fn outline(contents: &Path, article: &str) -> std::io::Result<Outline> {
	let (source, sidecar) = read(contents, article)?;
	let live = i18n::segment::translatable(&source).map_err(|error| {
		std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{article}: {error}"))
	})?;
	let source_locale = crate::document::fields_of(&source, &contents.join(article))
		.ok()
		.as_ref()
		.and_then(crate::summary::lang_of)
		.and_then(crate::summary::source_locale);

	let mut rows: Vec<Row> = live
		.iter()
		.map(|(id, segment)| Row {
			id: id.clone(),
			stale: false,
			source: Some(preview_of(&segment.source)),
			preview: None,
			locales: sidecar.segments.get(id).map(|by| by.keys().cloned().collect()).unwrap_or_default(),
		})
		.collect();

	let mut stale: Vec<Row> = sidecar
		.segments
		.iter()
		.filter(|(id, _)| !live.contains_key(*id))
		.map(|(id, by)| Row {
			id: id.clone(),
			stale: true,
			source: None,
			// The source locale reads closest to what the paragraph said; any locale is better
			// than nothing when the article was written in one this sidecar does not carry.
			preview: source_locale
				.and_then(|locale| by.get(locale))
				.or_else(|| by.values().next())
				.map(|translation| preview_of(&translation.text)),
			locales: by.keys().cloned().collect(),
		})
		.collect();
	stale.sort_by(|left, right| left.id.cmp(&right.id));
	rows.extend(stale);

	Ok(Outline { article: article.to_owned(), rows })
}

/// One segment's translations, in the order the interface shows locales.
pub fn detail(contents: &Path, article: &str, id: &str) -> std::io::Result<Detail> {
	let (source, sidecar) = read(contents, article)?;
	let live = i18n::segment::translatable(&source).map_err(|error| {
		std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{article}: {error}"))
	})?;

	// The same order the listing shows, so a segment reads down the page like an article does.
	let order = i18n::prompt::LOCALES;
	let stored = sidecar.segments.get(id);
	let mut renderings: Vec<Rendering> = Vec::new();
	for locale in order {
		let Some(translation) = stored.and_then(|by| by.get(locale)) else {
			continue;
		};
		renderings.push(Rendering {
			locale: locale.to_owned(),
			text: translation.text.clone(),
			provider: translation.provider.clone(),
			model: translation.model.clone(),
			at: translation.at.clone(),
			tokens: translation.tokens,
			review: translation.review,
		});
	}

	Ok(Detail {
		id: id.to_owned(),
		stale: !live.contains_key(id),
		source: live.get(id).map(|segment| segment.source.clone()),
		renderings,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::BTreeMap;

	fn translation(text: &str) -> i18n::store::Translation {
		i18n::store::Translation {
			text: text.to_owned(),
			provider: "openai".to_owned(),
			model: "gpt-5".to_owned(),
			at: "2026-08-01T00:00:00Z".to_owned(),
			seconds: 1.0,
			tokens: 10,
			review: false,
		}
	}

	/// An article with one paragraph, plus a sidecar carrying that paragraph and one id the
	/// article does not have.
	fn scenario() -> (tempfile::TempDir, String) {
		let temporary = tempfile::tempdir().expect("temp");
		let contents = temporary.path();
		let path = contents.join("one.md");
		std::fs::write(&path, "---\nlang: en\n---\n\nA live paragraph.\n").expect("write");

		let source = std::fs::read_to_string(&path).expect("read");
		let live = i18n::segment::translatable(&source).expect("segments");
		let mut sidecar = i18n::store::Sidecar::default();
		for id in live.keys() {
			let mut by: BTreeMap<String, i18n::store::Translation> = BTreeMap::new();
			by.insert("en-US".to_owned(), translation("A live paragraph."));
			sidecar.segments.insert(id.clone(), by);
		}
		let mut gone: BTreeMap<String, i18n::store::Translation> = BTreeMap::new();
		gone.insert("en-US".to_owned(), translation("What the edited paragraph used to say."));
		sidecar.segments.insert("deadbeef".to_owned(), gone);
		i18n::store::save(&i18n::store::path_for(&path), &sidecar).expect("sidecar");

		(temporary, "one.md".to_owned())
	}

	#[test]
	fn a_live_segment_shows_its_source_and_a_stale_one_shows_what_it_says() {
		let (temporary, article) = scenario();
		let outline = outline(temporary.path(), &article).expect("outline");

		let live: Vec<&Row> = outline.rows.iter().filter(|row| !row.stale).collect();
		let stale: Vec<&Row> = outline.rows.iter().filter(|row| row.stale).collect();
		assert_eq!(stale.len(), 1);

		assert!(live[0].source.is_some(), "a live segment must carry its paragraph");
		assert!(live[0].preview.is_none(), "a live segment has no preview to give");
		assert!(stale[0].source.is_none(), "a stale segment has no paragraph left");
		assert_eq!(
			stale[0].preview.as_deref(),
			Some("What the edited paragraph used to say."),
			"a stale segment is shown by what it says, since nothing else survives"
		);
	}

	#[test]
	fn the_stale_ones_come_after_the_article_s_own() {
		let (temporary, article) = scenario();
		let rows = outline(temporary.path(), &article).expect("outline").rows;
		let first_stale = rows.iter().position(|row| row.stale).expect("a stale row");
		assert!(rows[..first_stale].iter().all(|row| !row.stale));
	}

	#[test]
	fn a_detail_carries_the_translations_and_says_whether_the_paragraph_is_gone() {
		let (temporary, article) = scenario();
		let detail = detail(temporary.path(), &article, "deadbeef").expect("detail");

		assert!(detail.stale);
		assert!(detail.source.is_none());
		assert_eq!(detail.renderings.len(), 1);
		assert_eq!(detail.renderings[0].locale, "en-US");
		assert_eq!(detail.renderings[0].text, "What the edited paragraph used to say.");
	}
}
