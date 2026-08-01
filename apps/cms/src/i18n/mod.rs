//! The `cms i18n` command: translating what the articles say.
//!
//! Segment by segment, all locales at once, because a paragraph edited on its own should cost
//! one request and update every language together. See spec/architecture.md.

pub mod model;
pub mod prompt;
pub mod segment;
pub mod store;

use claude_codes::{AsyncClient, ClaudeModel, ClaudeOutput, cli::ClaudeCliBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use segment::{Kind, Segment};
use std::path::Path;
use store::Translation;

/// Requests in flight. The same reasoning as `cms alt`: politeness rather than local limits.
pub const PARALLEL: usize = 4;

/// How many times a segment is asked for again before it is reported as failed.
///
/// A retry escalates the model, which is the one difficulty signal here that is measured
/// rather than guessed: the light model failing is an event, where a difficulty score would
/// only ever have been an estimate.
const ATTEMPTS: usize = 3;

#[derive(Debug, Default)]
pub struct Outcome {
	pub translated: usize,
	pub segments: usize,
	pub tokens: u64,
	pub usd: f64,
	pub failed: Vec<(String, String)>,
	pub orphans: usize,
}

fn model_for(kind: Kind, attempt: usize) -> ClaudeModel {
	match (kind.is_light(), attempt) {
		// Headings and directive attributes: structural text, no register to lose.
		(true, 0) => ClaudeModel::Haiku,
		(_, 0 | 1) => ClaudeModel::Sonnet,
		// Only after the answer has already failed validation twice.
		_ => ClaudeModel::Opus,
	}
}

struct Answer {
	locales: Vec<(String, String)>,
	model: String,
	tokens: u64,
	usd: f64,
}

async fn ask(text: &str, model: ClaudeModel) -> Result<Answer, String> {
	let builder = ClaudeCliBuilder::new()
		.model(model.cli_arg())
		// Nothing is read and nothing is written; the whole task is in the prompt.
		.allowed_tools(Vec::<String>::new());

	let mut client = AsyncClient::from_builder(builder)
		.await
		.map_err(|error| error.to_string())?;
	let outputs = client
		.query(text)
		.await
		.map_err(|error| error.to_string())?;
	let _ = client.shutdown().await;

	let result = outputs
		.iter()
		.find_map(|output| match output {
			ClaudeOutput::Result(message) => Some(message),
			_ => None,
		})
		.ok_or("the CLI ended without a result")?;
	if result.is_error {
		return Err(format!("{:?}", result.subtype));
	}

	let reply = result.result.clone().unwrap_or_default();
	let usage = result.usage.as_ref();
	Ok(Answer {
		locales: prompt::parse(&reply),
		// Read from the envelope rather than asked for: what actually ran is a runtime fact,
		// and a model's account of itself is not.
		model: result
			.model_usage
			.as_ref()
			.and_then(|usage| usage.keys().next())
			.map(|name| model::normalise(name))
			.unwrap_or_else(|| model::normalise(model.cli_arg())),
		tokens: usage.map_or(0, |u| {
			u64::from(u.input_tokens)
				+ u64::from(u.output_tokens)
				+ u64::from(u.cache_read_input_tokens)
				+ u64::from(u.cache_creation_input_tokens)
		}),
		usd: result.total_cost_usd,
	})
}

/// Translate one segment into every locale it is missing.
async fn translate(
	item: &Segment,
	before: Option<String>,
	after: Option<String>,
) -> Result<(Vec<(String, Translation)>, u64, f64), String> {
	let masked = segment::mask(&item.source);
	let started = crate::image::manifest::now();
	let clock = std::time::Instant::now();
	let mut last = String::new();

	for attempt in 0..ATTEMPTS {
		let text = prompt::build(item, &masked.text, before.as_deref(), after.as_deref());
		let answer = match ask(&text, model_for(item.kind, attempt)).await {
			Ok(answer) => answer,
			Err(error) => {
				last = error;
				continue;
			}
		};

		// Every marker back exactly once, or the answer is not usable. This is the point of
		// masking: not hoping the model left the code alone, but being able to show it did.
		let kept: Vec<(String, String)> = answer
			.locales
			.into_iter()
			.filter(|(_, text)| masked.intact(text))
			.map(|(locale, text)| (locale, masked.restore(&text)))
			.collect();

		if kept.is_empty() {
			last = "no locale survived marker validation".to_owned();
			continue;
		}

		let provider = model::Provider::of_model(&answer.model)
			.map(|p| p.as_str().to_owned())
			.unwrap_or_default();
		let seconds = clock.elapsed().as_secs_f64();
		let entries = kept
			.into_iter()
			.map(|(locale, text)| {
				(
					locale,
					Translation {
						text,
						provider: provider.clone(),
						model: answer.model.clone(),
						at: started.clone(),
						seconds,
						tokens: answer.tokens,
						review: false,
					},
				)
			})
			.collect();
		return Ok((entries, answer.tokens, answer.usd));
	}
	Err(last)
}

/// One line, rewritten in place, showing what is being worked on.
fn bar(total: u64) -> ProgressBar {
	let bar = ProgressBar::new(total);
	bar.set_style(
		ProgressStyle::with_template("  {bar:28} {pos}/{len}  {wide_msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar()),
	);
	bar
}

/// A first line of a segment, short enough to sit on one terminal row.
fn preview(source: &str) -> String {
	let line = source.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
	let flat = line.split_whitespace().collect::<Vec<_>>().join(" ");
	let mut out = String::new();
	for c in flat.chars() {
		// Counted in display width, roughly: a CJK glyph occupies two columns, so a character
		// count would overflow the row on exactly the articles this site publishes.
		let width = if (c as u32) > 0x2e80 { 2 } else { 1 };
		if out
			.chars()
			.map(|c| if (c as u32) > 0x2e80 { 2 } else { 1 })
			.sum::<usize>()
			+ width
			> 44
		{
			out.push('…');
			break;
		}
		out.push(c);
	}
	out
}

/// Translate every article under `articles`.
pub async fn run(
	articles: &Path,
	only: &[std::path::PathBuf],
	limit: Option<usize>,
	force: bool,
) -> std::io::Result<Outcome> {
	let mut outcome = Outcome::default();
	let mut budget = limit.unwrap_or(usize::MAX);

	for path in crate::refs::markdown_under(articles)? {
		// Named articles narrow the run. Retranslating one edited piece should not mean walking
		// everything before it in the tree.
		if !only.is_empty()
			&& !only
				.iter()
				.any(|wanted| path.ends_with(wanted) || &path == wanted)
		{
			continue;
		}
		if budget == 0 {
			break;
		}
		let article = std::fs::read_to_string(&path)?;
		let live = segment::translatable(&article);
		let sidecar_path = store::path_for(&path);
		let mut sidecar = store::load(&sidecar_path);
		outcome.segments += live.len();
		outcome.orphans += store::orphans(&sidecar, &live).len();

		let wanted = if force {
			live.keys().cloned().collect::<Vec<_>>()
		} else {
			store::missing(&sidecar, &live, &prompt::LOCALES)
				.into_keys()
				.collect()
		};
		if wanted.is_empty() {
			continue;
		}

		// Order for context comes from the article, which is the only place it lives.
		let ordered: Vec<&Segment> = {
			let mut all: Vec<&Segment> = live.values().collect();
			all.sort_by_key(|s| s.line);
			all
		};
		let neighbours = |id: &str| {
			let at = ordered.iter().position(|s| s.id == id);
			at.map_or((None, None), |at| {
				(
					at.checked_sub(1)
						.and_then(|i| ordered.get(i))
						.map(|s| s.source.clone()),
					ordered.get(at + 1).map(|s| s.source.clone()),
				)
			})
		};

		let todo: Vec<&Segment> = wanted
			.iter()
			.filter_map(|id| live.get(id))
			.take(budget)
			.collect();
		budget -= todo.len();

		let progress = bar(todo.len() as u64);
		progress.set_message(format!("{}", path.display()));

		let mut queue = todo.into_iter();
		let mut running: Vec<tokio::task::JoinHandle<(String, Result<_, String>)>> = Vec::new();

		loop {
			while running.len() < PARALLEL {
				let Some(item) = queue.next() else {
					break;
				};
				progress.set_message(preview(&item.source));
				let (before, after) = neighbours(&item.id);
				let owned = item.clone();
				running.push(tokio::spawn(async move {
					let id = owned.id.clone();
					(id, translate(&owned, before, after).await)
				}));
			}
			if running.is_empty() {
				break;
			}

			let finished = match running.remove(0).await {
				Ok(result) => result,
				Err(error) => (String::new(), Err(error.to_string())),
			};
			progress.inc(1);

			let (id, result) = finished;
			match result {
				Ok((entries, tokens, usd)) => {
					let slot = sidecar.segments.entry(id).or_default();
					for (locale, entry) in entries {
						slot.insert(locale, entry);
						outcome.translated += 1;
					}
					outcome.tokens += tokens;
					outcome.usd += usd;
					// Written the moment it arrives. Every segment cost real money, and keeping a
					// run's worth in memory means one interrupt throws all of it away -- which is
					// exactly what happened the first time this ran for real.
					sidecar.version = store::VERSION;
					store::save(&sidecar_path, &sidecar)?;
				}
				Err(error) => outcome.failed.push((id, error)),
			}
		}
		progress.finish_and_clear();
	}
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_light_model_is_only_used_where_structure_says_it_is_safe() {
		assert_eq!(model_for(Kind::Heading, 0).cli_arg(), "haiku");
		assert_eq!(model_for(Kind::Prose, 0).cli_arg(), "sonnet");
		assert_eq!(model_for(Kind::Quote, 0).cli_arg(), "sonnet");
	}

	#[test]
	fn failing_twice_escalates_rather_than_repeating() {
		// The only difficulty signal worth acting on is one that actually happened.
		assert_eq!(model_for(Kind::Heading, 1).cli_arg(), "sonnet");
		assert_eq!(model_for(Kind::Prose, 2).cli_arg(), "opus");
	}

	#[test]
	fn the_status_line_is_measured_in_columns_not_characters() {
		// A CJK glyph is two columns wide, so counting characters overflows the row on exactly
		// the articles this site publishes.
		let cjk = preview(&"中".repeat(60));
		assert!(cjk.ends_with('…'));
		assert!(cjk.chars().count() <= 24);

		let latin = preview(&"a".repeat(60));
		assert!(latin.chars().count() > 24);
	}

	#[test]
	fn the_status_line_collapses_a_block_to_one_row() {
		assert_eq!(preview("\n\nfirst line\nsecond line"), "first line");
	}
}
