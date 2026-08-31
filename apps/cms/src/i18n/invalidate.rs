//! `cms invalidate`: drop recorded translations precisely, so the next run redoes them.
//!
//! The sidecar is a map, so invalidation is deletion: remove the entries a selector names and
//! the ordinary repair run buys exactly those back, merging into everything kept. This existed
//! twice as a throwaway script before it was a command -- retiring `source` provenance, then
//! reselecting note-bearing segments after a policy change -- which is the extraction
//! threshold; see spec/architecture/workspace.md.
//!
//! Dry by default, like `cms gc`: the selection is printed and nothing is touched until
//! `--live`. What a person vouched for is never dropped -- a `review: true` entry outranks any
//! selector, because the flag means a human read that text and no policy sweep should undo a
//! judgement it cannot see.

use super::{segment, store};
use std::path::{Path, PathBuf};

/// Which stored entries a run means. At least one selector must be given: a selection that
/// matches by default would make the empty command line mean "drop everything".
pub struct Selection<'a> {
	/// Exact segment ids.
	pub segments: &'a [String],
	/// Segments whose live source contains any of these.
	pub containing: &'a [String],
	/// Entries whose stored translation contains any of these.
	pub translation_containing: &'a [String],
	/// Restrict to these locales; empty means every locale.
	pub locales: &'a [String],
}

impl Selection<'_> {
	pub fn names_nothing(&self) -> bool {
		self.segments.is_empty() && self.containing.is_empty() && self.translation_containing.is_empty()
	}

	/// Whether this entry is selected, given the live source its segment currently has.
	///
	/// A segment the article no longer contains has no source to match `containing` against;
	/// its entries are still addressable by id or by translation text, and otherwise belong to
	/// the orphan report rather than to this command.
	fn wants(&self, id: &str, source: Option<&str>, locale: &str, text: &str) -> bool {
		if !self.locales.is_empty() && !self.locales.iter().any(|wanted| wanted == locale) {
			return false;
		}
		self.segments.iter().any(|wanted| wanted == id)
			|| source.is_some_and(|source| self.containing.iter().any(|needle| source.contains(needle)))
			|| self.translation_containing.iter().any(|needle| text.contains(needle))
	}
}

#[derive(Debug, Default)]
pub struct Report {
	/// (article, segment id, locales dropped).
	pub dropped: Vec<(String, String, Vec<String>)>,
	/// Entries a selector named but `review: true` protected.
	pub kept_reviewed: usize,
}

/// Apply `selection` under `articles`, deleting only when `live` is set.
pub fn run(
	articles: &Path,
	only: &[PathBuf],
	selection: &Selection<'_>,
	live: bool,
) -> std::io::Result<Report> {
	let mut report = Report::default();
	for path in crate::refs::markdown_under(articles)? {
		if !only.is_empty() && !only.iter().any(|wanted| path.ends_with(wanted) || path == *wanted) {
			continue;
		}
		let sidecar_path = store::path_for(&path);
		let mut sidecar = match store::load_checked(&sidecar_path)? {
			Some(sidecar) if !sidecar.segments.is_empty() => sidecar,
			_ => continue,
		};
		let article = std::fs::read_to_string(&path)?;
		let sources: std::collections::BTreeMap<String, String> = segment::translatable(&article)
			.map(|live| live.into_iter().map(|(id, segment)| (id, segment.source)).collect())
			.unwrap_or_default();

		let mut changed = false;
		for (id, locales) in sidecar.segments.iter_mut() {
			let mut dropped = Vec::new();
			locales.retain(|locale, entry| {
				if !selection.wants(id, sources.get(id).map(String::as_str), locale, &entry.text) {
					return true;
				}
				if entry.review {
					report.kept_reviewed += 1;
					return true;
				}
				dropped.push(locale.clone());
				false
			});
			if !dropped.is_empty() {
				changed = true;
				report.dropped.push((path.display().to_string(), id.clone(), dropped));
			}
		}
		sidecar.segments.retain(|_, locales| !locales.is_empty());
		if live && changed {
			store::save(&sidecar_path, &sidecar)?;
		}
	}
	Ok(report)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn selection<'a>(
		segments: &'a [String],
		containing: &'a [String],
		translation_containing: &'a [String],
		locales: &'a [String],
	) -> Selection<'a> {
		Selection { segments, containing, translation_containing, locales }
	}

	#[test]
	fn an_empty_command_line_names_nothing() {
		assert!(selection(&[], &[], &[], &[]).names_nothing());
		let ids = ["abc".to_owned()];
		assert!(!selection(&ids, &[], &[], &[]).names_nothing());
		// A locale alone is a restriction, not a selection.
		let locales = ["ja-JP".to_owned()];
		assert!(selection(&[], &[], &[], &locales).names_nothing());
	}

	#[test]
	fn selectors_reach_an_entry_by_id_source_or_translation() {
		let ids = ["abc".to_owned()];
		let by_id = selection(&ids, &[], &[], &[]);
		assert!(by_id.wants("abc", None, "ja-JP", "text"));
		assert!(!by_id.wants("def", None, "ja-JP", "text"));

		let needles = [":fn[".to_owned()];
		let by_source = selection(&[], &needles, &[], &[]);
		assert!(by_source.wants("x", Some("a :fn[word]{is=\"note\"} b"), "ja-JP", "text"));
		assert!(!by_source.wants("x", Some("plain"), "ja-JP", "text"));
		// A segment the article dropped has no source to match.
		assert!(!by_source.wants("x", None, "ja-JP", "a :fn[w]{is=\"n\"} b"));

		let by_translation = selection(&[], &[], &needles, &[]);
		assert!(by_translation.wants("x", None, "ja-JP", "a :fn[w]{is=\"n\"} b"));
		assert!(!by_translation.wants("x", Some("a :fn[w]{is=\"n\"} b"), "ja-JP", "plain"));
	}

	#[test]
	fn a_locale_filter_restricts_every_selector() {
		let ids = ["abc".to_owned()];
		let locales = ["ja-JP".to_owned()];
		let narrowed = selection(&ids, &[], &[], &locales);
		assert!(narrowed.wants("abc", None, "ja-JP", "text"));
		assert!(!narrowed.wants("abc", None, "de-DE", "text"));
	}
}
