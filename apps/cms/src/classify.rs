//! The `cms tag` command: what kind of image this is, and what is in it.
//!
//! One request per image covering both, because they are one look at one picture. Asking
//! separately would pay twice for the same glance and let the two answers disagree -- a
//! `screenshot` tagged `landscape` is a contradiction only a second request can produce.
//!
//! Existing tags go into the prompt in full. Left to itself a model writes `terminal-window`
//! beside `terminal` and `cli` beside both; telling it to be consistent achieves nothing when
//! it has nothing to be consistent with. See spec/architecture.md.

use crate::alt::SOURCE_LOCALE;
use crate::i18n::runner::{self, Refusal, Runner};
use crate::i18n::store::Translation;
use crate::media::{self, Category, Entry};
use crate::tags::{self, Tag};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many tags one image should carry.
///
/// Below three and a picture is barely findable; above five and every tag is one somebody
/// added because the budget allowed it rather than because it says anything.
const MIN_TAGS: usize = 3;
const MAX_TAGS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
struct Tagged {
	name: String,
	tag: Tag,
}

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

fn prompt(path: &Path, existing: &[String]) -> String {
	let categories = Category::all().join(", ");
	let known = if existing.is_empty() {
		"There are no tags yet, so every one you give will be a new one.".to_owned()
	} else {
		format!(
			"Tags already in use, with the exact concept each one denotes. Reuse a raw identifier \
			 only when that meaning matches the image. Copy the identifier alone into the answer; \
			 do not restate or alter its stored fields. If the same surface word would mean something \
			 else, mint a qualified identifier instead:\n{}",
			existing.join("\n")
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
		 Second, between {MIN_TAGS} and {MAX_TAGS} tags for what is in it. A tag names a subject, \
		 a named tool, a place or a medium -- not an opinion and not the category again. Every \
		 new tag needs a raw identifier, a kind, an English label and a short semantic meaning.\n\
		 \n\
		 The raw identifier uses lower case, digits and hyphens only, English, no spaces. It must \
		 identify one concept without relying on this image for context. Avoid ambiguous bare words: \
		 use `cellular-network`, not `cellular`; use `mold-linker` for the named linker and `mold` \
		 for fungal growth.\n\
		 \n\
		 A technical tag is a proper name -- a brand, named tool, format, protocol or organisation \
		 whose name stays the same in every language. Its display is the official casing and spacing, \
		 including deliberately lowercase names: `cargo|technical|Cargo|Rust package manager and \
		 build tool`, `typescript|technical|TypeScript|programming language`, or \
		 `mold-linker|technical|mold|high-performance executable linker`. A generic class such as \
		 `linker` is ordinary even though it is technical subject matter.\n\
		 \n\
		 An ordinary tag is a common noun a reader expects in their own language. Give it a \
		 disambiguated English standalone label in Title Case: `cellular-network|ordinary|Cellular \
		 Network|mobile carrier connectivity and SIM service, not biology` or \
		 `mold|ordinary|Mold|fungal growth on a surface`. The meaning is one short English phrase. \
		 Do not put `|` or a newline inside any field.\n\
		 \n\
		 {known}\n\
		 \n\
		 Answer with one category line followed by one line per tag, and nothing else:\n\
		 category: <one word>\n\
		 tag: <existing raw identifier>\n\
		 tag: <new raw|technical|official display|meaning>\n\
		 tag: <new raw|ordinary|English source label|meaning>",
		path.display()
	)
}

/// Read the two lines back.
///
/// Line-anchored for the same reason translations are: a malformed answer costs one field
/// rather than the whole reply, and there is no structure to get subtly wrong.
fn parse_tag(value: &str, registry: &tags::Registry) -> Option<Tagged> {
	let value = value.trim();
	if !value.contains('|') {
		let tag = registry.tags.get(value)?;
		return media::is_valid_tag(value).then(|| Tagged {
			name: value.to_owned(),
			tag: tag.clone(),
		});
	}

	let mut fields = value.split('|').map(str::trim);
	let name = fields.next()?;
	let kind = fields.next()?;
	let label = fields.next()?;
	let meaning = fields.next()?;
	if fields.next().is_some()
		|| registry.tags.contains_key(name)
		|| !media::is_valid_tag(name)
		|| label.is_empty()
		|| meaning.is_empty()
	{
		return None;
	}
	let tag = match kind {
		"technical" => Tag::Technical {
			display: label.to_owned(),
			meaning: meaning.to_owned(),
		},
		"ordinary" => Tag::ordinary(label, meaning),
		_ => return None,
	};
	Some(Tagged {
		name: name.to_owned(),
		tag,
	})
}

fn parse(reply: &str, registry: &tags::Registry) -> (Option<Category>, Vec<Tagged>) {
	let mut category = None;
	let mut tags = Vec::new();
	for line in reply.lines() {
		let line = line.trim();
		if let Some(value) = line.strip_prefix("category:") {
			category = Category::parse(value);
		} else if let Some(value) = line.strip_prefix("tag:")
			&& let Some(tagged) = parse_tag(value, registry)
			&& !tags.iter().any(|found: &Tagged| found.name == tagged.name)
			&& tags.len() < MAX_TAGS
		{
			tags.push(tagged);
		}
	}
	(category, tags)
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

fn needs_classification(entry: Option<&Entry>, registry: &tags::Registry, force: bool) -> bool {
	force
		|| entry.is_none_or(|entry| {
			entry.category.is_none()
				|| entry.tags.is_empty()
				|| entry
					.tags
					.iter()
					.any(|tag| !registry.tags.contains_key(tag))
		})
}

fn insert_new_tag(registry: &mut tags::Registry, tagged: &Tagged, creator: &Translation) -> bool {
	if registry.tags.contains_key(&tagged.name) {
		return false;
	}
	let mut tag = tagged.tag.clone();
	if let Tag::Ordinary {
		source, display, ..
	} = &mut tag
	{
		let mut english = creator.clone();
		english.text.clone_from(source);
		display.insert(SOURCE_LOCALE.to_owned(), english);
	}
	registry.tags.insert(tagged.name.clone(), tag);
	true
}

/// Classify every asset missing either its own labels or the registry records behind them.
pub async fn run(
	repo: &Path,
	runner: Runner,
	force: bool,
	limit: Option<usize>,
) -> std::io::Result<Outcome> {
	let merged = crate::image::run::load(&repo.join(crate::image::run::MERGED))?;
	let described_path = media::path_for(repo);
	let mut described = media::load(&described_path);
	let registry_path = tags::path_for(repo);
	let mut registry = tags::load(&registry_path);

	let wanted: Vec<String> = merged
		.media
		.keys()
		.filter(|cid| needs_classification(described.media.get(*cid), &registry, force))
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
	let progress = crate::task::progress::Progress::new_terminal(todo.len() as u64);
	for (cid, path) in todo {
		progress.set_message(
			path
				.file_name()
				.map(|n| n.to_string_lossy().chars().take(40).collect::<String>())
				.unwrap_or_default(),
		);

		let text = prompt(&path, &tags::known(&registry));
		let at = crate::image::manifest::now();
		let clock = std::time::Instant::now();
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
		let seconds = clock.elapsed().as_secs_f64();

		let (category, found) = parse(&answer.text, &registry);
		if found.len() < MIN_TAGS {
			outcome
				.failed
				.push((cid, format!("only {} usable tags", found.len())));
			progress.inc(1);
			continue;
		}

		let creator = Translation {
			text: String::new(),
			provider: runner.provider().to_owned(),
			model: answer.model.clone(),
			at,
			seconds,
			tokens: answer.tokens,
			review: false,
		};
		for tagged in &found {
			if insert_new_tag(&mut registry, tagged, &creator) {
				outcome.minted.push(tagged.name.clone());
			}
		}

		let entry = described.media.entry(cid).or_insert_with(Entry::default);
		entry.category = category;
		entry.tags = found.into_iter().map(|tagged| tagged.name).collect();
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

	fn registry() -> tags::Registry {
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"terminal".to_owned(),
			Tag::ordinary("Terminal", "terminal emulator or command-line window"),
		);
		registry.tags.insert(
			"rust".to_owned(),
			Tag::Technical {
				display: "Rust".to_owned(),
				meaning: "programming language".to_owned(),
			},
		);
		registry
	}

	#[test]
	fn existing_and_new_tags_are_read_off_independent_lines() {
		let registry = registry();
		let (category, tags) = parse(
			"category: screenshot\n\
			 tag: terminal\n\
			 tag: cargo|technical|Cargo|Rust package manager and build tool\n\
			 tag: cellular-network|ordinary|Cellular Network|mobile carrier connectivity and SIM service, not biology\n",
			&registry,
		);
		assert_eq!(category, Some(Category::Screenshot));
		assert_eq!(
			tags,
			vec![
				Tagged {
					name: "terminal".to_owned(),
					tag: registry.tags["terminal"].clone(),
				},
				Tagged {
					name: "cargo".to_owned(),
					tag: Tag::Technical {
						display: "Cargo".to_owned(),
						meaning: "Rust package manager and build tool".to_owned(),
					},
				},
				Tagged {
					name: "cellular-network".to_owned(),
					tag: Tag::ordinary(
						"Cellular Network",
						"mobile carrier connectivity and SIM service, not biology",
					),
				},
			]
		);
	}

	#[test]
	fn a_malformed_tag_is_dropped_and_not_repaired() {
		// Replacing the space would invent `shell-terminal`, a name nobody chose and which now
		// competes with `terminal` forever.
		let registry = registry();
		let (_, tags) = parse(
			"category: screenshot\n\
			 tag: terminal\n\
			 tag: shell terminal|ordinary|Shell Terminal|command-line window\n\
			 tag: TypeScript|technical|TypeScript|programming language\n\
			 tag: unknown\n\
			 tag: cargo|technical|Cargo|",
			&registry,
		);
		assert_eq!(
			tags
				.iter()
				.map(|tagged| tagged.name.as_str())
				.collect::<Vec<_>>(),
			vec!["terminal"]
		);
	}

	#[test]
	fn a_bad_category_leaves_the_field_empty_rather_than_guessing() {
		let (category, tags) = parse(
			"category: terminal\n\
			 tag: a|ordinary|A|concept a\n\
			 tag: b|ordinary|B|concept b\n\
			 tag: c|ordinary|C|concept c",
			&tags::Registry::default(),
		);
		assert_eq!(category, None);
		// The tags still land: one malformed line costs one field.
		assert_eq!(tags.len(), 3);
	}

	#[test]
	fn the_budget_holds_at_the_top() {
		let (_, tags) = parse(
			"tag: a|ordinary|A|a\n\
			 tag: b|ordinary|B|b\n\
			 tag: c|ordinary|C|c\n\
			 tag: d|ordinary|D|d\n\
			 tag: e|ordinary|E|e\n\
			 tag: f|ordinary|F|f\n\
			 tag: g|ordinary|G|g\n\
			 tag: h|ordinary|H|h",
			&tags::Registry::default(),
		);
		assert_eq!(tags.len(), MAX_TAGS);
	}

	#[test]
	fn the_prompt_shows_the_model_what_already_exists() {
		// A raw name alone cannot tell the model whether `mold` is fungus or a linker. Existing
		// meanings make reuse a concept decision rather than a spelling decision.
		let with = prompt(Path::new("/tmp/a.png"), &tags::known(&registry()));
		assert!(with.contains("terminal | ordinary | source: Terminal"));
		assert!(with.contains("meaning: programming language"));
		assert!(with.contains("Reuse a raw identifier only when that meaning matches"));
		assert!(with.contains("cellular-network"));
		assert!(with.contains("mold-linker"));
		assert!(with.contains("official casing"));
		assert!(with.contains("Title Case"));

		let without = prompt(Path::new("/tmp/a.png"), &[]);
		assert!(without.contains("no tags yet"));
	}

	#[test]
	fn a_missing_registry_entry_puts_an_already_tagged_asset_back_in_the_queue() {
		let entry = Entry {
			category: Some(Category::Screenshot),
			tags: vec!["terminal".to_owned()],
			..Entry::default()
		};
		assert!(needs_classification(
			Some(&entry),
			&tags::Registry::default(),
			false
		));
		assert!(!needs_classification(Some(&entry), &registry(), false));
	}

	#[test]
	fn an_ordinary_tag_keeps_the_creating_answers_provenance_for_english() {
		let mut registry = tags::Registry::default();
		let tagged = Tagged {
			name: "cellular-network".to_owned(),
			tag: Tag::ordinary(
				"Cellular Network",
				"mobile carrier connectivity and SIM service, not biology",
			),
		};
		let creator = Translation {
			text: String::new(),
			provider: "openai".to_owned(),
			model: "gpt-5-6-terra-medium".to_owned(),
			at: "2026-08-01T00:00:00Z".to_owned(),
			seconds: 1.25,
			tokens: 42,
			review: false,
		};

		assert!(insert_new_tag(&mut registry, &tagged, &creator));
		let english = &registry.tags["cellular-network"]
			.translations()
			.expect("ordinary")[SOURCE_LOCALE];
		assert_eq!(english.text, "Cellular Network");
		assert_eq!(english.provider, creator.provider);
		assert_eq!(english.model, creator.model);
		assert_eq!(english.at, creator.at);
		assert_eq!(english.seconds, creator.seconds);
		assert_eq!(english.tokens, creator.tokens);
		assert!(!english.review);
		assert!(!insert_new_tag(&mut registry, &tagged, &creator));
	}
}
