//! The `cms tag` command: what kind of image this is, and what is in it.
//!
//! One request per image covering both, because they are one look at one picture. Asking
//! separately would pay twice for the same glance and let the two answers disagree -- a
//! `screenshot` tagged `landscape` is a contradiction only a second request can produce.
//!
//! Existing tags go into the prompt in full. Left to itself a model writes `terminal-window`
//! beside `terminal` and `cli` beside both; telling it to be consistent achieves nothing when
//! it has nothing to be consistent with. See spec/architecture.md.

use crate::i18n::runner::{self, Refusal, Runner};
use crate::media::{self, Category, Entry};
use crate::tags::{self, Tag};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many tags one image should carry.
///
/// Below three and a picture is barely findable; above five and every tag is one somebody
/// added because the budget allowed it rather than because it says anything.
const MIN_TAGS: usize = 3;
const MAX_TAGS: usize = 5;

#[derive(Debug, Default)]
pub struct Outcome {
	pub classified: usize,
	pub skipped: usize,
	pub tokens: u64,
	pub usd: f64,
	pub failed: Vec<(String, String)>,
	pub unreadable: Vec<String>,
	pub exhausted: Option<String>,
	/// Tags the model asked for that were not already in the registry.
	pub minted: Vec<String>,
}

fn prompt(path: &Path, existing: &[&str]) -> String {
	let categories = Category::all().join(", ");
	let known = if existing.is_empty() {
		"There are no tags yet, so every one you give will be a new one.".to_owned()
	} else {
		format!(
			"Tags already in use, in full. Reuse one wherever it fits, even loosely -- a near \
			 match is better than a new tag that means almost the same thing:\n{}",
			existing.join(", ")
		)
	};

	format!(
		"Look at the image at {} and answer two questions about it.\n\
		 \n\
		 First, its category. Exactly one of: {categories}.\n\
		 A screenshot is a capture of a screen whatever is on it; a terminal, a browser and an \
		 editor are all screenshots. A diagram is drawn to explain something. A document has \
		 text as its subject. Artwork is illustrated, rendered or generated.\n\
		 \n\
		 Second, between {MIN_TAGS} and {MAX_TAGS} tags for what is in it. Lower case, digits \
		 and hyphens only, English, no spaces. A tag names a subject, a tool, a place or a \
		 medium -- not an opinion and not the category again.\n\
		 \n\
		 {known}\n\
		 \n\
		 Answer in exactly two lines and nothing else:\n\
		 category: <one word>\n\
		 tags: <comma-separated>",
		path.display()
	)
}

/// Read the two lines back.
///
/// Line-anchored for the same reason translations are: a malformed answer costs one field
/// rather than the whole reply, and there is no structure to get subtly wrong.
fn parse(reply: &str) -> (Option<Category>, Vec<String>) {
	let mut category = None;
	let mut tags = Vec::new();
	for line in reply.lines() {
		let line = line.trim();
		if let Some(value) = line.strip_prefix("category:") {
			category = Category::parse(value);
		} else if let Some(value) = line.strip_prefix("tags:") {
			tags = value
				.split(',')
				.map(|tag| tag.trim().to_ascii_lowercase())
				// Silently dropped rather than repaired. A tag with a space in it is a phrase,
				// and turning it into one by replacing the space invents a name nobody chose.
				.filter(|tag| media::is_valid_tag(tag))
				.collect();
			tags.dedup();
			tags.truncate(MAX_TAGS);
		}
	}
	(category, tags)
}

fn bar(total: u64) -> ProgressBar {
	let bar = ProgressBar::new(total);
	bar.set_style(
		ProgressStyle::with_template("  {bar:28} {pos}/{len}  {wide_msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar()),
	);
	bar
}

/// Originals on hand, by the id they hash to.
fn originals_by_id(originals: &Path) -> BTreeMap<String, PathBuf> {
	let Ok(entries) = std::fs::read_dir(originals) else {
		return BTreeMap::new();
	};
	entries
		.filter_map(Result::ok)
		.map(|entry| entry.path())
		.filter(|path| path.is_file())
		.filter_map(|path| {
			let bytes = std::fs::read(&path).ok()?;
			Some((crate::image::cid(&bytes), path))
		})
		.collect()
}

/// Classify and tag every asset that has neither yet.
pub async fn run(
	repo: &Path,
	runner: Runner,
	force: bool,
	limit: Option<usize>,
) -> std::io::Result<Outcome> {
	let merged = crate::image::run::load(&repo.join(crate::image::run::MERGED));
	let described_path = media::path_for(repo);
	let mut described = media::load(&described_path);
	let registry_path = tags::path_for(repo);
	let mut registry = tags::load(&registry_path);

	let wanted: Vec<String> = merged
		.media
		.keys()
		.filter(|cid| {
			force
				|| described
					.media
					.get(*cid)
					.is_none_or(|entry| entry.category.is_none() || entry.tags.is_empty())
		})
		.cloned()
		.collect();

	let mut outcome = Outcome {
		skipped: merged.media.len() - wanted.len(),
		..Outcome::default()
	};
	if wanted.is_empty() {
		return Ok(outcome);
	}

	let by_id = originals_by_id(&repo.join("data").join("image"));
	let mut todo: Vec<(String, PathBuf)> = Vec::new();
	for cid in wanted {
		match by_id.get(&cid) {
			Some(path) => todo.push((cid, path.clone())),
			None => outcome.unreadable.push(cid),
		}
	}
	if let Some(limit) = limit {
		todo.truncate(limit);
	}

	// Refused up front rather than once per image. A runner that cannot see would fail every
	// one of these identically, and finding that out twenty times is not finding it out
	// better.
	let Some(model) = runner.model_for_vision() else {
		outcome.failed.push((
			String::new(),
			format!(
				"{} cannot read an image; pick a runner that can",
				runner.provider()
			),
		));
		return Ok(outcome);
	};

	// One at a time, unlike translation. Each answer changes the list the next request is
	// shown, and running four in parallel would let four images each invent their own name
	// for the same thing before any of them could see the others.
	let progress = bar(todo.len() as u64);
	for (cid, path) in todo {
		progress.set_message(
			path
				.file_name()
				.map(|n| n.to_string_lossy().chars().take(40).collect::<String>())
				.unwrap_or_default(),
		);

		let text = prompt(&path, &tags::known(&registry));
		let answer = match runner::ask_vision(runner, &text, model, &path).await {
			Ok(answer) => answer,
			Err(Refusal::Exhausted(reason)) => {
				outcome.exhausted = Some(reason);
				break;
			}
			Err(error) => {
				outcome.failed.push((cid, error.to_string()));
				progress.inc(1);
				continue;
			}
		};

		let (category, found) = parse(&answer.text);
		if found.len() < MIN_TAGS {
			outcome
				.failed
				.push((cid, format!("only {} usable tags", found.len())));
			progress.inc(1);
			continue;
		}

		for tag in &found {
			if !registry.tags.contains_key(tag) {
				registry.tags.insert(tag.clone(), Tag::default());
				outcome.minted.push(tag.clone());
			}
		}

		let entry = described.media.entry(cid).or_insert_with(Entry::default);
		entry.category = category;
		entry.tags = found;
		outcome.classified += 1;
		outcome.tokens += answer.tokens;
		outcome.usd += answer.usd;

		// Written as they arrive, for the reason every other command here writes as it goes:
		// each of these was paid for, and holding a run's worth in memory means one interrupt
		// discards all of it.
		media::save(&described_path, &described)?;
		tags::save(&registry_path, &registry)?;
		progress.inc(1);
	}
	progress.finish_and_clear();
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_two_answers_are_read_off_two_lines() {
		let (category, tags) = parse("category: screenshot\ntags: terminal, cargo, rust\n");
		assert_eq!(category, Some(Category::Screenshot));
		assert_eq!(tags, vec!["terminal", "cargo", "rust"]);
	}

	#[test]
	fn a_malformed_tag_is_dropped_and_not_repaired() {
		// Replacing the space would invent `shell-terminal`, a name nobody chose and which now
		// competes with `terminal` forever.
		let (_, tags) = parse("category: screenshot\ntags: terminal, shell terminal, TypeScript, rust");
		assert_eq!(tags, vec!["terminal", "typescript", "rust"]);
	}

	#[test]
	fn a_bad_category_leaves_the_field_empty_rather_than_guessing() {
		let (category, tags) = parse("category: terminal\ntags: a, b, c");
		assert_eq!(category, None);
		// The tags still land: one malformed line costs one field.
		assert_eq!(tags.len(), 3);
	}

	#[test]
	fn the_budget_holds_at_the_top() {
		let (_, tags) = parse("tags: a, b, c, d, e, f, g, h");
		assert_eq!(tags.len(), MAX_TAGS);
	}

	#[test]
	fn the_prompt_shows_the_model_what_already_exists() {
		// Without the list, a model invents a near-duplicate of a tag it cannot see. With it,
		// reuse is the cheaper answer.
		let with = prompt(Path::new("/tmp/a.png"), &["terminal", "rust"]);
		assert!(with.contains("terminal, rust"));
		assert!(with.contains("Reuse one wherever it fits"));

		let without = prompt(Path::new("/tmp/a.png"), &[]);
		assert!(without.contains("no tags yet"));
	}
}
