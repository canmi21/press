//! The article listing shared by the command-line and desktop adapters.
//!
//! One walk answers both questions the interface asks about writing: what exists, and what the
//! derived records owe it. Splitting them would mean reading every article and both its sidecars
//! twice to produce two views that must agree, and a disagreement between them is a bug nobody
//! can see from either page alone.

use serde::Serialize;
use std::path::Path;

use crate::{i18n, paths, refs, summary};

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
	/// Every locale an article is expected to reach, in the order the interface shows them.
	pub locales: Vec<String>,
	pub articles: Vec<Article>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Article {
	/// Slash-separated and relative to `contents`, so it reads the same on every platform and
	/// can address the file again without the caller rebuilding an absolute path.
	pub path: String,
	pub section: String,
	pub title: String,
	pub subtitle: Option<String>,
	/// Authored `lastmod`, falling back to `created` for an article not yet revised.
	pub modified: Option<String>,
	/// The `lang` frontmatter. A file without one is a page, not an article, and is absent here.
	pub lang: String,
	/// Translatable segments the article currently holds.
	pub segments: usize,
	/// (segment, locale) pairs that exist.
	pub translated: usize,
	/// (segment, locale) pairs the article wants. `translated` counts against this.
	pub wanted: usize,
	/// Per locale, how many segments are still missing. Locales that are complete are absent.
	pub gaps: Vec<LocaleGap>,
	/// Translations left behind by an edit, still in the sidecar under an id the article dropped.
	pub orphans: usize,
	/// Whether a summary exists in the article's own language.
	pub summary: bool,
	/// Locales whose summary is missing, in `locales` order.
	pub summary_gaps: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocaleGap {
	pub locale: String,
	pub segments: usize,
}

#[derive(Debug)]
pub enum Error {
	Repository(paths::NotFound),
	Read(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Repository(error) => error.fmt(formatter),
			Self::Read(error) => error.fmt(formatter),
		}
	}
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Self::Read(error)
	}
}

pub fn listing() -> Result<Listing, Error> {
	let repository = paths::repo_root().map_err(Error::Repository)?;
	Ok(listing_at(&repository)?)
}

pub fn listing_at(repository: &Path) -> std::io::Result<Listing> {
	let contents = repository.join("contents");
	let locales = i18n::prompt::LOCALES;
	// Loaded once for the whole walk: a gloss applies to a segment id, so which article recorded
	// it stops mattering the moment it is written down.
	let glosses = i18n::tn::load(&i18n::tn::path_for(&contents))?;

	let mut articles = Vec::new();
	for path in refs::markdown_under(&contents)? {
		let source = std::fs::read_to_string(&path)?;
		// The same test `cms summary` and `cms i18n` apply: no `lang`, no language to translate
		// out of, so the file is a page rather than an article. The homepage is the standing
		// example. See spec/i18n.md.
		let Some(lang) = summary::lang_of(&source) else {
			continue;
		};
		let source_locale = summary::source_locale(&lang);

		let live = i18n::segment::translatable(&source);
		let sidecar = i18n::store::load(&i18n::store::path_for(&path))?;
		let missing = i18n::store::missing(&sidecar, &live, &locales, source_locale, &glosses);

		let wanted = live.len() * locales.len();
		let absent: usize = missing.values().map(Vec::len).sum();
		let mut gaps: Vec<LocaleGap> = locales
			.iter()
			.filter_map(|locale| {
				let segments = missing
					.values()
					.filter(|absent| absent.iter().any(|value| value == locale))
					.count();
				(segments > 0).then(|| LocaleGap {
					locale: (*locale).to_owned(),
					segments,
				})
			})
			.collect();
		gaps.sort_by_key(|gap| std::cmp::Reverse(gap.segments));

		let summaries = summary::load(&summary::sidecar_for(&path))?;
		let summary_gaps: Vec<String> = locales
			.iter()
			.filter(|locale| !summaries.summary.contains_key(**locale))
			.map(|locale| (*locale).to_owned())
			.collect();

		let relative = path.strip_prefix(&contents).unwrap_or(&path);
		let section = relative
			.parent()
			.filter(|parent| !parent.as_os_str().is_empty())
			.and_then(|parent| parent.components().next())
			.and_then(|component| component.as_os_str().to_str())
			.unwrap_or("other")
			.to_owned();

		articles.push(Article {
			path: relative
				.components()
				.filter_map(|component| component.as_os_str().to_str())
				.collect::<Vec<_>>()
				.join("/"),
			section,
			title: summary::title_of(&source).unwrap_or_else(|| {
				relative
					.file_stem()
					.and_then(|stem| stem.to_str())
					.unwrap_or("untitled")
					.to_owned()
			}),
			subtitle: summary::subtitle_of(&source),
			modified: summary::modified_of(&source),
			lang,
			segments: live.len(),
			translated: wanted.saturating_sub(absent),
			wanted,
			gaps,
			orphans: i18n::store::orphans(&sidecar, &live).len(),
			summary: source_locale.is_some_and(|locale| summaries.summary.contains_key(locale)),
			summary_gaps,
		});
	}

	articles.sort_by(|left, right| {
		left
			.section
			.cmp(&right.section)
			.then_with(|| left.path.cmp(&right.path))
	});

	Ok(Listing {
		locales: locales.iter().map(|locale| (*locale).to_owned()).collect(),
		articles,
	})
}
