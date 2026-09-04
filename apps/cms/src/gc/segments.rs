//! Translations for paragraphs an article no longer contains.
//!
//! A segment id comes from the paragraph's canonical form, so editing a paragraph gives it a new
//! id. The next translation run fills that new id, and the old one keeps its entries -- one per
//! locale, each with the provider, model and token count that produced it. Nothing reads them
//! again and nothing removed them, so a file grew by eight entries every time a typo was fixed.
//!
//! They were kept on purpose: [i18n::store::orphans] says a corrected typo leaves a translation
//! that is still almost right and worth reading before it goes. That remains true and is exactly
//! why this is a separate, asked-for sweep rather than something a translation run does on its way
//! past -- the same reasoning the asset sweep next door is built on.
//!
//! **Scoped, unlike the asset sweep.** The command line asks about the whole corpus; the desktop
//! client asks about the article somebody is looking at, or the several they ticked. The operation
//! is one and the scope is an argument, so neither shell owns a narrower copy of it.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::i18n;
use crate::task::{Record, writer};

/// One article's stale ids.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Stale {
	/// Slash-separated and relative to `contents`, the same spelling the listing uses.
	pub article: String,
	pub ids: Vec<String>,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Sweep {
	/// Only articles with something to drop. An article with a clean sidecar is absent.
	pub articles: Vec<Stale>,
}

impl Sweep {
	pub fn total(&self) -> usize {
		self.articles.iter().map(|stale| stale.ids.len()).sum()
	}

	pub fn is_empty(&self) -> bool {
		self.articles.is_empty()
	}
}

/// What each article in `scope` is carrying, or the whole corpus when `scope` is empty.
///
/// An article named in `scope` that has no sidecar, or nothing stale in it, is simply absent from
/// the result rather than an error: asking about a clean article is a fair question with a short
/// answer.
pub fn plan(contents: &Path, scope: &[String]) -> std::io::Result<Sweep> {
	let wanted: Vec<PathBuf> = if scope.is_empty() {
		crate::refs::markdown_under(contents)?
	} else {
		scope.iter().map(|path| contents.join(path)).collect()
	};

	let mut articles = Vec::new();
	for path in wanted {
		if !path.is_file() {
			continue;
		}
		let source = std::fs::read_to_string(&path)?;
		// A file with no `lang` is a page rather than an article and has no sidecar to sweep --
		// the same test `cms i18n` and the listing apply. See spec/i18n.md.
		let fields = crate::document::fields_of(&source, &path)?;
		if crate::summary::lang_of(&fields).is_none() {
			continue;
		}

		let live = i18n::segment::translatable(&source).map_err(|error| {
			std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {error}", path.display()))
		})?;
		let sidecar = i18n::store::load(&i18n::store::path_for(&path))?;
		let ids = i18n::store::orphans(&sidecar, &live);
		if ids.is_empty() {
			continue;
		}

		let relative = path.strip_prefix(contents).unwrap_or(&path);
		articles.push(Stale {
			article: relative
				.components()
				.filter_map(|component| component.as_os_str().to_str())
				.collect::<Vec<_>>()
				.join("/"),
			ids,
		});
	}

	articles.sort_by(|left, right| left.article.cmp(&right.article));
	Ok(Sweep { articles })
}

/// Drop what `sweep` found, and report how many entries went.
///
/// Each article is re-read inside the lock rather than saved from the copy `plan` read. A plan is
/// a view of the past the moment another process writes, and saving it back would replace rather
/// than merge -- losing a translation somebody paid for, with no error anywhere. See spec/tasks.md.
pub fn apply(repository: &Path, contents: &Path, sweep: &Sweep) -> std::io::Result<usize> {
	if sweep.is_empty() {
		return Ok(0);
	}

	let translations = writer::Writer::start(repository, Record::Translations)?;
	let mut dropped = 0;
	for stale in &sweep.articles {
		let path = i18n::store::path_for(&contents.join(&stale.article));
		let ids = stale.ids.clone();
		let counted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let seen = std::sync::Arc::clone(&counted);
		translations.apply(move || {
			let mut sidecar = i18n::store::load(&path)?;
			let before = sidecar.segments.len();
			sidecar.segments.retain(|id, _| !ids.contains(id));
			seen.store(before - sidecar.segments.len(), std::sync::atomic::Ordering::Relaxed);
			i18n::store::save(&path, &sidecar)
		})?;
		dropped += counted.load(std::sync::atomic::Ordering::Relaxed);
	}
	Ok(dropped)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// An article plus a sidecar holding exactly `sidecar_ids`.
	fn article(directory: &Path, name: &str, body: &str, sidecar_ids: &[&str]) {
		let path = directory.join(name);
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent).expect("dir");
		}
		std::fs::write(&path, format!("---\nlang: en\n---\n\n{body}\n")).expect("write");

		let mut sidecar = i18n::store::Sidecar::default();
		for id in sidecar_ids {
			sidecar.segments.insert((*id).to_owned(), Default::default());
		}
		i18n::store::save(&i18n::store::path_for(&path), &sidecar).expect("sidecar");
	}

	/// The ids an article's own paragraphs currently hash to.
	fn live_ids(path: &Path) -> Vec<String> {
		let source = std::fs::read_to_string(path).expect("read");
		i18n::segment::translatable(&source).expect("segments").into_keys().collect()
	}

	#[test]
	fn an_id_the_article_still_has_is_not_stale() {
		let directory = tempfile::tempdir().expect("temp");
		let contents = directory.path();
		article(contents, "one.md", "A paragraph.", &[]);
		let ids = live_ids(&contents.join("one.md"));
		let kept: Vec<&str> = ids.iter().map(String::as_str).collect();
		article(contents, "one.md", "A paragraph.", &kept);

		let sweep = plan(contents, &[]).expect("plan");
		assert!(sweep.is_empty(), "a live id was reported as stale: {sweep:?}");
	}

	#[test]
	fn an_id_the_article_dropped_is_stale_and_is_swept() {
		let directory = tempfile::tempdir().expect("temp");
		let contents = directory.path();
		article(contents, "one.md", "A paragraph.", &["deadbeef"]);

		let sweep = plan(contents, &[]).expect("plan");
		assert_eq!(sweep.total(), 1);
		assert_eq!(sweep.articles[0].article, "one.md");

		let dropped = apply(contents, contents, &sweep).expect("apply");
		assert_eq!(dropped, 1);
		assert!(plan(contents, &[]).expect("replan").is_empty(), "sweeping left something behind");
	}

	#[test]
	fn a_scope_answers_only_for_what_it_names() {
		let directory = tempfile::tempdir().expect("temp");
		let contents = directory.path();
		article(contents, "one.md", "A paragraph.", &["deadbeef"]);
		article(contents, "two.md", "Another paragraph.", &["cafebabe"]);

		assert_eq!(plan(contents, &[]).expect("all").articles.len(), 2);

		let narrowed = plan(contents, &["two.md".to_owned()]).expect("scoped");
		assert_eq!(narrowed.articles.len(), 1);
		assert_eq!(narrowed.articles[0].article, "two.md");
	}

	#[test]
	fn a_page_without_a_language_is_not_swept() {
		let directory = tempfile::tempdir().expect("temp");
		let contents = directory.path();
		std::fs::write(contents.join("page.md"), "---\ntitle: Home\n---\n\nA paragraph.\n")
			.expect("write");

		assert!(plan(contents, &[]).expect("plan").is_empty());
	}
}
