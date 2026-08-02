//! The `cms alt` command: describing an image for someone who cannot see it.
//!
//! The description belongs to the asset, not to any article referencing it, so it is written
//! into the manifest once and every reference inherits it. See spec/architecture.md.
//!
//! Work is handed to a local agent CLI rather than to an API. The runner either attaches the
//! image or reads the named path itself, so there is no multimodal request to assemble, no image
//! to base64, and no key to hold.

use crate::i18n::runner::{self, Refusal, Runner};
use crate::image::manifest::Merged;
use crate::media::{self, Entry};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many descriptions are in flight at once.
///
/// Each call is minutes of somebody else's compute and seconds of wall clock, so the limit is
/// about politeness and rate limits rather than local resources. Four keeps a batch of two
/// dozen under a couple of minutes without arriving as a burst.
pub const PARALLEL: usize = 4;

/// The locale a generated description is written in.
///
/// One language out of the model, then translated like anything else. Asking for eight at
/// once would mean eight looks at the same picture, and the picture does not change.
const SOURCE_LOCALE: &str = "en-US";

/// What the model is asked for.
///
/// The framing is the whole instruction. "Describe this image" produces a caption -- a label
/// naming the subject. Asking for what a person who cannot see it would need produces the
/// thing that is actually useful: what kind of image it is, what it contains, and what it is
/// evidently for.
fn prompt(path: &Path) -> String {
	format!(
		"Read the image at {} and describe it for someone who cannot see it.\n\n\
		 Say what kind of image it is first -- a screenshot, a photograph, a diagram, a chart, \
		 a code sample -- because that frames everything after it. Then give the content: for \
		 a screenshot or a chart, what the interface or the data actually says, including \
		 figures and labels that carry meaning; for a photograph, the subject, the setting and \
		 the light; for a diagram, what connects to what and in which direction. Say what the \
		 image appears to be evidence of, where that is clear.\n\n\
		 Two to four sentences. Write it as flowing prose, not a list. Do not open with \
		 \"An image of\" or \"This picture shows\" -- start with the content. Reply with the \
		 description alone: no preamble, no quotes, no markdown.",
		path.display()
	)
}

/// What one call spent. Summed across a batch so a run can be priced afterwards.
#[derive(Debug, Default, Clone, Copy)]
pub struct Spend {
	pub input: u64,
	pub output: u64,
	pub cache_read: u64,
	pub cache_written: u64,
	pub usd: f64,
}

impl Spend {
	fn add(&mut self, other: Spend) {
		self.input += other.input;
		self.output += other.output;
		self.cache_read += other.cache_read;
		self.cache_written += other.cache_written;
		self.usd += other.usd;
	}

	/// Everything the model was shown, however it was billed.
	pub fn total_in(self) -> u64 {
		self.input + self.cache_read + self.cache_written
	}
}

#[derive(Debug, Default)]
pub struct Outcome {
	pub spent: Spend,
	pub described: usize,
	/// Assets that already had a description and were not asked about again.
	pub skipped: usize,
	/// Assets that still want one but were held back by `--limit`.
	///
	/// Counted apart from `skipped` because the two mean opposite things: one is work already
	/// done, the other is work still owed. Reporting them together would say a library was
	/// finished when it had barely started.
	pub deferred: usize,
	pub failed: Vec<(String, String)>,
	/// Assets with no original on hand, which cannot be looked at.
	pub unreadable: Vec<String>,
}

/// Which assets still need describing, paired with the original to look at.
///
/// The originals are matched by hashing rather than by filename: the id *is* the hash, and
/// `data/image` holds whatever names the files arrived under.
fn pending(
	merged: &Merged,
	described: &media::Media,
	originals: &Path,
	force: bool,
) -> (Vec<(String, PathBuf)>, Vec<String>) {
	let wanted: Vec<&String> = merged
		.media
		.keys()
		.filter(|cid| {
			force
				|| described
					.media
					.get(*cid)
					.is_none_or(|entry| entry.description.is_empty())
		})
		.collect();
	if wanted.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let by_id = originals_by_id(originals);
	let mut found = Vec::new();
	let mut missing = Vec::new();
	for cid in wanted {
		match by_id.get(cid) {
			Some(path) => found.push((cid.clone(), path.clone())),
			None => missing.push(cid.clone()),
		}
	}
	(found, missing)
}

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

/// Ask an agent to describe one image.
///
/// Through the shared runner, so `--model` picks who answers and the same allowance handling
/// applies here as everywhere else. The provider and model recorded afterwards come from what
/// actually ran rather than from a constant in this file.
async fn describe(runner: Runner, path: &Path) -> Result<(String, Spend, String), Refusal> {
	let answer = runner::ask_vision(runner, &prompt(path), runner.model_for_vision(), path).await?;
	let text = answer.text.trim().to_owned();
	if text.is_empty() {
		return Err(Refusal::Failed("the model returned nothing".to_owned()));
	}
	let spend = Spend {
		// The runner reports one total; the split by billing class is only available from
		// Claude's envelope, and inventing zeros for the rest would read as measurement.
		input: answer.tokens,
		output: 0,
		cache_read: 0,
		cache_written: 0,
		usd: answer.usd,
	};
	Ok((text, spend, answer.model))
}

/// Describe every asset that has no description yet, and record what came back.
pub async fn run(
	runner: Runner,
	merged: &Merged,
	described: &mut media::Media,
	originals: &Path,
	force: bool,
	limit: Option<usize>,
) -> Outcome {
	let (mut todo, unreadable) = pending(merged, described, originals, force);
	let wanted = todo.len();
	// Each call costs real money, so a whole library should be something asked for rather than
	// the only option. Trying two first is how you find out the prompt is wrong for cheap.
	if let Some(limit) = limit {
		todo.truncate(limit);
	}
	let mut outcome = Outcome {
		skipped: merged.media.len() - wanted - unreadable.len(),
		deferred: wanted - todo.len(),
		unreadable,
		..Outcome::default()
	};

	// A call takes tens of seconds and there is nothing to read while it does, so silence for
	// several minutes is indistinguishable from a hang.
	let bar = ProgressBar::new(todo.len() as u64);
	bar.set_style(
		ProgressStyle::with_template("  {bar:32} {pos}/{len}  {msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar()),
	);

	// Bounded rather than unbounded: the point of the limit is that it holds.
	let mut queue = todo.into_iter();
	let mut running = Vec::new();
	type Finished = (String, Result<(String, Spend, String), Refusal>);
	let mut results: Vec<Finished> = Vec::new();

	loop {
		while running.len() < PARALLEL {
			let Some((cid, path)) = queue.next() else {
				break;
			};
			running.push(tokio::spawn(
				async move { (cid, describe(runner, &path).await) },
			));
		}
		if running.is_empty() {
			break;
		}
		// One failure must not abandon the rest; a batch over a library should record what it
		// managed and report the gaps.
		let finished = running.remove(0);
		match finished.await {
			Ok(result) => results.push(result),
			Err(error) => results.push((String::new(), Err(Refusal::Failed(error.to_string())))),
		}
		bar.inc(1);
	}
	bar.finish_and_clear();

	for (cid, result) in results {
		match result {
			Ok((text, spend, model)) => {
				outcome.spent.add(spend);
				let entry = described.media.entry(cid).or_insert_with(Entry::default);
				// Written under the source locale the article is authored in. `cms i18n` fills
				// the rest from here, through the same pipeline a paragraph goes through.
				entry.description.insert(
					SOURCE_LOCALE.to_owned(),
					crate::i18n::store::Translation {
						text,
						provider: runner.provider().to_owned(),
						model,
						at: crate::image::manifest::now(),
						seconds: 0.0,
						tokens: spend.total_in() + spend.output,
						review: false,
					},
				);
				outcome.described += 1;
			}
			Err(error) => outcome.failed.push((cid, error.to_string())),
		}
	}
	outcome
}

/// Whether this asset still wants a description.
pub fn wants_description(described: &media::Media, cid: &str) -> bool {
	described
		.media
		.get(cid)
		.is_none_or(|entry| entry.description.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_prompt_names_the_file_and_asks_for_prose() {
		let text = prompt(Path::new("/tmp/a.png"));
		assert!(text.contains("/tmp/a.png"));
		// The framing is what separates a description from a caption, so it is worth a test:
		// losing this line would silently downgrade every alt written afterwards.
		assert!(text.contains("cannot see it"));
		assert!(text.contains("not a list"));
	}

	#[test]
	fn an_asset_with_a_description_is_not_pending() {
		let merged = Merged {
			version: crate::image::manifest::VERSION,
			created: crate::image::manifest::now(),
			updated: crate::image::manifest::now(),
			media: BTreeMap::from([(
				"a".to_owned(),
				crate::image::manifest::media_for(
					&crate::image::Derived {
						cid: "a".into(),
						width: 1,
						height: 1,
						thumb: Vec::new(),
						variants: Vec::new(),
					},
					"image/png",
					1,
					None,
					None,
				),
			)]),
		};

		// The manifest no longer carries this. Whether an asset has been described is a
		// question for media.yaml, which is the point of them being separate files.
		let mut described = crate::media::Media::default();
		assert!(wants_description(&described, "a"));

		described.media.insert(
			"a".to_owned(),
			crate::media::Entry {
				description: BTreeMap::from([(
					"en-US".to_owned(),
					crate::i18n::store::Translation {
						text: "a thing".into(),
						provider: "anthropic".into(),
						model: "claude-sonnet-5".into(),
						at: "2026-08-01T00:00:00Z".into(),
						seconds: 0.0,
						tokens: 0,
						review: false,
					},
				)]),
				..crate::media::Entry::default()
			},
		);
		assert!(!wants_description(&described, "a"));

		// Nothing pending, so the originals directory is never even read.
		let (todo, missing) = pending(&merged, &described, Path::new("/nowhere"), false);
		assert!(todo.is_empty());
		assert!(missing.is_empty());
	}
}
