//! The `cms i18n` command: translating what the articles say.
//!
//! Segment by segment, all locales at once, because a paragraph edited on its own should cost
//! one request and update every language together. See spec/architecture.md.

pub mod layout;
pub mod model;
pub mod prompt;
pub mod runner;
pub mod segment;
pub mod store;
pub mod tn;
pub mod validate;

use runner::{Refusal, Runner};
use segment::Segment;
use std::path::Path;
use store::Translation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
	All,
	Frontmatter,
}

impl Scope {
	fn includes(self, segment: &Segment) -> bool {
		self == Self::All || segment.region == segment::Region::Frontmatter
	}
}

/// Requests in flight. The same reasoning as `cms alt`: politeness rather than local limits.
pub const PARALLEL: usize = 4;

/// How many times a segment is asked for again before it is reported as failed.
///
/// A retry escalates the model, which is the one difficulty signal here that is measured
/// rather than guessed: the light model failing is an event, where a difficulty score would
/// only ever have been an estimate.
const ATTEMPTS: usize = 3;

/// Where the backoff starts, and how far it is allowed to grow.
///
/// Throttling is not counted against `ATTEMPTS`: nothing was wrong with the request, the
/// runner was simply busy. Waiting and asking again is the whole response, so the only limit
/// is the allowance itself.
const BACKOFF_START: std::time::Duration = std::time::Duration::from_secs(5);
const BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug, Default)]
pub struct Outcome {
	pub translated: usize,
	pub segments: usize,
	pub tokens: u64,
	pub usd: f64,
	pub failed: Vec<(String, String)>,
	pub orphans: usize,
	/// Set when the allowance ran out, carrying what the runner said about the reset.
	pub exhausted: Option<String>,
}

/// Lines a block occupies, ignoring the blank ones a reply may pad with.
fn body_lines(text: &str) -> usize {
	text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// Accept only locale text that can be restored and rendered under this segment's rules.
///
/// A failure returns to `translate`, whose attempt counter retries and escalates it. Keeping the
/// acceptance boundary here makes a malformed successful process no more trusted than a runner
/// process that explicitly failed.
fn validate_reply(
	reply: &str,
	boundary: &str,
	region: segment::Region,
	masked: &segment::Masked,
) -> Result<Vec<(String, String)>, Refusal> {
	let parsed = prompt::parse(reply, Some(boundary)).map_err(|prompt::BoundaryLeak| {
		Refusal::Failed("the model echoed the prompt boundary".to_owned())
	})?;
	// The neighbouring paragraphs go into the prompt as context, and a reply that includes them
	// is not a translation of this block -- it is this block plus somebody else's, stored under
	// this block's id. It shows up as untranslated prose appearing inside an unrelated view,
	// which is a long way from the reply that caused it. A block cannot gain lines in
	// translation, so counting them catches it at the point it happens.
	let allowed = body_lines(masked.text.as_str());
	let kept: Vec<(String, String)> = parsed
		.into_iter()
		.filter(|(_, text)| {
			masked.intact(text)
				&& body_lines(text) <= allowed
				&& validate::translation(region, text).is_ok()
		})
		.map(|(locale, text)| (locale, masked.restore(&text)))
		.collect();
	if kept.is_empty() {
		return Err(Refusal::Failed(
			"no locale survived marker and shape validation".to_owned(),
		));
	}
	Ok(kept)
}

/// Translate one segment into every locale it is missing.
async fn translate(
	item: &Segment,
	before: Option<String>,
	after: Option<String>,
	runner: Runner,
	gloss: Option<tn::Entry>,
) -> Result<(Vec<(String, Translation)>, u64, f64), Refusal> {
	let masked = segment::mask(&item.source);
	let started = crate::image::manifest::now();
	let clock = std::time::Instant::now();
	let mut last = Refusal::Failed(String::new());

	let mut attempt = 0usize;
	let mut backoff = BACKOFF_START;

	while attempt < ATTEMPTS {
		let request = prompt::build(
			item,
			&masked.text,
			before.as_deref(),
			after.as_deref(),
			gloss.as_ref(),
		);
		let wanted = runner.model_for(item.kind, attempt);
		let answer = match runner::ask(runner, &request.text, wanted).await {
			Ok(answer) => answer,
			// No point trying a stronger model against an allowance that is gone; it is the
			// same account either way. Stop and say so.
			Err(Refusal::Exhausted(reason)) => return Err(Refusal::Exhausted(reason)),
			// Busy, not spent. Wait and ask the same question again -- this does not consume
			// an attempt, because nothing was wrong with the request.
			Err(Refusal::Throttled(_)) => {
				tokio::time::sleep(backoff).await;
				backoff = (backoff * 2).min(BACKOFF_MAX);
				continue;
			}
			Err(error) => {
				last = error;
				attempt += 1;
				continue;
			}
		};

		// Every marker back exactly once, or the answer is not usable. This is the point of
		// masking: not hoping the model left the code alone, but being able to show it did.
		let kept = match validate_reply(&answer.text, &request.boundary, item.region, &masked) {
			Ok(kept) => kept,
			Err(error) => {
				last = error;
				attempt += 1;
				continue;
			}
		};

		// A locale that failed validation is missing, and asking again later reproduces it: the
		// same prompt to the same model fails the same way, so the gap becomes permanent while
		// every run reports success. Retry here instead, where the attempt counter escalates the
		// model. The last attempt keeps whatever survived -- some languages beat none -- and the
		// gap is then real rather than invisible, because the next run has something new to try.
		if kept.len() < prompt::LOCALES.len() && attempt + 1 < ATTEMPTS {
			let lost: Vec<&str> = prompt::LOCALES
				.iter()
				.copied()
				.filter(|locale| !kept.iter().any(|(kept, _)| kept == locale))
				.collect();
			last = Refusal::Failed(format!("{} did not survive validation", lost.join(", ")));
			attempt += 1;
			continue;
		}

		let provider = runner.provider().to_owned();
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
/// Translate every article under `articles`.
pub async fn run(
	runner: Runner,
	articles: &Path,
	only: &[std::path::PathBuf],
	limit: Option<usize>,
	force: bool,
	scope: Scope,
) -> std::io::Result<Outcome> {
	let mut outcome = Outcome::default();
	// Loaded once. A suggestion applies to a segment id, so which article it came from stops
	// mattering the moment it is written down.
	let glosses = tn::load(&tn::path_for(articles.parent().unwrap_or(articles)));
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
		outcome.segments += live
			.values()
			.filter(|segment| scope.includes(segment))
			.count();
		outcome.orphans += store::orphans(&sidecar, &live).len();

		let mut wanted = if force {
			live.keys().cloned().collect::<Vec<_>>()
		} else {
			store::missing(&sidecar, &live, &prompt::LOCALES, &glosses)
				.into_keys()
				.collect()
		};
		wanted.retain(|id| live.get(id).is_some_and(|segment| scope.includes(segment)));
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

		let progress = crate::progress::bar(todo.len() as u64);
		progress.set_message(format!("{}", path.display()));

		let mut queue = todo.into_iter();
		type Finished = (
			String,
			Result<(Vec<(String, Translation)>, u64, f64), Refusal>,
		);
		let mut running: Vec<tokio::task::JoinHandle<Finished>> = Vec::new();

		loop {
			while running.len() < PARALLEL {
				let Some(item) = queue.next() else {
					break;
				};
				progress.set_message(crate::progress::preview(&item.source, 44));
				let (before, after) = neighbours(&item.id);
				let owned = item.clone();
				let gloss = glosses.find(&item.id).cloned();
				running.push(tokio::spawn(async move {
					let id = owned.id.clone();
					(id, translate(&owned, before, after, runner, gloss).await)
				}));
			}
			if running.is_empty() {
				break;
			}

			let finished = match running.remove(0).await {
				Ok(result) => result,
				Err(error) => (String::new(), Err(Refusal::Failed(error.to_string()))),
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
				// One spent allowance ends the run. Every request after it would fail the same
				// way, and firing a hundred more only fills the screen with one fact repeated.
				Err(Refusal::Exhausted(reason)) => {
					outcome.exhausted = Some(reason);
					progress.finish_and_clear();
					return Ok(outcome);
				}
				Err(error) => outcome.failed.push((id, error.to_string())),
			}
		}
		progress.finish_and_clear();
	}
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::BTreeMap;

	#[test]
	fn frontmatter_scope_never_selects_body_prose() {
		let segments = segment::split(
			"---\ntitle: Visible title\nlang: en-US\n---\n\nBody that must stay out of this run.",
		);
		let selected = segments
			.iter()
			.filter(|segment| Scope::Frontmatter.includes(segment))
			.collect::<Vec<_>>();

		assert_eq!(selected.len(), 1);
		assert_eq!(selected[0].source, "Visible title");
		assert_eq!(selected[0].region, segment::Region::Frontmatter);
	}

	#[test]
	fn a_note_that_would_print_its_own_braces_is_refused() {
		// `:tn[words]{is="..."}` fails as a whole when any part of it is off, and fails quietly:
		// the parser stops seeing a directive and the braces render as text beside an empty
		// title. 90 of the first 168 markers were malformed this way, nearly all of them a
		// missing closing quote.
		assert!(validate::notes_well_formed("a :tn[word]{is=\"a note\"} b"));
		assert!(validate::notes_well_formed("nothing to check here"));

		// The one that reached a page: no closing quote, so the whole attribute block is text.
		assert!(!validate::notes_well_formed("a :tn[word]{is=\"a note} b"));
		// The syntax has no escape for a quote inside the value; it simply ends there.
		assert!(!validate::notes_well_formed(
			"a :tn[word]{is=\"he said \"no\" loudly\"} b"
		));
		assert!(!validate::notes_well_formed("a :tn[word] b"));
		assert!(!validate::notes_well_formed(
			"a :tn[word]{was=\"wrong key\"} b"
		));
	}

	#[test]
	fn a_frontmatter_note_is_refused_before_it_can_be_stored() {
		// Metadata has no rendering channel for the explanation. Prompt wording is not an
		// acceptance boundary, so a model that ignores it must enter the retry path here.
		let boundary = "F7Q2L9DM4KX8V1C6R0PB3HNS5WJATGEU";
		let reply = format!(
			"{}\n:tn[Translated title]{{is=\"a gloss\"}}\n",
			prompt::locale_marker("en-US"),
		);

		assert!(matches!(
			validate_reply(
				&reply,
				boundary,
				segment::Region::Frontmatter,
				&segment::mask("Source title"),
			),
			Err(Refusal::Failed(_))
		));
	}

	#[test]
	fn a_reply_carrying_the_neighbouring_paragraphs_is_refused() {
		// Measured from four articles: seventeen stored translations held their own block plus a
		// neighbour that had been supplied as context. Filed under this block's id, the extra
		// prose then surfaced untranslated inside an unrelated view, a long way from the reply
		// that caused it. A block does not gain lines in translation, so the shape says so.
		let boundary = "K3QZ7XW1M8ND5VBRTY2LPCFA6GHJ0SEU";
		let source = "::linkcard{src=\"a.avif\" url=\"https://example.com\" title=\"One\"}";
		let echoed = format!(
			"{}\n::linkcard{{src=\"b.avif\" url=\"https://other.com\" title=\"Two\"}}\n\
			 A paragraph that belongs to the block before this one.\n{source}\n",
			prompt::locale_marker("en-US"),
		);
		assert!(matches!(
			validate_reply(
				&echoed,
				boundary,
				segment::Region::Body,
				&segment::mask(source)
			),
			Err(Refusal::Failed(_))
		));

		// The same block answered on its own is kept, so the check costs nothing that is correct.
		let clean = format!(
			"{}\n::linkcard{{src=\"a.avif\" url=\"https://example.com\" title=\"Eins\"}}\n",
			prompt::locale_marker("de-DE"),
		);
		assert!(
			validate_reply(
				&clean,
				boundary,
				segment::Region::Body,
				&segment::mask(source)
			)
			.is_ok()
		);
	}

	#[test]
	fn a_multi_line_block_may_keep_its_lines() {
		// A list translates line for line, so the rule is "no more than", never "exactly one".
		let boundary = "PQ9WZ4WX2TN7VLKD8RYC5MBFA1GHJ0SE";
		let source = "- first\n- second\n- third";
		let reply = format!(
			"{}\n- erste\n- zweite\n- dritte\n",
			prompt::locale_marker("de-DE"),
		);
		assert!(
			validate_reply(
				&reply,
				boundary,
				segment::Region::Body,
				&segment::mask(source)
			)
			.is_ok()
		);
	}

	#[test]
	fn a_boundary_echo_cannot_reach_a_sidecar() {
		let boundary = "VVF4KTLBKEI0X2NJT7FOCD2N6HO4C0N2";
		let reply = format!(
			"{}\n{boundary}\nPaid-for prose remains intact.\n{boundary}\n",
			prompt::locale_marker("en-US"),
		);
		let result = validate_reply(
			&reply,
			boundary,
			segment::Region::Body,
			&segment::mask("source"),
		);
		assert!(matches!(&result, Err(Refusal::Failed(_))));

		let mut sidecar = store::Sidecar::default();
		if let Ok(entries) = result {
			sidecar.segments.insert(
				"segment".to_owned(),
				entries
					.into_iter()
					.map(|(locale, text)| {
						(
							locale,
							Translation {
								text,
								provider: "openai".to_owned(),
								model: "gpt-oss-120b-medium".to_owned(),
								at: "2026-08-02T00:00:00Z".to_owned(),
								seconds: 1.0,
								tokens: 10,
								review: false,
							},
						)
					})
					.collect::<BTreeMap<_, _>>(),
			);
		}
		assert!(sidecar.segments.is_empty());
		assert!(
			!serde_yaml_ng::to_string(&sidecar)
				.expect("sidecar")
				.contains(boundary)
		);
	}
}
