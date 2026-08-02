//! The `cms locale` command: translating tag labels and image descriptions.
//!
//! These are short plain strings, not article blocks. Each target locale is one request and
//! is saved as soon as it returns, so an interrupted run keeps every answer it paid for.

use crate::alt::SOURCE_LOCALE;
use crate::i18n::runner::{self, Answer, Refusal, Runner};
use crate::i18n::segment::Kind;
use crate::i18n::store::Translation;
use crate::{media, tags};
use indicatif::{ProgressBar, ProgressStyle};
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
	pub exhausted: Option<String>,
}

#[derive(Debug, Clone)]
enum Destination {
	Tag(String),
	Description(String),
}

#[derive(Debug, Clone)]
struct Item {
	destination: Destination,
	source: String,
	locales: Vec<String>,
	kind: Kind,
}

impl Item {
	fn id(&self, locale: &str) -> String {
		match &self.destination {
			Destination::Tag(name) => format!("tag {name}/{locale}"),
			Destination::Description(cid) => format!("description {cid}/{locale}"),
		}
	}

	fn label(&self) -> &str {
		match &self.destination {
			Destination::Tag(name) | Destination::Description(name) => name,
		}
	}
}

fn targets(
	translations: &std::collections::BTreeMap<String, Translation>,
	locales: &[&str],
	force: bool,
) -> (Vec<String>, usize) {
	let mut wanted = Vec::new();
	let mut skipped = 0;
	for locale in locales {
		// The source is input, never output. This remains true under --force.
		if *locale == SOURCE_LOCALE {
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
		let (wanted, already) = targets(&tag.display, locales, force);
		skipped += already;
		if !wanted.is_empty() {
			items.push(Item {
				destination: Destination::Tag(name.clone()),
				source: name.clone(),
				locales: wanted,
				kind: Kind::Heading,
			});
		}
	}

	for (cid, entry) in &described.media {
		let Some(source) = entry.description.get(SOURCE_LOCALE) else {
			continue;
		};
		let (wanted, already) = targets(&entry.description, locales, force);
		skipped += already;
		if !wanted.is_empty() {
			items.push(Item {
				destination: Destination::Description(cid.clone()),
				source: source.text.clone(),
				locales: wanted,
				kind: Kind::Prose,
			});
		}
	}
	(items, skipped)
}

fn request(item: &Item, locale: &str) -> String {
	match item.destination {
		Destination::Tag(_) => format!(
			"Translate one tag for display in {locale}. A tag is one word or a short noun phrase. \
			 A brand keeps its form in every language (for example, typescript becomes TypeScript \
			 in zh-CN too); a common noun is translated (for example, terminal becomes 终端 in \
			 zh-CN).\n\nRaw tag: {}\nLocale: {locale}\n\nReply with the translated tag alone. \
			 No preamble, quotes, explanation, or markdown.",
			item.source
		),
		Destination::Description(_) => format!(
			"Translate this short plain-text image description from {SOURCE_LOCALE} into {locale}. \
			 Preserve its meaning and factual detail. Reply with the translation alone: no \
			 preamble, quotes, explanation, or markdown.\n\n{}",
			item.source
		),
	}
}

async fn translate<F, Fut>(
	runner: Runner,
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
		let prompt = request(item, locale);
		let model = runner.model_for(item.kind, attempt).to_owned();
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

fn bar(total: usize) -> ProgressBar {
	let bar = ProgressBar::new(total as u64);
	bar.set_style(
		ProgressStyle::with_template("  {bar:28} {pos}/{len}  {wide_msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar()),
	);
	bar
}

pub async fn run(
	repo: &Path,
	runner: Runner,
	force: bool,
	limit: Option<usize>,
) -> std::io::Result<Outcome> {
	run_with(
		repo,
		runner,
		force,
		limit,
		&crate::i18n::prompt::LOCALES,
		|runner, prompt, model| async move { runner::ask(runner, &prompt, &model).await },
	)
	.await
}

async fn run_with<F, Fut>(
	repo: &Path,
	runner: Runner,
	force: bool,
	limit: Option<usize>,
	locales: &[&str],
	mut ask: F,
) -> std::io::Result<Outcome>
where
	F: FnMut(Runner, String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	let registry_path = tags::path_for(repo);
	let mut registry = tags::load(&registry_path);
	let described_path = media::path_for(repo);
	let mut described = media::load(&described_path);
	let (mut items, skipped) = pending(&registry, &described, locales, force);
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
	let calls = items.iter().map(|item| item.locales.len()).sum();
	let progress = bar(calls);

	for item in items {
		for locale in &item.locales {
			progress.set_message(format!("{} {locale}", item.label()));
			let result = translate(runner, &item, locale, &mut ask).await;
			match result {
				Ok((translation, tokens, usd)) => {
					match &item.destination {
						Destination::Tag(name) => {
							registry
								.tags
								.entry(name.clone())
								.or_default()
								.display
								.insert(locale.clone(), translation);
							tags::save(&registry_path, &registry)?;
						}
						Destination::Description(cid) => {
							described
								.media
								.entry(cid.clone())
								.or_default()
								.description
								.insert(locale.clone(), translation);
							media::save(&described_path, &described)?;
						}
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

	#[tokio::test]
	async fn an_existing_translation_is_skipped_without_a_request() {
		let temp = Temp::new("skip");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"terminal".to_owned(),
			tags::Tag {
				display: std::collections::BTreeMap::from([("zh-CN".to_owned(), translation("终端"))]),
			},
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
				std::future::ready(Ok(answer("unexpected")))
			},
		)
		.await
		.expect("run");

		assert_eq!(requests, 0);
		assert_eq!(outcome.skipped, 1);
		assert_eq!(
			tags::load(&tags::path_for(&temp.root)).tags["terminal"].display["zh-CN"],
			registry.tags["terminal"].display["zh-CN"]
		);
	}

	#[tokio::test]
	async fn force_never_overwrites_the_source_locale() {
		let temp = Temp::new("source");
		let mut registry = tags::Registry::default();
		registry.tags.insert(
			"typescript".to_owned(),
			tags::Tag {
				display: std::collections::BTreeMap::from([(
					SOURCE_LOCALE.to_owned(),
					translation("TypeScript"),
				)]),
			},
		);
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

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

		assert_eq!(outcome.translated, 2);
		let saved_tags = tags::load(&tags::path_for(&temp.root));
		assert_eq!(
			saved_tags.tags["typescript"].display[SOURCE_LOCALE].text,
			"TypeScript"
		);
		let translated = &saved_tags.tags["typescript"].display["zh-CN"];
		assert_eq!(translated.provider, "openai");
		assert_eq!(translated.model, "gpt-oss-120b-medium");
		assert_eq!(translated.tokens, 12);
		assert!(!translated.at.is_empty());
		assert!(!translated.review);
		let saved_media = media::load(&media::path_for(&temp.root));
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
			.insert("terminal".to_owned(), tags::Tag::default());
		tags::save(&tags::path_for(&temp.root), &registry).expect("tags");

		let mut requests = 0;
		let outcome = run_with(
			&temp.root,
			Runner::GptOss,
			false,
			None,
			&[SOURCE_LOCALE, "zh-CN", "ja-JP"],
			|_, _, _| {
				requests += 1;
				std::future::ready(if requests <= ATTEMPTS {
					Err(Refusal::Failed("bad answer".to_owned()))
				} else {
					Ok(answer("ターミナル"))
				})
			},
		)
		.await
		.expect("run");

		assert_eq!(outcome.failed.len(), 1);
		assert_eq!(outcome.translated, 1);
		let saved = tags::load(&tags::path_for(&temp.root));
		assert!(!saved.tags["terminal"].display.contains_key("zh-CN"));
		assert_eq!(saved.tags["terminal"].display["ja-JP"].text, "ターミナル");
	}

	#[tokio::test]
	async fn the_limit_reaches_tags_before_descriptions() {
		let temp = Temp::new("order");
		let mut registry = tags::Registry::default();
		registry
			.tags
			.insert("terminal".to_owned(), tags::Tag::default());
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
				std::future::ready(Ok(answer("终端")))
			},
		)
		.await
		.expect("run");

		assert_eq!(prompts.len(), 1);
		assert!(prompts[0].contains("Raw tag: terminal"));
		assert_eq!(outcome.deferred, 1);
		assert!(
			!media::load(&media::path_for(&temp.root)).media["000-first-by-key"]
				.description
				.contains_key("zh-CN")
		);
	}

	#[test]
	fn a_tag_prompt_carries_only_the_raw_name_and_target_locale_as_data() {
		let item = Item {
			destination: Destination::Tag("typescript".to_owned()),
			source: "typescript".to_owned(),
			locales: vec!["zh-CN".to_owned()],
			kind: Kind::Heading,
		};
		let text = request(&item, "zh-CN");
		assert!(text.contains("brand keeps its form"));
		assert!(text.contains("common noun is translated"));
		assert!(text.contains("Raw tag: typescript"));
		assert!(text.contains("Locale: zh-CN"));
	}
}
