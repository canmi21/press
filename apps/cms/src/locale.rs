//! The `cms locale` command: translating tag labels and image descriptions.
//!
//! These are short plain strings, not article blocks. An ordinary tag asks for every missing
//! non-source locale at once; descriptions remain one request per locale. Each unit is saved
//! as soon as it returns, so an interrupted run keeps every answer it paid for.

use crate::alt::SOURCE_LOCALE;
use crate::i18n::runner::{self, Answer, Refusal, Runner};
use crate::i18n::segment::Kind;
use crate::i18n::store::Translation;
use crate::task::{Record, claim, progress, registry as task_registry, writer};
use crate::{media, tags};
use std::future::Future;
use std::path::Path;

const ATTEMPTS: usize = 3;
const BACKOFF_START: std::time::Duration = std::time::Duration::from_secs(5);
const BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug, Default)]
pub struct Outcome {
	pub translated: usize,
	pub sources: usize,
	pub skipped: usize,
	pub deferred: usize,
	pub tokens: u64,
	pub usd: f64,
	pub failed: Vec<(String, String)>,
	/// Values another run holds a claim on, left to it rather than translated twice.
	pub claimed_elsewhere: usize,
	pub exhausted: Option<String>,
}

#[derive(Debug, Clone)]
enum Destination {
	Tag(String),
	Description(String),
	/// An article's summary, addressed by the path of the article it belongs to.
	Summary(std::path::PathBuf),
}

#[derive(Debug, Clone)]
struct Item {
	destination: Destination,
	source: String,
	/// The locale `source` is written in. `en-US` for everything a vision model produced;
	/// an article's own language for a summary, which is written where the article was.
	source_locale: String,
	meaning: Option<String>,
	locales: Vec<String>,
	kind: Kind,
}

impl Item {
	fn id(&self, locale: &str) -> String {
		match &self.destination {
			Destination::Tag(name) => format!("tag {name}/{locale}"),
			Destination::Description(cid) => format!("description {cid}/{locale}"),
			Destination::Summary(path) => format!("summary {}/{locale}", path.display()),
		}
	}
}

fn targets(
	translations: &std::collections::BTreeMap<String, Translation>,
	locales: &[&str],
	source_locale: &str,
	force: bool,
) -> (Vec<String>, usize) {
	let mut wanted = Vec::new();
	let mut skipped = 0;
	for locale in locales {
		// The source is input, never output. For an ordinary tag, `cms tag` records the English
		// label from the same vision answer that created it; even force must preserve that fact.
		// A summary's source is whichever locale the article is written in, so this is passed
		// rather than assumed.
		if *locale == source_locale {
			continue;
		}
		if !force && translations.contains_key(*locale) {
			skipped += 1;
		} else {
			wanted.push((*locale).to_owned());
		}
	}
	(wanted, skipped)
}

fn pending(
	registry: &tags::Registry,
	described: &media::Media,
	locales: &[&str],
	force: bool,
) -> (Vec<Item>, usize) {
	let mut items = Vec::new();
	let mut skipped = 0;

	// Tags deliberately come first. They are cheap enough to prove the runner and persistence
	// path before the longer descriptions spend real money.
	for (name, tag) in &registry.tags {
		let Some((source, meaning)) = tag.translation_source() else {
			continue;
		};
		let display = tag.translations().expect("ordinary tag has translations");
		let (wanted, already) = targets(display, locales, SOURCE_LOCALE, force);
		skipped += already;
		if !wanted.is_empty() {
			items.push(Item {
				destination: Destination::Tag(name.clone()),
				source_locale: SOURCE_LOCALE.to_owned(),
				source: source.to_owned(),
				meaning: Some(meaning.to_owned()),
				locales: wanted,
				kind: Kind::Heading,
			});
		}
	}

	for (cid, entry) in &described.media {
		let Some(source) = entry.description.get(SOURCE_LOCALE) else {
			continue;
		};
		let (wanted, already) = targets(&entry.description, locales, SOURCE_LOCALE, force);
		skipped += already;
		if !wanted.is_empty() {
			items.push(Item {
				destination: Destination::Description(cid.clone()),
				source_locale: SOURCE_LOCALE.to_owned(),
				source: source.text.clone(),
				meaning: None,
				locales: wanted,
				kind: Kind::Prose,
			});
		}
	}
	(items, skipped)
}

/// Every article summary that has a source but not yet every translation.
///
/// Walks the sidecars rather than the articles: a summary the command has never been asked to
/// write simply has no file, and an article is not evidence that one is owed.
fn pending_summaries(
	contents: &Path,
	locales: &[&str],
	force: bool,
) -> std::io::Result<(Vec<Item>, usize)> {
	let mut items = Vec::new();
	let mut skipped = 0;
	let mut stack = vec![contents.to_path_buf()];
	while let Some(dir) = stack.pop() {
		let Ok(entries) = std::fs::read_dir(&dir) else {
			continue;
		};
		for entry in entries.flatten() {
			let path = entry.path();
			if path.is_dir() {
				stack.push(path);
				continue;
			}
			if !path.to_string_lossy().ends_with(".summary.yaml") {
				continue;
			}
			// Which locale is the source is not a property of the sidecar -- every entry in it
			// has the same shape. It is the article's own language, so the article is what
			// answers, reached back from the sidecar's own name.
			let article = path.with_extension("").with_extension("md");
			let Some(source_locale) = std::fs::read_to_string(&article)
				.ok()
				.and_then(|source| crate::summary::lang_of(&source))
				.and_then(|lang| crate::summary::source_locale(&lang))
			else {
				continue;
			};
			let sidecar = crate::summary::load(&path)?;
			let Some(source) = sidecar
				.summary
				.get(source_locale)
				.filter(|entry| !entry.text.trim().is_empty())
			else {
				continue;
			};
			let (wanted, already) = targets(&sidecar.summary, locales, source_locale, force);
			skipped += already;
			if !wanted.is_empty() {
				items.push(Item {
					destination: Destination::Summary(path.clone()),
					source: source.text.clone(),
					source_locale: source_locale.to_owned(),
					meaning: None,
					locales: wanted,
					kind: Kind::Prose,
				});
			}
		}
	}
	items.sort_by(|a, b| a.id("").cmp(&b.id("")));
	Ok((items, skipped))
}

fn tag_request(item: &Item) -> String {
	let name = match &item.destination {
		Destination::Tag(name) => name,
		Destination::Description(_) | Destination::Summary(_) => {
			unreachable!("a tag request needs a tag destination")
		}
	};
	let meaning = item.meaning.as_deref().unwrap_or_default();
	let markers = item
		.locales
		.iter()
		.map(|locale| crate::i18n::prompt::locale_marker(locale))
		.collect::<Vec<_>>()
		.join("\n");
	format!(
		"Translate one ordinary tag into every locale listed below. Translate the stated concept, \
		 not the raw identifier in isolation. Every answer is a short, ready-to-render standalone \
		 UI label, never an explanation or a sentence. Use the standard term a native reader would \
		 expect.\n\nCasing rules:\n- en-US uses Title Case for a short tag label.\n- de-DE \
		 follows normal German noun capitalisation.\n- fr-FR and es-ES capitalise the first word \
		 as a standalone label and otherwise follow native orthography.\n- Scripts without case use \
		 their natural written form.\n- Preserve conventional casing inside any established term; never \
		 apply mechanical title casing.\n\nAll locales must express the same meaning below. The English \
		 source label and meaning are authoritative; context in the raw identifier is only a stable \
		 key.\n\nOutput format, exactly: one marker line, then the short label, then a blank \
		 line.\n{markers}\n\nNothing else. No preamble, quotes, explanation, or markdown.\n\nRaw \
		 identifier: {name}\nEnglish source label: {}\nMeaning: {meaning}",
		item.source,
	)
}

/// Translating a summary, which is prose that was written to hold something back.
///
/// Said explicitly because a translator that "improves" it will complete the thought the
/// original deliberately left open, and the withholding is the whole point of the text.
fn summary_request(item: &Item, locale: &str) -> String {
	format!(
		"Translate this article summary from {} into {locale}. Keep its meaning, its register \
		 and its length. It deliberately describes what the article asks without giving away \
		 what the article concludes -- preserve that exactly; do not complete a thought it \
		 leaves open, and do not add detail it withholds. Reply with the translation alone: no \
		 preamble, quotes, explanation, or markdown.\n\n{}",
		item.source_locale, item.source
	)
}

fn description_request(item: &Item, locale: &str) -> String {
	format!(
		"Translate this short plain-text image description from {SOURCE_LOCALE} into {locale}. \
		 Preserve its meaning and factual detail. Reply with the translation alone: no preamble, \
		 quotes, explanation, or markdown.\n\n{}",
		item.source
	)
}

async fn translate_description<F, Fut>(
	runner: Runner,
	model_override: Option<&str>,
	item: &Item,
	locale: &str,
	ask: &mut F,
) -> Result<(Translation, u64, f64), Refusal>
where
	F: FnMut(Runner, String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	let mut attempt = 0usize;
	let mut backoff = BACKOFF_START;
	let mut last = Refusal::Failed(String::new());

	while attempt < ATTEMPTS {
		let prompt = match &item.destination {
			Destination::Summary(_) => summary_request(item, locale),
			_ => description_request(item, locale),
		};
		let model = model_override
			.unwrap_or_else(|| runner.model_for(item.kind, attempt))
			.to_owned();
		let at = crate::image::manifest::now();
		let clock = std::time::Instant::now();
		match ask(runner, prompt, model).await {
			Ok(answer) => {
				let text = answer.text.trim().to_owned();
				if text.is_empty() {
					last = Refusal::Failed("the model returned nothing".to_owned());
					attempt += 1;
					continue;
				}
				let tokens = answer.tokens;
				let usd = answer.usd;
				return Ok((
					Translation {
						text,
						provider: runner.provider().to_owned(),
						model: answer.model,
						at,
						seconds: clock.elapsed().as_secs_f64(),
						tokens,
						review: false,
					},
					tokens,
					usd,
				));
			}
			Err(Refusal::Exhausted(reason)) => return Err(Refusal::Exhausted(reason)),
			Err(Refusal::Throttled(_)) => {
				tokio::time::sleep(backoff).await;
				backoff = (backoff * 2).min(BACKOFF_MAX);
			}
			Err(error) => {
				last = error;
				attempt += 1;
			}
		}
	}
	Err(last)
}

async fn translate_tag<F, Fut>(
	runner: Runner,
	model_override: Option<&str>,
	item: &Item,
	ask: &mut F,
) -> Result<(Vec<(String, Translation)>, u64, f64), Refusal>
where
	F: FnMut(Runner, String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	let mut attempt = 0usize;
	let mut backoff = BACKOFF_START;
	let mut last = Refusal::Failed(String::new());

	while attempt < ATTEMPTS {
		let prompt = tag_request(item);
		let model = model_override
			.unwrap_or_else(|| runner.model_for(item.kind, attempt))
			.to_owned();
		let at = crate::image::manifest::now();
		let clock = std::time::Instant::now();
		match ask(runner, prompt, model).await {
			Ok(answer) => {
				let wanted = &item.locales;
				let found: Vec<(String, String)> = crate::i18n::prompt::parse(&answer.text, None)
					.unwrap_or_default()
					.into_iter()
					.filter(|(locale, _)| wanted.contains(locale))
					.collect();
				if found.is_empty() {
					last = Refusal::Failed("the model returned no requested locale".to_owned());
					attempt += 1;
					continue;
				}
				let provider = runner.provider().to_owned();
				let seconds = clock.elapsed().as_secs_f64();
				let entries = found
					.into_iter()
					.map(|(locale, text)| {
						(
							locale,
							Translation {
								text,
								provider: provider.clone(),
								model: answer.model.clone(),
								at: at.clone(),
								seconds,
								tokens: answer.tokens,
								review: false,
							},
						)
					})
					.collect();
				return Ok((entries, answer.tokens, answer.usd));
			}
			Err(Refusal::Exhausted(reason)) => return Err(Refusal::Exhausted(reason)),
			Err(Refusal::Throttled(_)) => {
				tokio::time::sleep(backoff).await;
				backoff = (backoff * 2).min(BACKOFF_MAX);
			}
			Err(error) => {
				last = error;
				attempt += 1;
			}
		}
	}
	Err(last)
}

pub struct Options<'a> {
	pub repository: &'a Path,
	pub runner: Runner,
	pub model_override: Option<String>,
	pub force: bool,
	pub limit: Option<usize>,
	pub shell: task_registry::Shell,
	/// Where to report progress. The CLI passes a terminal bar; the desktop passes its own.
	pub sink: Box<dyn progress::Sink>,
}

pub async fn run(options: Options<'_>) -> std::io::Result<Outcome> {
	let Options {
		repository,
		runner,
		model_override,
		force,
		limit,
		shell,
		sink,
	} = options;
	run_with_model(
		repository,
		runner,
		model_override.as_deref(),
		force,
		limit,
		&crate::i18n::prompt::LOCALES,
		shell,
		sink,
		|runner, prompt, model| async move { runner::ask(runner, &prompt, &model).await },
	)
	.await
}

#[cfg(test)]
async fn run_with<F, Fut>(
	repo: &Path,
	runner: Runner,
	force: bool,
	limit: Option<usize>,
	locales: &[&str],
	ask: F,
) -> std::io::Result<Outcome>
where
	F: FnMut(Runner, String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	run_with_model(
		repo,
		runner,
		None,
		force,
		limit,
		locales,
		task_registry::Shell::Cli,
		Box::new(progress::Silent),
		ask,
	)
	.await
}

#[allow(clippy::too_many_arguments)]
async fn run_with_model<F, Fut>(
	repo: &Path,
	runner: Runner,
	model_override: Option<&str>,
	force: bool,
	limit: Option<usize>,
	locales: &[&str],
	shell: task_registry::Shell,
	sink: Box<dyn progress::Sink>,
	mut ask: F,
) -> std::io::Result<Outcome>
where
	F: FnMut(Runner, String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	let registry_path = tags::path_for(repo);
	let mut registry = tags::load(&registry_path)?;
	let described_path = media::path_for(repo);
	// Read once to plan against. Every write below goes through a writer that re-reads inside its
	// own lock, so this copy is never what gets saved.
	let described = media::load(&described_path)?;
	let (mut items, skipped) = pending(&registry, &described, locales, force);
	// Summaries ride the same queue: the backoff, the exhausted-allowance stop and the
	// save-per-answer rule are all already here, and a second loop would have to grow its own.
	let (summaries, summaries_skipped) = pending_summaries(&repo.join("contents"), locales, force)?;
	items.extend(summaries);
	let skipped = skipped + summaries_skipped;
	let wanted = items.len();
	if let Some(limit) = limit {
		items.truncate(limit);
	}

	let mut outcome = Outcome {
		sources: items.len(),
		skipped,
		deferred: wanted - items.len(),
		..Outcome::default()
	};
	let calls: usize = items
		.iter()
		.map(|item| match item.destination {
			Destination::Tag(_) => 1,
			Destination::Description(_) | Destination::Summary(_) => item.locales.len(),
		})
		.sum();
	let progress = crate::task::start(repo, "locale", shell, calls as u64, sink)?;

	// One record per destination and no answer touching two, so the ordering rule spec/tasks.md
	// gives for multi-record tasks has nothing to decide here.
	let tag_writer = writer::Writer::start(repo, Record::Tags)?;
	let media_writer = writer::Writer::start(repo, Record::Media)?;
	let summary_writer = writer::Writer::start(repo, Record::Summaries)?;

	for item in items {
		match &item.destination {
			Destination::Tag(name) => {
				progress.set_message(name.clone());
				// Claimed before the model is asked: a label another run is translating now is
				// left to it rather than paid for twice.
				let claimed = match claim::take(repo, "locale", &item.id("")) {
					Ok(claimed) => claimed,
					Err(claim::Denied::Taken(_)) => {
						outcome.claimed_elsewhere += 1;
						progress.inc(1);
						continue;
					}
					Err(claim::Denied::Io(error)) => return Err(error),
				};
				match translate_tag(runner, model_override, &item, &mut ask).await {
					Ok((entries, tokens, usd)) => {
						let Some(display) = registry
							.tags
							.get_mut(name)
							.and_then(tags::Tag::translations_mut)
						else {
							outcome.failed.push((
								format!("tag {name}"),
								"tag is no longer ordinary".to_owned(),
							));
							progress.inc(1);
							continue;
						};
						for (locale, translation) in entries {
							display.insert(locale, translation);
							outcome.translated += 1;
						}
						outcome.tokens += tokens;
						outcome.usd += usd;
						// One paid turn produced the whole tag, so one durable write commits it.
						// Re-read inside the writer: another run may have minted a tag since this
						// one started, and saving the copy read at the top would drop it.
						let path = registry_path.clone();
						let updated = registry.tags.get(name).cloned();
						let key = name.clone();
						if let Err(error) = tag_writer.apply(move || {
							let mut current = tags::load(&path)?;
							if let Some(tag) = updated {
								current.tags.insert(key, tag);
							}
							tags::save(&path, &current)
						}) {
							outcome
								.failed
								.push((format!("tag {name}"), error.to_string()));
						}
					}
					Err(Refusal::Exhausted(reason)) => {
						outcome.exhausted = Some(reason);
						progress.finish_and_clear();
						return Ok(outcome);
					}
					Err(error) => outcome
						.failed
						.push((format!("tag {name}"), error.to_string())),
				}
				drop(claimed);
				progress.inc(1);
			}
			Destination::Summary(path) => {
				for locale in &item.locales {
					progress.set_message(format!("{} {locale}", path.display()));
					let claimed = match claim::take(repo, "locale", &item.id(locale)) {
						Ok(claimed) => claimed,
						Err(claim::Denied::Taken(_)) => {
							outcome.claimed_elsewhere += 1;
							progress.inc(1);
							continue;
						}
						Err(claim::Denied::Io(error)) => return Err(error),
					};
					match translate_description(runner, model_override, &item, locale, &mut ask).await {
						Ok((translation, tokens, usd)) => {
							// Reloaded per answer rather than held open: an interrupted run has
							// to leave every translation it paid for on disk.
							let id = item.id(locale);
							let sidecar_path = path.clone();
							let locale = locale.clone();
							let applied = summary_writer.apply(move || {
								let mut sidecar = crate::summary::load(&sidecar_path)?;
								sidecar.version = crate::summary::VERSION;
								sidecar.summary.insert(locale, translation);
								let encoded = serde_yaml_ng::to_string(&sidecar).map_err(std::io::Error::other)?;
								std::fs::write(&sidecar_path, encoded)
							});
							if let Err(error) = applied {
								outcome.failed.push((id, error.to_string()));
								drop(claimed);
								progress.inc(1);
								continue;
							}
							outcome.translated += 1;
							outcome.tokens += tokens;
							outcome.usd += usd;
						}
						Err(Refusal::Exhausted(reason)) => {
							outcome.exhausted = Some(reason);
							progress.finish_and_clear();
							return Ok(outcome);
						}
						Err(error) => outcome.failed.push((item.id(locale), error.to_string())),
					}
					progress.inc(1);
				}
			}
			Destination::Description(cid) => {
				for locale in &item.locales {
					progress.set_message(format!("{cid} {locale}"));
					let claimed = match claim::take(repo, "locale", &item.id(locale)) {
						Ok(claimed) => claimed,
						Err(claim::Denied::Taken(_)) => {
							outcome.claimed_elsewhere += 1;
							progress.inc(1);
							continue;
						}
						Err(claim::Denied::Io(error)) => return Err(error),
					};
					match translate_description(runner, model_override, &item, locale, &mut ask).await {
						Ok((translation, tokens, usd)) => {
							let id = item.id(locale);
							let path = described_path.clone();
							let key = cid.clone();
							let locale = locale.clone();
							let applied = media_writer.apply(move || {
								let mut current = media::load(&path)?;
								current
									.media
									.entry(key)
									.or_default()
									.description
									.insert(locale, translation);
								media::save(&path, &current)
							});
							if let Err(error) = applied {
								outcome.failed.push((id, error.to_string()));
								drop(claimed);
								progress.inc(1);
								continue;
							}
							outcome.translated += 1;
							outcome.tokens += tokens;
							outcome.usd += usd;
						}
						Err(Refusal::Exhausted(reason)) => {
							outcome.exhausted = Some(reason);
							progress.finish_and_clear();
							return Ok(outcome);
						}
						Err(error) => outcome.failed.push((item.id(locale), error.to_string())),
					}
					progress.inc(1);
				}
			}
		}
	}
	progress.finish_and_clear();
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

	struct Temp {
		root: std::path::PathBuf,
	}

	impl Temp {
		fn new(name: &str) -> Self {
			let root = std::env::temp_dir().join(format!(
				"cms-locale-{name}-{}-{}",
				std::process::id(),
				NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
			));
			let _ = std::fs::remove_dir_all(&root);
			std::fs::create_dir_all(root.join("data")).expect("temp data");
			Self { root }
		}
	}

	impl Drop for Temp {
		fn drop(&mut self) {
			std::fs::remove_dir_all(&self.root).ok();
		}
	}

	fn translation(text: &str) -> Translation {
		Translation {
			text: text.to_owned(),
			provider: "anthropic".to_owned(),
			model: "claude-sonnet-5".to_owned(),
			at: "2026-08-01T00:00:00Z".to_owned(),
			seconds: 1.0,
			tokens: 10,
			review: false,
		}
	}

	fn answer(text: &str) -> Answer {
		Answer {
			text: text.to_owned(),
			model: "gpt-oss-120b-medium".to_owned(),
			tokens: 12,
			usd: 0.0,
		}
	}

	fn ordinary(
		source: &str,
		meaning: &str,
		entries: impl IntoIterator<Item = (&'static str, Translation)>,
	) -> tags::Tag {
		let mut display =
			std::collections::BTreeMap::from([(SOURCE_LOCALE.to_owned(), translation(source))]);
		display.extend(
			entries
				.into_iter()
				.map(|(locale, translation)| (locale.to_owned(), translation)),
		);
		tags::Tag::Ordinary {
			source: source.to_owned(),
			meaning: meaning.to_owned(),
			display,
		}
	}

	fn marked(entries: &[(&str, &str)]) -> String {
		entries
			.iter()
			.map(|(locale, text)| format!("{}\n{text}\n", crate::i18n::prompt::locale_marker(locale)))
			.collect::<Vec<_>>()
			.join("\n")
	}

	/// A value another run holds a claim on is left to it, and no request is made for it.
	///
	/// The claim is what stops two runs paying for the same translation. Counting requests is
	/// the check that matters: a run that skipped the write but still asked would have spent
	/// the money anyway.
	#[tokio::test]
	async fn a_value_another_run_claimed_is_not_translated_again() {
		let temp = Temp::new("claimed");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"terminal".to_owned(),
			ordinary("Terminal", "terminal emulator or command-line window", []),
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let held = claim::take(&temp.root, "locale", "tag terminal/").expect("claim");
		let mut requests = 0;
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			false,
			None,
			&["zh-CN"],
			|_, _, _| {
				requests += 1;
				async { Ok(answer("终端")) }
			},
		)
		.await
		.expect("run");
		drop(held);

		assert_eq!(requests, 0);
		assert_eq!(outcome.claimed_elsewhere, 1);
		assert_eq!(outcome.translated, 0);
	}

	#[tokio::test]
	async fn an_existing_translation_is_skipped_without_a_request() {
		let temp = Temp::new("skip");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"terminal".to_owned(),
			ordinary(
				"Terminal",
				"terminal emulator or command-line window",
				[("zh-CN", translation("终端"))],
			),
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let mut requests = 0;
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			false,
			None,
			&["zh-CN"],
			|_, _, _| {
				requests += 1;
				std::future::ready(Ok(answer("unexpected")))
			},
		)
		.await
		.expect("run");

		assert_eq!(requests, 0);
		assert_eq!(outcome.skipped, 1);
		assert_eq!(
			tags::load(&tags::path_for(&temp.root)).expect("tags").tags["terminal"]
				.translations()
				.expect("ordinary")["zh-CN"],
			registry.tags["terminal"].translations().expect("ordinary")["zh-CN"]
		);
	}

	#[tokio::test]
	async fn force_never_overwrites_the_source_locale() {
		let temp = Temp::new("source");
		let mut described = media::Media::default();
		described.media.insert(
			"asset".to_owned(),
			media::Entry {
				description: std::collections::BTreeMap::from([(
					SOURCE_LOCALE.to_owned(),
					translation("Original description"),
				)]),
				..media::Entry::default()
			},
		);
		media::save(&media::path_for(&temp.root), &described).expect("media");

		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			true,
			None,
			&[SOURCE_LOCALE, "zh-CN"],
			|_, _, _| std::future::ready(Ok(answer("translated"))),
		)
		.await
		.expect("run");

		assert_eq!(outcome.translated, 1);
		let saved_media = media::load(&media::path_for(&temp.root)).expect("media");
		assert_eq!(
			saved_media.media["asset"].description[SOURCE_LOCALE].text,
			"Original description"
		);
	}

	#[tokio::test]
	async fn a_failed_unit_does_not_discard_another_answer() {
		let temp = Temp::new("failure");
		let mut registry = tags::Registry::default();
		registry
			.tags
			.insert("first".to_owned(), ordinary("First", "first concept", []));
		registry.tags.insert(
			"second".to_owned(),
			ordinary("Second", "second concept", []),
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let mut requests = 0;
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			false,
			None,
			&[SOURCE_LOCALE, "zh-CN"],
			|_, _, _| {
				requests += 1;
				std::future::ready(if requests <= ATTEMPTS {
					Err(Refusal::Failed("bad answer".to_owned()))
				} else {
					Ok(answer(&marked(&[("zh-CN", "第二")])))
				})
			},
		)
		.await
		.expect("run");

		assert_eq!(outcome.failed.len(), 1);
		assert_eq!(outcome.translated, 1);
		let saved = tags::load(&tags::path_for(&temp.root)).expect("tags");
		assert_eq!(
			saved.tags["first"].translations().expect("ordinary").len(),
			1
		);
		assert_eq!(
			saved.tags["second"].translations().expect("ordinary")["zh-CN"].text,
			"第二"
		);
	}

	#[tokio::test]
	async fn a_technical_tag_never_produces_a_translation_request() {
		let temp = Temp::new("technical");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"typescript".to_owned(),
			tags::Tag::Technical {
				display: "TypeScript".to_owned(),
				meaning: "programming language".to_owned(),
			},
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let mut requests = 0;
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			true,
			None,
			&crate::i18n::prompt::LOCALES,
			|_, _, _| {
				requests += 1;
				std::future::ready(Ok(answer("unexpected")))
			},
		)
		.await
		.expect("run");

		assert_eq!(requests, 0);
		assert_eq!(outcome.sources, 0);
	}

	#[tokio::test]
	async fn one_tag_requests_every_non_source_locale_once() {
		let temp = Temp::new("one-call");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"browser".to_owned(),
			ordinary("Browser", "software for viewing websites", []),
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");
		let translations: Vec<(&str, &str)> = crate::i18n::prompt::LOCALES
			.iter()
			.filter(|locale| **locale != SOURCE_LOCALE)
			.map(|locale| (*locale, *locale))
			.collect();
		let reply = marked(&translations);
		let mut requests = 0;

		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			true,
			None,
			&crate::i18n::prompt::LOCALES,
			|_, prompt, _| {
				requests += 1;
				for locale in crate::i18n::prompt::LOCALES
					.into_iter()
					.filter(|locale| *locale != SOURCE_LOCALE)
				{
					assert!(prompt.contains(&crate::i18n::prompt::locale_marker(locale)));
				}
				assert!(!prompt.contains(&crate::i18n::prompt::locale_marker(SOURCE_LOCALE)));
				std::future::ready(Ok(answer(&reply)))
			},
		)
		.await
		.expect("run");

		assert_eq!(requests, 1);
		assert_eq!(outcome.translated, crate::i18n::prompt::LOCALES.len() - 1);
		let saved = tags::load(&tags::path_for(&temp.root)).expect("tags");
		let display = saved.tags["browser"].translations().expect("ordinary");
		assert_eq!(display.len(), crate::i18n::prompt::LOCALES.len());
		assert_eq!(display[SOURCE_LOCALE].model, "claude-sonnet-5");
		assert_eq!(display[SOURCE_LOCALE].tokens, 10);
		assert!(
			display
				.iter()
				.filter(|(locale, _)| locale.as_str() != SOURCE_LOCALE)
				.all(|(_, translation)| translation.tokens == 12)
		);
	}

	#[tokio::test]
	async fn the_limit_reaches_tags_before_descriptions() {
		let temp = Temp::new("order");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"terminal".to_owned(),
			ordinary("Terminal", "terminal emulator or command-line window", []),
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let mut described = media::Media::default();
		described.media.insert(
			"000-first-by-key".to_owned(),
			media::Entry {
				description: std::collections::BTreeMap::from([(
					SOURCE_LOCALE.to_owned(),
					translation("A long description"),
				)]),
				..media::Entry::default()
			},
		);
		media::save(&media::path_for(&temp.root), &described).expect("media");

		let mut prompts = Vec::new();
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			false,
			Some(1),
			&[SOURCE_LOCALE, "zh-CN"],
			|_, prompt, _| {
				prompts.push(prompt);
				std::future::ready(Ok(answer(&marked(&[("zh-CN", "终端")]))))
			},
		)
		.await
		.expect("run");

		assert_eq!(prompts.len(), 1);
		assert!(prompts[0].contains("Raw identifier: terminal"));
		assert_eq!(outcome.deferred, 1);
		assert!(
			!media::load(&media::path_for(&temp.root))
				.expect("media")
				.media["000-first-by-key"]
				.description
				.contains_key("zh-CN")
		);
	}

	#[test]
	fn a_tag_prompt_carries_one_raw_name_and_every_target_locale() {
		let item = Item {
			destination: Destination::Tag("cellular-network".to_owned()),
			source: "Cellular Network".to_owned(),
			source_locale: SOURCE_LOCALE.to_owned(),
			meaning: Some("mobile carrier connectivity and SIM service, not biology".to_owned()),
			locales: vec!["zh-CN".to_owned()],
			kind: Kind::Heading,
		};
		let text = tag_request(&item);
		assert!(text.contains("ordinary tag"));
		assert!(text.contains("Raw identifier: cellular-network"));
		assert!(text.contains("English source label: Cellular Network"));
		assert!(text.contains("not biology"));
		assert!(text.contains("ready-to-render standalone UI label"));
		assert!(text.contains("en-US uses Title Case"));
		assert!(text.contains(&crate::i18n::prompt::locale_marker("zh-CN")));
	}
}
