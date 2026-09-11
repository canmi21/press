//! The `cms i18n` command: translating what the articles say.
//!
//! Segment by segment, with every missing locale in one request, because a paragraph edited on
//! its own should cost one call while a partial repair should never repay for finished work. See
//! spec/i18n.md.

pub mod audit;
pub mod invalidate;
pub mod layout;
pub mod model;
pub mod prompt;
pub mod runner;
pub mod segment;
pub mod store;
pub mod tn;
pub mod validate;
pub mod width;

use crate::task::{Record, claim, progress, registry, writer};
use runner::{Refusal, Runner};
use segment::Segment;
use std::path::Path;
use store::Translation;

/// Makes the sink each article's progress bar reports to.
///
/// A factory rather than one sink, because the terminal draws a bar per article and a bar that has
/// been finished cannot be started again. The desktop will pass something that folds them into one
/// view; that is its decision to make, not this function's.
pub type Sinks = Box<dyn Fn() -> Box<dyn progress::Sink> + Send + Sync>;

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
pub const DEFAULT_PARALLEL: usize = 4;

pub fn parallelism(value: Option<&str>) -> Result<usize, String> {
	let Some(value) = value else {
		return Ok(DEFAULT_PARALLEL);
	};
	match value.parse::<usize>() {
		Ok(value) if value > 0 => Ok(value),
		_ => Err("--parallel takes a positive integer".to_owned()),
	}
}

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
	pub incomplete_segments: usize,
	pub missing_locales: usize,
	/// Segments another live run was translating, left to it.
	pub claimed_elsewhere: usize,
	/// Segments that turned out to be translated already once the claim was held -- work another
	/// run finished between the list being built and this item being reached.
	pub already_done: usize,
	/// Set when the allowance ran out, carrying what the runner said about the reset.
	pub exhausted: Option<String>,
	/// Report-only policy findings from `--check`: article, then what audit::of saw.
	pub audit: Vec<(String, audit::Finding)>,
	/// Locale views that will not render at all: article, locale, and how many body segments are
	/// missing. One is enough -- the site serves the source article rather than mixing languages
	/// inside a page -- so this is reported as its own outcome rather than left to be inferred
	/// from a count of missing entries.
	pub blocked_views: Vec<(String, String, usize)>,
}

/// When a file was last written, or `None` if it does not exist yet.
///
/// Used to notice another process having touched a sidecar without re-reading it every time. A
/// missing file and an unreadable one are the same answer here: reload and find out.
fn modified_at(path: &Path) -> Option<std::time::SystemTime> {
	std::fs::metadata(path).ok()?.modified().ok()
}

pub fn selected_locales(values: &[String]) -> Result<Vec<&'static str>, String> {
	if values.is_empty() {
		return Ok(prompt::LOCALES.to_vec());
	}
	for value in values {
		if !prompt::LOCALES.contains(&value.as_str()) {
			return Err(format!("--locale takes one of {}", prompt::LOCALES.join(", ")));
		}
	}
	Ok(
		prompt::LOCALES
			.iter()
			.copied()
			.filter(|locale| values.iter().any(|value| value == locale))
			.collect(),
	)
}

/// Lines a block occupies, ignoring the blank ones a reply may pad with.
/// The file name of a page. The homepage is the whole of that category today.
const PAGE_FILE: &str = "homepage.md";

/// Refuse a file that is a page rather than an article.
///
/// A page has no `lang`, so nothing here could translate it out of anything; and it is identity
/// copy the site renders from the source in every view, so a sidecar for it would be text nobody
/// reads. See spec/i18n.md.
///
/// **This is not how pages are excluded.** `run` drops them while it is still building the list,
/// before a byte is read, and that is the path taken every time. This exists for the day somebody
/// edits that filter: without it a page would flow into the run and quietly acquire translations,
/// and the first sign of it would be a sidecar appearing in a diff. With it the run says which
/// file arrived and where it should have been stopped.
fn refuse_page(path: &Path) -> Result<(), String> {
	if path.file_name().is_some_and(|name| name == PAGE_FILE) {
		return Err(format!(
			"{PAGE_FILE} is a page, not an article, and is never translated. It reached the run \
			 anyway, which means the filter in `run` no longer holds. See spec/i18n.md."
		));
	}
	Ok(())
}

fn body_lines(text: &str) -> usize {
	text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// Accept only locale text that can be restored and rendered under this segment's rules.
///
/// A failure returns to `translate`, whose attempt counter retries and escalates it. Keeping the
/// acceptance boundary here makes a malformed successful process no more trusted than a runner
/// process that explicitly failed.
#[cfg(test)]
fn validate_reply(
	reply: &str,
	boundary: &str,
	region: segment::Region,
	masked: &segment::Masked,
) -> Result<Vec<(String, String)>, Refusal> {
	validate_reply_for(
		reply,
		boundary,
		segment::Kind::Prose,
		region,
		masked.text.as_str(),
		masked,
		&prompt::LOCALES,
		None,
	)
}

fn validate_reply_for(
	reply: &str,
	boundary: &str,
	kind: segment::Kind,
	region: segment::Region,
	source: &str,
	masked: &segment::Masked,
	locales: &[&str],
	_source_locale: Option<&str>,
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
	// Read from the source rather than from the answer: a reply that changed the marks is a
	// different fault, and it must not be able to talk its way out of this one.
	let level = width::level(source);
	// Width is read after restoring, not before: a marker stands in for code of a different
	// length, so measuring the masked text would charge the heading for the wrong glyphs.
	let mut too_wide = Vec::new();
	let mut glued = Vec::new();
	let mut oversized = Vec::new();
	let wrong_block = std::cell::RefCell::new(Vec::new());
	let kept: Vec<(String, String)> = parsed
		.into_iter()
		.filter(|(locale, text)| {
			// The shape checks read the masked text, which is what was asked for. Masking lifts
			// inline code and nothing else, so the note counts they compare are the same on either
			// side of it.
			if !locales.contains(&locale.as_str()) {
				return false;
			}
			if !masked.intact(text) || body_lines(text) > allowed {
				return false;
			}
			match validate::translation(region, masked.text.as_str(), text) {
				Ok(()) => true,
				// Named rather than swept into the generic message: an author's note that did not
				// come back is not a translation of this block at all, and the next move is to
				// look at what block it *is* -- not to ask the same question again.
				Err(error @ validate::Error::AuthorNoteCountChanged) => {
					wrong_block.borrow_mut().push(format!("{locale} ({error})"));
					false
				}
				Err(_) => false,
			}
		})
		.map(|(locale, text)| (locale, masked.restore(&text)))
		.filter(|(locale, text)| {
			// Read after restoring: every marker this block owns has just been put back, so one
			// still standing here belongs to the folded context and was copied out of it.
			if !validate::markers_resolved(text) {
				wrong_block.borrow_mut().push(format!("{locale} ({})", validate::Error::UnresolvedMarker));
				return false;
			}
			// Size is compared against the real source, not the masked one, for the same reason
			// the two below are read here: a marker stands in for code of a different length.
			if !validate::size_plausible(source, text) {
				oversized.push(format!("{locale} at {} columns", width::raw(text)));
				return false;
			}
			// Spacing is read after restoring too: a marker is not the word it stands for, so
			// masked text cannot say whether a directive is glued to its neighbour.
			if !validate::spacing_intact(text) {
				glued.push(locale.clone());
				return false;
			}
			let fits = validate::heading_fits(kind, region, level, text);
			if !fits {
				too_wide.push(format!("{locale} at {} columns", width::of(text)));
			}
			fits
		})
		.collect();
	if kept.is_empty() {
		// Naming the width when that is the reason: the generic message covers a lost marker, an
		// added line and a malformed note alike, and none of those suggests the same next move.
		let wrong_block = wrong_block.into_inner();
		return Err(Refusal::Failed(if !wrong_block.is_empty() {
			format!("the reply is about a different block ({})", wrong_block.join(", "))
		} else if !oversized.is_empty() {
			format!(
				"a translation is far larger than its source ({}; source is {} columns) -- the \
				 reply is probably the neighbouring context rather than this block",
				oversized.join(", "),
				width::raw(source)
			)
		} else if !glued.is_empty() {
			format!("a note directive is glued to the word beside it ({})", glued.join(", "))
		} else if too_wide.is_empty() {
			"no locale survived marker and shape validation".to_owned()
		} else {
			format!(
				"heading overruns the table of contents ({}; {} columns is the limit)",
				too_wide.join(", "),
				width::CLAMP
			)
		}));
	}
	Ok(kept)
}

/// Translate one segment into every locale it is missing.
#[derive(Clone)]
struct TranslationOptions {
	runner: Runner,
	model_override: Option<String>,
	locales: Vec<String>,
	source_locale: Option<String>,
	gloss: Option<tn::Entry>,
}

async fn translate(
	item: &Segment,
	before: Option<String>,
	after: Option<String>,
	options: TranslationOptions,
) -> Result<(Vec<(String, Translation)>, u64, f64, Vec<String>), Refusal> {
	let masked = segment::mask(&item.source);
	let mut last = Refusal::Failed(String::new());
	let mut attempt = 0usize;
	let mut backoff = BACKOFF_START;
	let mut pending = options.locales;
	let mut entries = Vec::new();
	let mut total_tokens = 0u64;
	let mut total_usd = 0.0;

	while attempt < ATTEMPTS {
		let locale_refs = pending.iter().map(String::as_str).collect::<Vec<_>>();
		let request = prompt::build_for(
			item,
			&masked.text,
			before.as_deref(),
			after.as_deref(),
			&locale_refs,
			options.source_locale.as_deref(),
			options.gloss.as_ref(),
		);
		let wanted = options
			.model_override
			.as_deref()
			.unwrap_or_else(|| options.runner.model_for(item.kind, attempt));
		let started = crate::image::manifest::now();
		let clock = std::time::Instant::now();
		let answer = match runner::ask(options.runner, &request.text, wanted).await {
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
		total_tokens += answer.tokens;
		total_usd += answer.usd;

		// Every marker back exactly once, or the answer is not usable. This is the point of
		// masking: not hoping the model left the code alone, but being able to show it did.
		let kept = match validate_reply_for(
			&answer.text,
			&request.boundary,
			item.kind,
			item.region,
			&item.source,
			&masked,
			&locale_refs,
			options.source_locale.as_deref(),
		) {
			Ok(kept) => kept,
			Err(error) => {
				last = error;
				attempt += 1;
				continue;
			}
		};

		let provider = options.runner.provider().to_owned();
		let seconds = clock.elapsed().as_secs_f64();
		for (locale, text) in kept {
			entries.push((
				locale.clone(),
				Translation {
					text,
					provider: provider.clone(),
					model: answer.model.clone(),
					at: started.clone(),
					seconds,
					tokens: answer.tokens,
					review: false,
				},
			));
			pending.retain(|wanted| wanted != &locale);
		}
		if pending.is_empty() {
			return Ok((entries, total_tokens, total_usd, pending));
		}

		// Keep every locale that survived this attempt. The retry asks only for the remainder,
		// so one malformed language cannot make the other paid-for answers disposable.
		last = Refusal::Failed(format!("{} did not survive validation", pending.join(", ")));
		attempt += 1;
	}
	if entries.is_empty() { Err(last) } else { Ok((entries, total_tokens, total_usd, pending)) }
}

/// How many times a display request is sent before the run gives up on it.
///
/// Two, where a body block gets three. A body block escalates through model tiers because a hard
/// paragraph can defeat a cheap model; a title is short and the run is already on the strongest
/// tier the caller named. What a second attempt buys here is one specific thing: the first reply
/// is told, per field, that it came back over its budget, and a model that overshot by three
/// characters usually fixes that when shown the number. A third would be the same ask again.
const DISPLAY_ATTEMPTS: usize = 2;

struct DisplayRequest<'a> {
	title: &'a str,
	subtitle: Option<&'a str>,
	context: &'a str,
	wanted: Vec<(String, segment::Display)>,
	have: Vec<(String, segment::Display, String)>,
	runner: Runner,
	model_override: Option<String>,
	source_locale: Option<String>,
}

type DisplayEntry = (String, segment::Display, Translation);
type DisplayResult =
	Result<(Vec<DisplayEntry>, u64, f64, Vec<(String, segment::Display)>), Refusal>;

/// Ask for one article's display metadata: the missing fields, in one request, across all locales.
///
/// Mirrors `translate` and differs in what a reply can fail on. A body block is refused for its
/// shape -- a lost marker, a malformed note. A title is refused for its size, because the column
/// it is drawn in has a width and the reader loses whatever runs past it. So the retry carries the
/// measurement back: not "that was wrong" but "that one drew 214px where 186 is the room".
async fn translate_display(request: DisplayRequest<'_>) -> DisplayResult {
	let DisplayRequest {
		title,
		subtitle,
		context,
		mut wanted,
		mut have,
		runner,
		model_override,
		source_locale,
	} = request;

	let mut last = Refusal::Failed(String::new());
	let mut attempt = 0usize;
	let mut backoff = BACKOFF_START;
	let mut entries: Vec<DisplayEntry> = Vec::new();
	let mut total_tokens = 0u64;
	let mut total_usd = 0.0;
	let mut rejected: Vec<String> = Vec::new();
	// The article's own title and subtitle, read as one. The dash rule asks whether the author
	// spent a dash anywhere in what a card shows, not whether they spent it in this field.
	let metadata = format!("{title}\n{}", subtitle.unwrap_or_default());

	while attempt < DISPLAY_ATTEMPTS && !wanted.is_empty() {
		let built =
			prompt::build_display(title, subtitle, context, &wanted, &have, source_locale.as_deref());
		// What the previous attempt got wrong, in the units the rule is written in. A model shown
		// "over by 28px" can act on it; one shown "invalid" can only guess again.
		let text = if rejected.is_empty() {
			built.text
		} else {
			format!(
				"{}\n\nThe previous attempt came back over the limit on these. Write them shorter:\n{}",
				built.text,
				rejected.join("\n")
			)
		};
		rejected.clear();

		let model = model_override
			.as_deref()
			.unwrap_or_else(|| runner.model_for(segment::Kind::Heading, attempt));
		let started = crate::image::manifest::now();
		let clock = std::time::Instant::now();
		let answer = match runner::ask(runner, &text, model).await {
			Ok(answer) => answer,
			Err(Refusal::Exhausted(reason)) => return Err(Refusal::Exhausted(reason)),
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
		total_tokens += answer.tokens;
		total_usd += answer.usd;

		let parsed = match prompt::parse_display(&answer.text, Some(&built.boundary)) {
			Ok(parsed) => parsed,
			Err(prompt::BoundaryLeak) => {
				last = Refusal::Failed("the reply carried the fence back".to_owned());
				attempt += 1;
				continue;
			}
		};

		let provider = runner.provider().to_owned();
		let seconds = clock.elapsed().as_secs_f64();
		for (locale, field, answered) in parsed {
			if !wanted.iter().any(|(l, f)| l == &locale && f == &field) {
				continue;
			}
			if let Err(error) = validate::display(field, &answered, &metadata) {
				rejected.push(format!("{}  {error}: {answered}", prompt::field_marker(&locale, field)));
				continue;
			}
			entries.push((
				locale.clone(),
				field,
				Translation {
					text: answered.clone(),
					provider: provider.clone(),
					model: answer.model.clone(),
					at: started.clone(),
					seconds,
					tokens: answer.tokens,
					review: false,
				},
			));
			// Kept for the retry: a field written on this attempt is context for one written on
			// the next, which is the whole reason the four are asked for together.
			have.push((locale.clone(), field, answered));
			wanted.retain(|(l, f)| !(l == &locale && f == &field));
		}

		if wanted.is_empty() {
			return Ok((entries, total_tokens, total_usd, wanted));
		}
		// Carrying what was measured, not only which field failed. A run that reports a name
		// leaves the reader to reproduce the request to find out how far over it was, and how far
		// over is the whole question: three characters is a prompt to tighten, three hundred is a
		// model answering something else.
		last = Refusal::Failed(if rejected.is_empty() {
			format!(
				"{} came back missing",
				wanted
					.iter()
					.map(|(locale, field)| format!("{locale}:{}", field.name()))
					.collect::<Vec<_>>()
					.join(", ")
			)
		} else {
			format!("{}", rejected.join("; "))
		});
		attempt += 1;
	}
	if entries.is_empty() && !wanted.is_empty() {
		Err(last)
	} else {
		Ok((entries, total_tokens, total_usd, wanted))
	}
}

/// One line, rewritten in place, showing what is being worked on.
/// Translate every article under `articles`.
pub struct RunOptions<'a> {
	pub runner: Runner,
	pub model_override: Option<String>,
	pub limit: Option<usize>,
	pub parallel: usize,
	pub force: bool,
	pub scope: Scope,
	pub locales: &'a [&'a str],
	pub check: bool,
	/// The repository root, for the run registry, the claims and the record lock.
	pub repository: &'a Path,
	pub shell: registry::Shell,
	pub sinks: Sinks,
}

pub async fn run(
	articles: &Path,
	only: &[std::path::PathBuf],
	options: RunOptions<'_>,
) -> std::io::Result<Outcome> {
	let RunOptions {
		runner,
		model_override,
		limit,
		parallel,
		force,
		scope,
		locales,
		check,
		repository,
		shell,
		sinks,
	} = options;
	let mut outcome = Outcome::default();
	// Loaded once. A suggestion applies to a segment id, so which article it came from stops
	// mattering the moment it is written down.
	let glosses = tn::load(&tn::path_for(articles.parent().unwrap_or(articles)))?;
	let mut budget = limit.unwrap_or(usize::MAX);

	// Walked up front so the registry can publish a total rather than a number that grows while
	// somebody watches it. Reading the articles twice is local file I/O against a run that spends
	// minutes per article on a model.
	let planned: Vec<std::path::PathBuf> = crate::refs::markdown_under(articles)?
		.into_iter()
		// Named articles narrow the run. Retranslating one edited piece should not mean walking
		// everything before it in the tree.
		.filter(|path| {
			only.is_empty() || only.iter().any(|wanted| path.ends_with(wanted) || path == wanted)
		})
		// Pages leave here, before anything is read and before the total is published, so a run
		// never counts work it was never going to do. `refuse_page` below is the guard for this
		// line being changed, not the mechanism it uses.
		.filter(|path| refuse_page(path).is_ok())
		.collect();

	// One entry for the whole run, counted in articles. The per-article bars below count segments;
	// the two are different units on purpose and each says which it is.
	let planned_total = planned.len() as u64;
	let entry = registry::publish(repository, "i18n", shell, planned_total)?;
	let published = registry::Published::new(entry);
	let translations = writer::Writer::start(repository, Record::Translations)?;
	let mut articles_done = 0u64;

	for path in planned {
		if budget == 0 {
			break;
		}
		{
			use progress::Sink as _;
			published.advanced(articles_done, planned_total, &path.display().to_string());
		}
		// Recorded rather than returned: one file arriving that should not have is a fault in
		// this command, not a reason to abandon the articles that are fine. It lands in the run's
		// own report, where a reader is already looking for what did not get done.
		if let Err(reason) = refuse_page(&path) {
			outcome.failed.push((path.display().to_string(), reason));
			continue;
		}
		let article = std::fs::read_to_string(&path)?;
		// A page is not an article and is never translated. The test is the same one `cms
		// summary` applies: no `lang` frontmatter, no language to translate out of. The homepage
		// is the standing example -- it is identity copy, rendered from the source in every
		// view, and translations of it were only ever dead weight. See spec/i18n.md.
		let fields = crate::document::fields_of(&article, &path)?;
		let Some(lang) = crate::summary::lang_of(&fields) else {
			continue;
		};
		let source_locale = crate::summary::source_locale(lang).map(str::to_owned);
		let live = segment::translatable(&article).map_err(|error| {
			std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {error}", path.display()))
		})?;
		let sidecar_path = store::path_for(&path);
		let mut sidecar = store::load(&sidecar_path)?;
		outcome.segments += live.values().filter(|segment| scope.includes(segment)).count();
		outcome.orphans += store::orphans(&sidecar, &live).len();

		let mut wanted = if force && !check {
			live
				.keys()
				.map(|id| (id.clone(), locales.iter().map(|locale| (*locale).to_owned()).collect()))
				.collect::<std::collections::BTreeMap<_, _>>()
		} else {
			store::missing(&sidecar, &live, locales, source_locale.as_deref(), &glosses)
		};
		wanted.retain(|id, _| live.get(id).is_some_and(|segment| scope.includes(segment)));
		if check {
			outcome.incomplete_segments += wanted.len();
			outcome.missing_locales += wanted.values().map(Vec::len).sum::<usize>();
			// The policies shape cannot enforce, reported over what is already stored. Only
			// live body segments: an orphaned entry is leaving anyway, and frontmatter carries
			// no notes.
			for (id, locales) in &sidecar.segments {
				let Some(segment) = live.get(id) else {
					continue;
				};
				if segment.region != segment::Region::Body {
					// Frontmatter carries no notes, but the fields that are drawn have a width.
					if let Some(field) = segment.display {
						for (locale, translation) in locales {
							for found in audit::display(id, locale, &translation.text, field) {
								outcome.audit.push((path.display().to_string(), found));
							}
						}
					}
					continue;
				}
				for (locale, translation) in locales {
					for found in audit::of(
						id,
						locale,
						&translation.text,
						segment.kind,
						&segment.source,
						glosses.find(id),
					) {
						outcome.audit.push((path.display().to_string(), found));
					}
				}
				// And once more with every locale of this segment in hand: the checks above cannot
				// see a sibling, and a sibling is what catches an answer that is well-formed and
				// simply about something else.
				let together: Vec<(&str, &str)> = locales
					.iter()
					.map(|(locale, translation)| (locale.as_str(), translation.text.as_str()))
					.collect();
				for found in audit::across_locales(id, &segment.source, &together) {
					outcome.audit.push((path.display().to_string(), found));
				}
			}
			// A missing body segment is not a gap in one paragraph. The site refuses to mix
			// languages within a page, so it serves the whole article in the source language and
			// says the translation is unavailable -- which means one absent block and a wholly
			// untranslated locale are the same event, and a count alone does not say so.
			for locale in locales {
				let blocked = wanted
					.iter()
					.filter(|(id, missing)| {
						missing.iter().any(|m| m == locale)
							&& live.get(*id).is_some_and(|segment| segment.region == segment::Region::Body)
					})
					.count();
				if blocked > 0 {
					outcome.blocked_views.push((path.display().to_string(), (*locale).to_owned(), blocked));
				}
			}
			continue;
		}

		// Order for context comes from the article, which is the only place it lives. Two things
		// this list is not: it is not `live`, and it is not the whole article.
		//
		// Not `live`, because that one is filtered to what gets translated -- so a code fence or a
		// figure between two paragraphs was invisible here and the "neighbouring paragraph" was
		// whatever prose lay beyond it. Every block is a neighbour now; `context_of` folds the
		// unreadable ones down to a word.
		//
		// Not the whole article, because `live` also carries the frontmatter title, subtitle and
		// description, and they sort ahead of the body by line. That made the article's own
		// description the "previous paragraph" of its first body block -- a dense, self-contained
		// summary presented to the model as the thing this paragraph follows on from. It came back
		// translated in its place. Metadata is not prose that leads anywhere, so it takes no
		// context and gives none. See spec/i18n.md.
		let ordered: Vec<Segment> = segment::split(&article)
			.map_err(|error| {
				std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {error}", path.display()))
			})?
			.into_iter()
			.filter(|segment| segment.region == segment::Region::Body)
			.collect();
		let neighbours = |item: &Segment| {
			if item.region != segment::Region::Body {
				return (None, None);
			}
			// By offset rather than by id: one id can sit in an article many times -- every
			// horizontal rule shares one -- and a position found by id would be the first of them.
			let at = ordered.iter().position(|s| s.start == item.start);
			at.map_or((None, None), |at| {
				(
					at.checked_sub(1).and_then(|i| ordered.get(i)).map(segment::context_of),
					ordered.get(at + 1).map(segment::context_of),
				)
			})
		};

		// The four drawn fields are asked for together, in one request, by the pass below. They
		// are separate segments and separate stored entries -- what differs is only that a
		// subtitle written without its title beside it, or a short form written without the full
		// one, is written in ignorance of the thing it has to agree with.
		let display_todo: Vec<(String, segment::Display, Vec<String>)> = wanted
			.iter()
			.filter_map(|(id, locales)| {
				let field = live.get(id)?.display?;
				Some((id.clone(), field, locales.clone()))
			})
			.collect();
		let todo: Vec<(Segment, Vec<String>)> = wanted
			.iter()
			.filter(|(id, _)| live.get(*id).is_none_or(|segment| segment.display.is_none()))
			.filter_map(|(id, locales)| live.get(id).map(|segment| (segment.clone(), locales.clone())))
			.take(budget)
			.collect();
		budget -= todo.len();

		// The article key claims are namespaced by, so two articles holding a segment with the
		// same id -- which happens, since an id is the hash of the text -- are two items.
		let article_key = path.strip_prefix(articles).unwrap_or(&path).display().to_string();
		let mut sidecar_seen = modified_at(&sidecar_path);

		if !display_todo.is_empty() && budget > 0 {
			let by_field = |field: segment::Display| {
				live.values().find(|segment| segment.display == Some(field)).cloned()
			};
			let title_segment = by_field(segment::Display::Title);
			let subtitle_segment = by_field(segment::Display::Subtitle);
			// What the article is about, so a short form can be written to the piece rather than
			// to the words of the title. The description is written for exactly this and is not
			// drawn anywhere, so it costs the reader nothing to be long.
			let context = live
				.values()
				.find(|segment| segment.region == segment::Region::Frontmatter && segment.display.is_none())
				.map(|segment| segment.source.clone())
				.or_else(|| {
					live
						.values()
						.find(|segment| {
							segment.region == segment::Region::Body && segment.kind == segment::Kind::Prose
						})
						.map(|segment| segment.source.chars().take(600).collect())
				})
				.unwrap_or_default();

			if let Some(title_segment) = title_segment {
				let ids: std::collections::HashMap<segment::Display, String> = live
					.values()
					.filter_map(|segment| Some((segment.display?, segment.id.clone())))
					.collect();
				let mut ask: Vec<(String, segment::Display)> = Vec::new();
				for (_, field, missing) in &display_todo {
					for locale in missing {
						ask.push((locale.clone(), *field));
					}
				}
				let mut held: Vec<(String, segment::Display, String)> = Vec::new();
				for (field, id) in &ids {
					let Some(stored) = sidecar.segments.get(id) else {
						continue;
					};
					for (locale, entry) in stored {
						if !ask.iter().any(|(l, f)| l == locale && f == field) {
							held.push((locale.clone(), *field, entry.text.clone()));
						}
					}
				}

				let claimed = claim::take(repository, "i18n", &format!("{article_key}#display"));
				match claimed {
					Err(claim::Denied::Taken(_)) => outcome.claimed_elsewhere += 1,
					Err(claim::Denied::Io(error)) => return Err(error),
					Ok(claimed) => {
						let asked = ask.len();
						let result = translate_display(DisplayRequest {
							title: &title_segment.source,
							subtitle: subtitle_segment.as_ref().map(|s| s.source.as_str()),
							context: &context,
							wanted: ask,
							have: held,
							runner,
							model_override: model_override.clone(),
							source_locale: source_locale.clone(),
						})
						.await;
						budget = budget.saturating_sub(asked);
						match result {
							Ok((entries, tokens, usd, unfinished)) => {
								for (locale, field, entry) in entries {
									let Some(id) = ids.get(&field) else {
										continue;
									};
									sidecar.segments.entry(id.clone()).or_default().insert(locale, entry);
									outcome.translated += 1;
								}
								outcome.tokens += tokens;
								outcome.usd += usd;
								sidecar.version = store::VERSION;
								{
									let path = sidecar_path.clone();
									let snapshot = sidecar.clone();
									translations.apply(move || store::save(&path, &snapshot))?;
								}
								sidecar_seen = modified_at(&sidecar_path);
								if !unfinished.is_empty() {
									outcome.failed.push((
										"display".to_owned(),
										format!(
											"{} did not come back within budget",
											unfinished
												.iter()
												.map(|(locale, field)| format!("{locale}:{}", field.name()))
												.collect::<Vec<_>>()
												.join(", ")
										),
									));
								}
							}
							Err(Refusal::Exhausted(reason)) => {
								outcome.exhausted = Some(reason);
								drop(claimed);
								return Ok(outcome);
							}
							Err(error) => {
								outcome.failed.push(("display".to_owned(), error.to_string()));
							}
						}
						drop(claimed);
					}
				}
			}
		}

		let progress = progress::Progress::new(todo.len() as u64, sinks());
		progress.set_message(format!("{}", path.display()));

		// The claim on each in-flight segment, released when its result has been stored. Kept
		// beside the JoinSet rather than moved into the task so that a claim outlives the request
		// and covers the write as well: releasing at the end of the model call would let another
		// process start the same segment while this one was still saving it.
		let mut held: std::collections::HashMap<String, claim::Claim> =
			std::collections::HashMap::new();

		let mut queue = todo.into_iter();
		type Finished = (String, Result<(Vec<(String, Translation)>, u64, f64, Vec<String>), Refusal>);
		let mut running = tokio::task::JoinSet::<Finished>::new();

		loop {
			while running.len() < parallel {
				let Some((item, locales)) = queue.next() else {
					break;
				};

				// Claimed before anything is spent. A segment another process is translating right
				// now is left to it; the run reports it rather than paying for the same answer.
				let claimed = match claim::take(repository, "i18n", &format!("{article_key}#{}", item.id)) {
					Ok(claimed) => claimed,
					Err(claim::Denied::Taken(_)) => {
						outcome.claimed_elsewhere += 1;
						progress.inc(1);
						continue;
					}
					Err(claim::Denied::Io(error)) => return Err(error),
				};

				// The claim only stops two runs translating this segment at the same instant. A
				// run that finished it a moment ago and let go is invisible to the claim, and the
				// answer would simply be bought twice -- measured on favicons, where it cost a
				// request; here it costs the price of a translation. So the sidecar is re-read
				// whenever another process has touched it since we last looked, and a segment that
				// is no longer missing is dropped. See spec/tasks.md.
				let latest = modified_at(&sidecar_path);
				if latest != sidecar_seen {
					sidecar = store::load(&sidecar_path)?;
					sidecar_seen = latest;
				}
				let required = if item.region == segment::Region::Body {
					glosses.find(&item.id).map(|entry| entry.spans.as_slice()).unwrap_or_default()
				} else {
					&[]
				};
				let have = sidecar.segments.get(&item.id);
				if !force
					&& locales.iter().all(|locale| {
						!store::translation_missing(have.and_then(|map| map.get(locale)), required)
					}) {
					outcome.already_done += 1;
					progress.inc(1);
					drop(claimed);
					continue;
				}
				held.insert(item.id.clone(), claimed);

				progress.set_message(progress::preview(&item.source, 44));
				let (before, after) = neighbours(&item);
				let model_override = model_override.clone();
				let gloss = glosses.find(&item.id).cloned();
				let source_locale = source_locale.clone();
				let owned = item;
				running.spawn(async move {
					let id = owned.id.clone();
					(
						id,
						translate(
							&owned,
							before,
							after,
							TranslationOptions { runner, model_override, locales, source_locale, gloss },
						)
						.await,
					)
				});
			}
			if running.is_empty() {
				break;
			}

			let finished = match running.join_next().await {
				None => break,
				Some(Ok(result)) => result,
				Some(Err(error)) => (String::new(), Err(Refusal::Failed(error.to_string()))),
			};
			progress.inc(1);

			let (id, result) = finished;
			// Released once the result is in hand and about to be stored, not when the request
			// returned: the write below is part of the work this claim covers.
			let claimed = held.remove(&id);
			match result {
				Ok((entries, tokens, usd, lost)) => {
					let slot = sidecar.segments.entry(id.clone()).or_default();
					for (locale, entry) in entries {
						slot.insert(locale, entry);
						outcome.translated += 1;
					}
					outcome.tokens += tokens;
					outcome.usd += usd;
					// Written the moment it arrives. Every segment cost real money, and keeping a
					// run's worth in memory means one interrupt throws all of it away -- which is
					// exactly what happened the first time this ran for real.
					//
					// Through the writer, so the sidecar is never open in two places: a second CMS
					// translating a different segment of this same article would otherwise read,
					// change and write the file underneath this one, and one of the two paid
					// results would vanish. The lock is held for the write alone.
					sidecar.version = store::VERSION;
					{
						let path = sidecar_path.clone();
						let snapshot = sidecar.clone();
						translations.apply(move || store::save(&path, &snapshot))?;
					}
					sidecar_seen = modified_at(&sidecar_path);
					if !lost.is_empty() {
						outcome.failed.push((id, format!("{} did not survive validation", lost.join(", "))));
					}
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
			drop(claimed);
		}
		progress.finish_and_clear();
		// Anything still in flight when the loop ended keeps nothing: dropping the map releases
		// every remaining claim so an interrupted run does not leave items locked behind it.
		held.clear();
		articles_done += 1;
		let mut incomplete =
			store::missing(&sidecar, &live, locales, source_locale.as_deref(), &glosses);
		incomplete.retain(|id, _| live.get(id).is_some_and(|segment| scope.includes(segment)));
		outcome.incomplete_segments += incomplete.len();
		outcome.missing_locales += incomplete.values().map(Vec::len).sum::<usize>();
	}
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::BTreeMap;

	#[test]
	fn a_page_is_refused_by_name_and_an_article_is_not() {
		// The filter in `run` is what actually keeps pages out. This is the second line: it has to
		// name the file that arrived, so a run that suddenly reports one says why rather than just
		// failing somewhere further in.
		let refused = refuse_page(Path::new("contents/homepage.md")).unwrap_err();
		assert!(refused.contains(PAGE_FILE), "{refused}");
		assert!(refused.contains("spec/i18n.md"), "{refused}");

		assert!(refuse_page(Path::new("contents/architecture/homepage.md")).is_err());
		assert!(refuse_page(Path::new("contents/milestone/less-is-more.md")).is_ok());
		// Only the whole name. An article may end in those letters without being a page.
		assert!(refuse_page(Path::new("contents/notes/not-homepage.md")).is_ok());
	}

	#[test]
	fn parallelism_defaults_to_four_and_rejects_zero() {
		assert_eq!(parallelism(None).unwrap(), 4);
		assert_eq!(parallelism(Some("10")).unwrap(), 10);
		assert!(parallelism(Some("0")).is_err());
		assert!(parallelism(Some("many")).is_err());
	}

	#[test]
	fn locale_selection_is_validated_and_keeps_canonical_order() {
		assert_eq!(
			selected_locales(&["zh-TW".into(), "en-US".into(), "zh-TW".into()]).unwrap(),
			vec!["en-US", "zh-TW"]
		);
		assert!(selected_locales(&["mw".into()]).is_err());
	}

	#[test]
	fn frontmatter_scope_never_selects_body_prose() {
		let segments = segment::split(
			"---\ntitle: Visible title\nlang: en-US\n---\n\nBody that must stay out of this run.",
		)
		.expect("frontmatter");
		let selected =
			segments.iter().filter(|segment| Scope::Frontmatter.includes(segment)).collect::<Vec<_>>();

		// The title and the short form written from it, and nothing from the body.
		assert_eq!(selected.len(), 2);
		assert!(selected.iter().all(|segment| segment.source == "Visible title"));
		assert!(selected.iter().all(|segment| segment.region == segment::Region::Frontmatter));
		assert_eq!(
			selected.iter().map(|segment| segment.display).collect::<Vec<_>>(),
			vec![Some(segment::Display::Title), Some(segment::Display::ShortTitle)],
		);
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
		assert!(!validate::notes_well_formed("a :tn[word]{is=\"he said \"no\" loudly\"} b"));
		assert!(!validate::notes_well_formed("a :tn[word] b"));
		assert!(!validate::notes_well_formed("a :tn[word]{was=\"wrong key\"} b"));
	}

	#[test]
	fn a_frontmatter_note_is_refused_before_it_can_be_stored() {
		// Metadata has no rendering channel for the explanation. Prompt wording is not an
		// acceptance boundary, so a model that ignores it must enter the retry path here.
		let boundary = "F7Q2L9DM4KX8V1C6R0PB3HNS5WJATGEU";
		let reply =
			format!("{}\n:tn[Translated title]{{is=\"a gloss\"}}\n", prompt::locale_marker("en-US"),);

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
			"{}\n::linkcard{{src=\"b.avif\" url=\"https://other.example\" title=\"Two\"}}\n\
			 A paragraph that belongs to the block before this one.\n{source}\n",
			prompt::locale_marker("en-US"),
		);
		assert!(matches!(
			validate_reply(&echoed, boundary, segment::Region::Body, &segment::mask(source)),
			Err(Refusal::Failed(_))
		));

		// The same block answered on its own is kept, so the check costs nothing that is correct.
		let clean = format!(
			"{}\n::linkcard{{src=\"a.avif\" url=\"https://example.com\" title=\"Eins\"}}\n",
			prompt::locale_marker("de-DE"),
		);
		assert!(
			validate_reply(&clean, boundary, segment::Region::Body, &segment::mask(source)).is_ok()
		);
	}

	#[test]
	fn a_multi_line_block_may_keep_its_lines() {
		// A list translates line for line, so the rule is "no more than", never "exactly one".
		let boundary = "PQ9WZ4WX2TN7VLKD8RYC5MBFA1GHJ0SE";
		let source = "- first\n- second\n- third";
		let reply = format!("{}\n- erste\n- zweite\n- dritte\n", prompt::locale_marker("de-DE"),);
		assert!(
			validate_reply(&reply, boundary, segment::Region::Body, &segment::mask(source)).is_ok()
		);
	}

	#[test]
	fn an_exact_source_locale_may_be_rewritten_as_a_localised_view() {
		let boundary = "JQ8WZ4MX2TN7VLKD9RYC5PBFA1GH30SE";
		let source = "这是一段作者写下来的原文，它的措辞、节奏和判断都应该保持不变。";
		let polished = format!(
			"{}\n这是一段作者写下来的原文，它的措辞、节奏和判断都应该保持不变！\n",
			prompt::locale_marker("zh-CN"),
		);
		let rewritten = format!(
			"{}\n作者在这里主张，编辑应当完整保存文章的核心思想和表达方式。\n",
			prompt::locale_marker("zh-CN"),
		);
		let masked = segment::mask(source);

		assert!(
			validate_reply_for(
				&polished,
				boundary,
				segment::Kind::Prose,
				segment::Region::Body,
				source,
				&masked,
				&["zh-CN"],
				Some("zh-CN"),
			)
			.is_ok()
		);
		assert!(
			validate_reply_for(
				&rewritten,
				boundary,
				segment::Kind::Prose,
				segment::Region::Body,
				source,
				&masked,
				&["zh-CN"],
				Some("zh-CN"),
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
		let result = validate_reply(&reply, boundary, segment::Region::Body, &segment::mask("source"));
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
		assert!(!serde_yaml_ng::to_string(&sidecar).expect("sidecar").contains(boundary));
	}
}
