//! The `cms alt` command: describing an image for someone who cannot see it.
//!
//! The description belongs to the asset, not to any article referencing it, so it is written
//! into the manifest once and every reference inherits it. See spec/architecture.md.
//!
//! Work is handed to the local `claude` CLI rather than to the API. That binary is a whole
//! agent with a Read tool of its own, so naming a path in the prompt is enough -- there is no
//! multimodal request to assemble, no image to base64, and no key to hold. It is slower and
//! dearer per call than the raw API, neither of which matters for a batch that runs once per
//! imported picture.

use crate::image::manifest::{Media, Merged};
use claude_codes::{AsyncClient, ClaudeModel, ClaudeOutput, cli::ClaudeCliBuilder};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many descriptions are in flight at once.
///
/// Each call is minutes of somebody else's compute and seconds of wall clock, so the limit is
/// about politeness and rate limits rather than local resources. Four keeps a batch of two
/// dozen under a couple of minutes without arriving as a burst.
pub const PARALLEL: usize = 4;

/// Sonnet: this is description, not reasoning, and the ceiling on quality here is how
/// carefully the picture is read rather than how hard it is thought about.
const MODEL: ClaudeModel = ClaudeModel::Sonnet;

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

#[derive(Debug, Default)]
pub struct Outcome {
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
	originals: &Path,
	force: bool,
) -> (Vec<(String, PathBuf)>, Vec<String>) {
	let wanted: Vec<&String> = merged
		.assets
		.iter()
		.filter(|(_, media)| force || media.description.is_none())
		.map(|(cid, _)| cid)
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

/// Ask the CLI to describe one image.
async fn describe(path: &Path) -> Result<String, String> {
	// No prompt on the builder: the client speaks to the CLI over a stream, and the question
	// goes through `query`. Setting both would put the CLI in one-shot mode and leave the
	// stream waiting for an answer to a question it had already been given.
	let builder = ClaudeCliBuilder::new()
		.model(MODEL.cli_arg())
		// Reading a file is the entire job. Granting more would let a description turn into an
		// edit, and this command runs unattended over a whole library.
		.allowed_tools(["Read"]);

	let mut client = AsyncClient::from_builder(builder)
		.await
		.map_err(|error| error.to_string())?;
	let outputs = client
		.query(&prompt(path))
		.await
		.map_err(|error| error.to_string())?;
	let _ = client.shutdown().await;

	// The result message is the CLI's own verdict on the turn, so it is read instead of the
	// assistant text: it says whether the run failed, which prose never would.
	let result = outputs.iter().find_map(|output| match output {
		ClaudeOutput::Result(message) => Some(message),
		_ => None,
	});
	let Some(message) = result else {
		return Err("the CLI ended without a result".to_owned());
	};
	if message.is_error {
		return Err(format!("{:?}", message.subtype));
	}

	let text = message.result.clone().unwrap_or_default().trim().to_owned();
	if text.is_empty() {
		return Err("the model returned nothing".to_owned());
	}
	Ok(text)
}

/// Describe every asset that has no description yet, and record what came back.
pub async fn run(
	merged: &mut Merged,
	originals: &Path,
	force: bool,
	limit: Option<usize>,
) -> Outcome {
	let (mut todo, unreadable) = pending(merged, originals, force);
	let wanted = todo.len();
	// Each call costs real money, so a whole library should be something asked for rather than
	// the only option. Trying two first is how you find out the prompt is wrong for cheap.
	if let Some(limit) = limit {
		todo.truncate(limit);
	}
	let mut outcome = Outcome {
		skipped: merged.assets.len() - wanted - unreadable.len(),
		deferred: wanted - todo.len(),
		unreadable,
		..Outcome::default()
	};

	// Bounded rather than unbounded: the point of the limit is that it holds.
	let mut queue = todo.into_iter();
	let mut running = Vec::new();
	let mut results: Vec<(String, Result<String, String>)> = Vec::new();

	loop {
		while running.len() < PARALLEL {
			let Some((cid, path)) = queue.next() else {
				break;
			};
			running.push(tokio::spawn(async move { (cid, describe(&path).await) }));
		}
		if running.is_empty() {
			break;
		}
		// One failure must not abandon the rest; a batch over a library should record what it
		// managed and report the gaps.
		let finished = running.remove(0);
		match finished.await {
			Ok(result) => results.push(result),
			Err(error) => results.push((String::new(), Err(error.to_string()))),
		}
	}

	for (cid, result) in results {
		match result {
			Ok(text) => {
				if let Some(media) = merged.assets.get_mut(&cid) {
					media.description = Some(text);
					media.updated = crate::image::manifest::now();
					outcome.described += 1;
				}
			}
			Err(error) => outcome.failed.push((cid, error)),
		}
	}
	outcome
}

/// Whether this asset still wants a description.
pub fn wants_description(media: &Media) -> bool {
	media.description.is_none()
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
		let mut merged = Merged {
			version: crate::image::manifest::VERSION,
			generated: crate::image::manifest::now(),
			assets: BTreeMap::new(),
		};
		let mut media = crate::image::manifest::media_for(
			&crate::image::Derived {
				cid: "a".into(),
				width: 1,
				height: 1,
				thumb: Vec::new(),
				preview: Vec::new(),
				variants: Vec::new(),
			},
			"image/png",
			1,
			None,
			false,
		);
		assert!(wants_description(&media));
		media.description = Some("a thing".into());
		assert!(!wants_description(&media));

		merged.assets.insert("a".into(), media);
		// Nothing pending, so the originals directory is never even read.
		let (todo, missing) = pending(&merged, Path::new("/nowhere"), false);
		assert!(todo.is_empty());
		assert!(missing.is_empty());
	}
}
