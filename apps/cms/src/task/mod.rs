//! What long-running work exists, declared in one place.
//!
//! Every operation that takes more than an instant is described here and nowhere else. The point
//! is that something can ask the question -- how many are there, which ones cost money, what does
//! this one touch -- without importing thirteen modules and reading their argument parsing. A GUI
//! listing tasks, a scheduler ordering them and a person running `cms tasks` all read this slice.
//!
//! This is data. Nothing here runs anything, and adding a task to the catalogue does not make it
//! runnable; it makes it *known*. Keeping the two apart is what lets the catalogue be complete
//! before the runner exists, which is the state this module ships in.
//!
//! ## What is not here
//!
//! Ordering. `after` records which tasks must have run first, because that fact belongs beside
//! the task rather than inside whatever eventually schedules them -- but nothing reads it yet and
//! no scheduler exists. Declaring it now means the scheduler can be written without reopening
//! thirteen operations to ask what they depend on.

pub mod claim;
pub mod progress;
pub mod registry;
pub mod writer;

use serde::Serialize;

/// A record store a task reads or mutates.
///
/// Named for the record rather than the path, because two tasks conflict when they write the same
/// *records*, and the file layout underneath is free to change without rewriting the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Record {
	/// `contents/**/*.md`. Rewritten only by `image`, which replaces reference paths in place.
	Articles,
	/// `contents/**/*.i18n.yaml`.
	Translations,
	/// `contents/**/*.summary.yaml`.
	Summaries,
	/// The translation-note table: phrases a translation has to gloss rather than render.
	Notes,
	/// `data/media.yaml`: descriptions and categories, which no command can rebuild.
	Media,
	/// `data/tags.yaml`.
	Tags,
	/// `data/build/segments.json`.
	Segments,
	/// Crate and repository facts the articles embed.
	Embeds,
	/// `data/public/image/**`.
	PublicImage,
	/// `data/public/favicon/**`.
	PublicFavicon,
	/// `data/public/opengraph/**`.
	PublicOpengraph,
	/// `data/public/license/**`.
	PublicLicense,
}

/// One long-running operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
	/// The `cms` subcommand, and the id everything else addresses this task by.
	pub id: &'static str,
	pub name: &'static str,
	pub detail: &'static str,
	/// Whether a run asks a model, and therefore spends money. Shown before anything offers it.
	pub paid: bool,
	/// Whether the operation fans out internally, so a run has many items rather than one.
	///
	/// This decides whether contention can be resolved per item. A task with one indivisible
	/// item can only be skipped whole; a task with many can hand off the ones already claimed
	/// and keep the rest.
	pub items: Items,
	pub reads: &'static [Record],
	pub writes: &'static [Record],
	/// Tasks whose output this one consumes. Declared, not yet enforced -- see the module note.
	pub after: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Items {
	/// One unit of work that cannot be divided; a second runner can only stand aside.
	Whole,
	/// Many independently claimable units, described by what one unit is keyed on.
	Many(&'static str),
}

impl Spec {
	/// Whether two tasks can be in flight at once.
	///
	/// Reading the same record is not a conflict, and neither is writing a record another task
	/// only reads -- a reader that wants a consistent view takes it at the moment it reads.
	/// What cannot overlap is two tasks writing the same record, and even that is a statement
	/// about their *mutations*, which the writer serialises. This answers the coarser question
	/// an interface asks first: may these two be offered together.
	pub fn conflicts_with(&self, other: &Spec) -> bool {
		self
			.writes
			.iter()
			.any(|record| other.writes.contains(record))
	}
}

/// Every long-running operation, in no significant order.
///
/// `overview`, `articles`, `derived`, `check` and `port` are absent on purpose: they read and
/// return, and calling them tasks would put five entries in every list that can never be waited
/// on, watched, or scheduled.
pub const CATALOG: &[Spec] = &[
	Spec {
		id: "image",
		name: "Derive images",
		detail: "Import what the articles reference, derive variants, then rewrite the references.",
		paid: false,
		items: Items::Many("image reference"),
		reads: &[Record::Articles],
		// The only task that edits article text. Everything reading `Articles` is downstream of
		// it, which is why so many entries below name it in `after`.
		writes: &[Record::Articles, Record::PublicImage],
		after: &[],
	},
	Spec {
		id: "favicon",
		name: "Collect favicons",
		detail: "Fetch the icon each linkcard draws, one per site an article links to.",
		paid: false,
		items: Items::Many("domain"),
		reads: &[Record::Articles],
		writes: &[Record::PublicFavicon],
		after: &[],
	},
	Spec {
		id: "segments",
		name: "Write segment layout",
		detail: "Record each article's segment ids and their source ranges.",
		paid: false,
		items: Items::Whole,
		reads: &[Record::Articles],
		writes: &[Record::Segments],
		after: &["image"],
	},
	Spec {
		id: "alt",
		name: "Describe images",
		detail: "Ask a model for an accessible description of every picture that has none.",
		paid: true,
		items: Items::Many("content id"),
		reads: &[Record::Articles, Record::PublicImage],
		writes: &[Record::Media],
		after: &["image"],
	},
	Spec {
		id: "tag",
		name: "Classify images",
		detail: "Give each picture a category and tags.",
		paid: true,
		items: Items::Many("content id"),
		reads: &[Record::PublicImage],
		writes: &[Record::Media, Record::Tags],
		after: &["image"],
	},
	Spec {
		id: "tn",
		name: "Find translation notes",
		detail: "Suggest passages a translation would have to gloss rather than render.",
		paid: true,
		items: Items::Many("article"),
		reads: &[Record::Articles],
		writes: &[Record::Notes],
		after: &["segments"],
	},
	Spec {
		id: "i18n",
		name: "Translate articles",
		detail: "Carry every article segment into every locale.",
		paid: true,
		items: Items::Many("article, segment and locale"),
		reads: &[Record::Articles, Record::Notes],
		writes: &[Record::Translations],
		after: &["segments", "tn"],
	},
	Spec {
		id: "summary",
		name: "Write summaries",
		detail: "Write a reader-facing summary into each article, in the article's own language.",
		paid: true,
		items: Items::Many("article"),
		reads: &[Record::Articles],
		writes: &[Record::Summaries],
		after: &[],
	},
	Spec {
		id: "locale",
		name: "Translate labels and descriptions",
		detail: "Carry tag labels, image descriptions and summaries into every locale.",
		paid: true,
		items: Items::Many("record and locale"),
		reads: &[Record::Media, Record::Tags, Record::Summaries],
		writes: &[Record::Media, Record::Tags, Record::Summaries],
		after: &["alt", "tag", "summary"],
	},
	Spec {
		id: "embed",
		name: "Fetch embedded data",
		detail: "Collect the crate and repository facts the articles embed.",
		paid: false,
		items: Items::Many("crate or repository"),
		reads: &[Record::Articles],
		writes: &[Record::Embeds],
		after: &[],
	},
	Spec {
		id: "og",
		name: "Render OpenGraph cards",
		detail: "Draw one card per page per language.",
		paid: false,
		items: Items::Many("page and locale"),
		reads: &[Record::Articles, Record::Translations],
		writes: &[Record::PublicOpengraph],
		after: &["i18n"],
	},
	Spec {
		id: "licenses",
		name: "Record licences",
		detail: "Record the licence of every dependency the apps ship.",
		paid: false,
		items: Items::Whole,
		reads: &[],
		writes: &[Record::PublicLicense],
		after: &[],
	},
	Spec {
		id: "gc",
		name: "Collect garbage",
		detail: "Drop published assets no article asks for.",
		paid: false,
		items: Items::Many("published asset"),
		reads: &[Record::Articles],
		// Deleting is writing. It is listed last and depends on everything that publishes,
		// because running it before those have caught up removes what they were about to claim.
		writes: &[
			Record::PublicImage,
			Record::PublicFavicon,
			Record::PublicOpengraph,
		],
		after: &["image", "favicon", "og"],
	},
];

/// The task with this id, if the catalogue has one.
pub fn find(id: &str) -> Option<&'static Spec> {
	CATALOG.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_id_is_unique() {
		let mut seen: Vec<&str> = CATALOG.iter().map(|spec| spec.id).collect();
		let count = seen.len();
		seen.sort_unstable();
		seen.dedup();
		assert_eq!(seen.len(), count, "two tasks share an id");
	}

	/// A dependency naming a task that does not exist is a scheduler that deadlocks or silently
	/// skips, and the mistake is invisible until something reads `after` -- which nothing does
	/// yet. Checking it here is what makes declaring the edges early safe.
	#[test]
	fn every_dependency_names_a_task_in_the_catalogue() {
		for spec in CATALOG {
			for dependency in spec.after {
				assert!(
					find(dependency).is_some(),
					"{} depends on {dependency}, which is not a task",
					spec.id
				);
			}
		}
	}

	#[test]
	fn no_task_depends_on_itself() {
		for spec in CATALOG {
			assert!(
				!spec.after.contains(&spec.id),
				"{} depends on itself",
				spec.id
			);
		}
	}

	/// The pair that motivated per-item claiming rather than per-file leases: both spend minutes
	/// asking a model and both touch `data/media.yaml` for milliseconds at the end.
	#[test]
	fn describing_and_classifying_contend_over_the_same_record() {
		let alt = find("alt").expect("alt");
		let tag = find("tag").expect("tag");
		assert!(alt.conflicts_with(tag));
		assert!(tag.conflicts_with(alt));
	}

	#[test]
	fn tasks_writing_unrelated_records_do_not_contend() {
		let favicon = find("favicon").expect("favicon");
		let i18n = find("i18n").expect("i18n");
		assert!(!favicon.conflicts_with(i18n));
	}

	#[test]
	fn only_deriving_images_rewrites_article_text() {
		let writers: Vec<&str> = CATALOG
			.iter()
			.filter(|spec| spec.writes.contains(&Record::Articles))
			.map(|spec| spec.id)
			.collect();
		assert_eq!(writers, vec!["image"]);
	}
}
