//! The command-line adapter for shared CMS operations. See spec/architecture.md.

use crate::{
	alt, articles, check, classify, derived, embed, favicon, gc, i18n, image, licenses, locale,
	opengraph, overview, paths, port, refs, summary, task, x,
};
use std::process::ExitCode;

pub fn run() -> ExitCode {
	let args: Vec<String> = std::env::args().skip(1).collect();
	match args.first().map(String::as_str) {
		Some("overview") => print_overview(),
		Some("articles") => print_articles(),
		Some("derived") => print_derived(),
		Some("tasks") => print_tasks(),
		Some("runs") => print_runs(),
		Some("port") => print_port(),
		Some("favicon") => fetch_favicons(&args[1..]),
		Some("image") => process_images(&args[1..]),
		Some("check") => check_assets(),
		Some("og") => render_cards(&args[1..]),
		Some("tag") => classify_images(&args[1..]),
		Some("segments") => write_segment_layout(),
		Some("i18n") => translate_articles(&args[1..]),
		Some("tn") => scan_notes(&args[1..]),
		Some("embed") => fetch_embeds(&args[1..]),
		Some("locale") => translate_locales(&args[1..]),
		Some("alt") => describe_images(&args[1..]),
		Some("summary") => summarise_articles(&args[1..]),
		Some("gc") => collect_garbage(&args[1..]),
		Some("licenses") => collect_licenses(),
		Some("x") => x_command(&args[1..]),
		Some(other) => {
			eprintln!("unknown command: {other}");
			usage();
			ExitCode::FAILURE
		}
		None => {
			usage();
			ExitCode::FAILURE
		}
	}
}

fn print_overview() -> ExitCode {
	match overview::snapshot() {
		Ok(snapshot) => match serde_json::to_string_pretty(&snapshot) {
			Ok(json) => {
				println!("{json}");
				ExitCode::SUCCESS
			}
			Err(error) => {
				eprintln!("could not encode overview: {error}");
				ExitCode::FAILURE
			}
		},
		Err(error) => {
			eprintln!("could not read overview: {error}");
			ExitCode::FAILURE
		}
	}
}

fn print_articles() -> ExitCode {
	match articles::listing() {
		Ok(listing) => match serde_json::to_string_pretty(&listing) {
			Ok(json) => {
				println!("{json}");
				ExitCode::SUCCESS
			}
			Err(error) => {
				eprintln!("could not encode the article listing: {error}");
				ExitCode::FAILURE
			}
		},
		Err(error) => {
			eprintln!("could not read the article listing: {error}");
			ExitCode::FAILURE
		}
	}
}

fn print_derived() -> ExitCode {
	match derived::report() {
		Ok(report) => match serde_json::to_string_pretty(&report) {
			Ok(json) => {
				println!("{json}");
				ExitCode::SUCCESS
			}
			Err(error) => {
				eprintln!("could not encode the derived report: {error}");
				ExitCode::FAILURE
			}
		},
		Err(error) => {
			eprintln!("could not read the derived report: {error}");
			ExitCode::FAILURE
		}
	}
}

fn print_tasks() -> ExitCode {
	match serde_json::to_string_pretty(task::CATALOG) {
		Ok(json) => {
			println!("{json}");
			ExitCode::SUCCESS
		}
		Err(error) => {
			eprintln!("could not encode the task catalogue: {error}");
			ExitCode::FAILURE
		}
	}
}

fn print_port() -> ExitCode {
	match port::from_env() {
		Ok(port) => {
			println!("{port}");
			ExitCode::SUCCESS
		}
		Err(error) => {
			eprintln!("{error}");
			ExitCode::FAILURE
		}
	}
}

/// Collect the icons the articles' linkcards need.
///
/// With no arguments this follows the articles, which is the normal way to run it: every
/// linkcard names a site, and a `favicon` attribute names where that site's icon should come
/// from. Nothing is rewritten afterwards -- the attribute is an instruction to this command,
/// while the page always draws `/favicon/{domain}`. Explicit domains remain accepted for
/// collecting one ahead of the article that will link to it.
fn fetch_favicons(args: &[String]) -> ExitCode {
	let mut force = false;
	let mut inputs: Vec<&str> = Vec::new();
	for arg in args {
		match arg.as_str() {
			"--force" => force = true,
			other => inputs.push(other),
		}
	}

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let wanted: Vec<refs::Wanted> = if inputs.is_empty() {
		match refs::scan(&root.join("contents")) {
			Ok(scan) => scan.wanted(),
			Err(error) => {
				eprintln!("could not read articles: {error}");
				return ExitCode::FAILURE;
			}
		}
	} else {
		favicon::host::normalise(inputs)
			.into_iter()
			.map(|domain| refs::Wanted {
				domain,
				source: None,
				tone: None,
			})
			.collect()
	};

	if wanted.is_empty() {
		println!("no linkcards ask for an icon");
		return ExitCode::SUCCESS;
	}

	let outcome = match favicon::collect::run(favicon::collect::Options {
		repository: &root,
		wanted: &wanted,
		force,
		shell: task::registry::Shell::Cli,
		sink: Box::new(task::progress::Terminal::new()),
	}) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("could not collect icons: {error}");
			return ExitCode::FAILURE;
		}
	};

	for (domain, reason) in &outcome.failed {
		eprintln!("fail  {domain}: {reason}");
	}
	// Named rather than counted: "three were somebody else's" is a different thing from "three
	// failed", and a person deciding whether to rerun needs to know which domains to expect.
	for domain in &outcome.claimed_elsewhere {
		println!("held  {domain} (another run has it)");
	}
	println!(
		"{} collected, {} already present, {} failed, {} held elsewhere",
		outcome.collected,
		outcome.skipped,
		outcome.failed.len(),
		outcome.claimed_elsewhere.len()
	);
	ExitCode::SUCCESS
}

/// What is running anywhere on this machine for this repository.
fn print_runs() -> ExitCode {
	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let runs = match task::registry::live(&root) {
		Ok(runs) => runs,
		Err(error) => {
			eprintln!("could not read the run registry: {error}");
			return ExitCode::FAILURE;
		}
	};
	match serde_json::to_string_pretty(&runs) {
		Ok(json) => {
			println!("{json}");
			ExitCode::SUCCESS
		}
		Err(error) => {
			eprintln!("could not encode the run registry: {error}");
			ExitCode::FAILURE
		}
	}
}

fn selected_runner(
	args: &[String],
	default: i18n::runner::Runner,
) -> Result<i18n::runner::Runner, ExitCode> {
	let Some(at) = args.iter().position(|arg| arg == "--model") else {
		return Ok(default);
	};
	let Some(runner) = args
		.get(at + 1)
		.and_then(|name| i18n::runner::Runner::parse(name))
	else {
		eprintln!("--model takes {}", i18n::runner::CHOICES);
		return Err(ExitCode::FAILURE);
	};
	Ok(runner)
}

fn option_value<'a>(args: &'a [String], option: &str) -> Result<Option<&'a str>, ExitCode> {
	let Some(at) = args.iter().position(|arg| arg == option) else {
		return Ok(None);
	};
	let Some(value) = args.get(at + 1).filter(|value| !value.starts_with('-')) else {
		eprintln!("{option} takes a value");
		return Err(ExitCode::FAILURE);
	};
	Ok(Some(value))
}

fn selected_model_override(
	args: &[String],
	runner: i18n::runner::Runner,
) -> Result<Option<String>, ExitCode> {
	let model = option_value(args, "--model-id")?;
	let effort = option_value(args, "--effort")?;
	i18n::runner::model_override(runner, model, effort).map_err(|error| {
		eprintln!("{error}");
		ExitCode::FAILURE
	})
}

/// Write a reader-facing summary into every article that has none.
///
/// The value lands in a sidecar beside the article, in the article's own language. `cms locale`
/// translates it into the other locales afterwards.
fn summarise_articles(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());
	// Not `DEFAULT_TEXT`. This is the one text task carrying a constraint the model has to hold
	// against its own training -- summarise, but withhold the conclusion -- and the open-weight
	// default measurably does not: it handed over the whole design and then appended "reaches a
	// surprising conclusion", and gave a first-person essay's author a pronoun the article never
	// uses. Translation has no comparable trap, which is why that one stays on the cheap model.
	let runner = match selected_runner(args, i18n::runner::Runner::Codex) {
		Ok(runner) => runner,
		Err(code) => return code,
	};
	let model_override = match selected_model_override(args, runner) {
		Ok(model) => model,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(summary::run(summary::Options {
		repository: &root,
		runner,
		model_override,
		force,
		limit,
		shell: task::registry::Shell::Cli,
		sink: Box::new(task::progress::Terminal::new()),
	})) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	for (path, error) in &outcome.failed {
		eprintln!("fail  {path}: {error}");
	}
	if outcome.claimed_elsewhere > 0 {
		eprintln!(
			"note  {} left to a run already summarising them",
			outcome.claimed_elsewhere
		);
	}
	println!(
		"{} written, {} already had one, {} reviewed, {} deferred, {} failed",
		outcome.written,
		outcome.skipped,
		outcome.reviewed,
		outcome.deferred,
		outcome.failed.len()
	);
	if outcome.written > 0 {
		let spent = outcome.spent;
		println!("{} in, ${:.2}", spent.total_in(), spent.usd);
		println!("run `cms locale` to translate the new values");
	}
	if outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Describe every asset that has no description yet.
///
/// The description is written into the manifest, so it belongs to the picture rather than to
/// whichever article happened to be open when it was generated. Every reference inherits it,
/// including ones written later.
fn describe_images(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());
	let runner = match selected_runner(args, i18n::runner::DEFAULT_VISION) {
		Ok(runner) => runner,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let originals = root.join("data").join("image");
	let public = root.join("data").join("public");
	let merged = match image::run::load(&root.join(image::run::MERGED)) {
		Ok(merged) => merged,
		Err(error) => {
			eprintln!("could not read {}: {error}", image::run::MERGED);
			return ExitCode::FAILURE;
		}
	};
	// A runtime only for this command. Everything else here is a local file walk that gains
	// nothing from one; this is the single place where the work is waiting on somebody else.
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(alt::run(alt::Options {
		repository: &root,
		runner,
		merged: &merged,
		originals: &originals,
		force,
		limit,
		shell: task::registry::Shell::Cli,
		sink: Box::new(task::progress::Terminal::new()),
	})) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	for (cid, error) in &outcome.failed {
		eprintln!("fail  {cid}: {error}");
	}
	// Reported rather than fatal: an asset whose original is gone can still be served, it just
	// cannot be looked at again.
	for cid in &outcome.unreadable {
		eprintln!("warn  no original on hand for {cid}");
	}
	if outcome.claimed_elsewhere > 0 {
		eprintln!(
			"note  {} left to a run already describing them",
			outcome.claimed_elsewhere
		);
	}
	let _ = public;

	println!(
		"{} described, {} already had one, {} left by --limit, {} failed",
		outcome.described,
		outcome.skipped,
		outcome.deferred,
		outcome.failed.len()
	);
	if outcome.described > 0 {
		let spent = outcome.spent;
		println!(
			"{} in ({} fresh, {} cached, {} written), {} out, ${:.2}",
			spent.total_in(),
			spent.input,
			spent.cache_read,
			spent.cache_written,
			spent.output,
			spent.usd
		);
	}
	if outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Translate every article segment that has no translation yet.
///
/// One request covers one segment's missing locales, so an edited paragraph costs one call while
/// a partial repair does not repay for completed languages.
fn translate_articles(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let check = args.iter().any(|arg| arg == "--check");
	let scope = if args.iter().any(|arg| arg == "--frontmatter") {
		i18n::Scope::Frontmatter
	} else {
		i18n::Scope::All
	};
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());
	let parallel = match option_value(args, "--parallel").and_then(|value| {
		i18n::parallelism(value).map_err(|error| {
			eprintln!("{error}");
			ExitCode::FAILURE
		})
	}) {
		Ok(parallel) => parallel,
		Err(code) => return code,
	};
	let mut locale_values = Vec::new();
	for (at, arg) in args.iter().enumerate() {
		if arg == "--locale" {
			let Some(value) = args.get(at + 1).filter(|value| !value.starts_with('-')) else {
				eprintln!("--locale takes a value");
				return ExitCode::FAILURE;
			};
			locale_values.push(value.clone());
		}
	}
	let locales = match i18n::selected_locales(&locale_values) {
		Ok(locales) => locales,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let mut only: Vec<std::path::PathBuf> = Vec::new();
	let mut runner = i18n::runner::DEFAULT_TEXT;
	let mut skip = false;
	for (at, arg) in args.iter().enumerate() {
		if skip {
			skip = false;
			continue;
		}
		match arg.as_str() {
			"--force" | "--frontmatter" | "--check" => {}
			"--limit" | "--parallel" | "--model-id" | "--effort" | "--locale" => skip = true,
			"--model" => {
				skip = true;
				match args
					.get(at + 1)
					.and_then(|name| i18n::runner::Runner::parse(name))
				{
					Some(chosen) => runner = chosen,
					None => {
						eprintln!("--model takes {}", i18n::runner::CHOICES);
						return ExitCode::FAILURE;
					}
				}
			}
			other => only.push(std::path::PathBuf::from(other)),
		}
	}
	let model_override = match selected_model_override(args, runner) {
		Ok(model) => model,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	if let Err(error) = i18n::layout::sync(&root) {
		eprintln!("could not write {}: {error}", i18n::layout::FILE);
		return ExitCode::FAILURE;
	}

	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(i18n::run(
		&root.join("contents"),
		&only,
		i18n::RunOptions {
			runner,
			model_override,
			limit,
			parallel,
			force,
			scope,
			locales: &locales,
			check,
			repository: &root,
			shell: task::registry::Shell::Cli,
			sinks: Box::new(|| Box::new(task::progress::Terminal::new())),
		},
	)) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("could not write: {error}");
			return ExitCode::FAILURE;
		}
	};

	for (id, error) in &outcome.failed {
		eprintln!("fail  {id}: {error}");
	}
	// Reported rather than swept here: an edited paragraph leaves its old translation behind,
	// and that text is usually still worth reading before it goes.
	if outcome.orphans > 0 {
		eprintln!("note  {} stale segments left by edits", outcome.orphans);
	}
	// Not a failure. The work done is kept, and running again after the reset picks up exactly
	// where this stopped, because only missing segments are ever requested.
	if let Some(reason) = &outcome.exhausted {
		println!("stopped: {reason}");
	}
	println!(
		"{} translations across {} segments, {} failed; {} incomplete segments ({} missing locale entries)",
		outcome.translated,
		outcome.segments,
		outcome.failed.len(),
		outcome.incomplete_segments,
		outcome.missing_locales,
	);
	if outcome.translated > 0 {
		println!("{} tokens, ${:.2}", outcome.tokens, outcome.usd);
	}
	// A spent allowance is a normal state to stop in, not an error to report as one.
	if outcome.failed.is_empty()
		&& (outcome.incomplete_segments == 0 || limit.is_some() || outcome.exhausted.is_some())
	{
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Materialise the Rust segment ids and source ranges for builds that do not have Rust.
fn write_segment_layout() -> ExitCode {
	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	match i18n::layout::sync(&root) {
		Ok(true) => println!("wrote {}", i18n::layout::FILE),
		Ok(false) => println!("{} unchanged", i18n::layout::FILE),
		Err(error) => {
			eprintln!("could not write {}: {error}", i18n::layout::FILE);
			return ExitCode::FAILURE;
		}
	}
	ExitCode::SUCCESS
}

/// Translate tag labels and image descriptions from their English source text.
fn translate_locales(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());
	let runner = match selected_runner(args, i18n::runner::DEFAULT_TEXT) {
		Ok(runner) => runner,
		Err(code) => return code,
	};
	let model_override = match selected_model_override(args, runner) {
		Ok(model) => model,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(locale::run(&root, runner, model_override, force, limit)) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("could not write: {error}");
			return ExitCode::FAILURE;
		}
	};

	for (id, error) in &outcome.failed {
		eprintln!("fail  {id}: {error}");
	}
	if let Some(reason) = &outcome.exhausted {
		println!("stopped: {reason}");
	}
	println!(
		"{} translations across {} sources, {} already present, {} left by --limit, {} failed",
		outcome.translated,
		outcome.sources,
		outcome.skipped,
		outcome.deferred,
		outcome.failed.len()
	);
	if outcome.translated > 0 {
		println!("{} tokens, ${:.2}", outcome.tokens, outcome.usd);
	}
	// Exhaustion is a normal stopping point even if an earlier independent unit failed.
	if outcome.exhausted.is_some() || outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Render one OpenGraph card per article.
///
/// Nothing references these: the page emits `/opengraph/{slug}.png` and no article writes the
/// URL down, so there is no reference to rewrite and the slug is the name.
fn render_cards(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	// The site name, the author and their role all come from the file the pages read them
	// from, so a card and the page it belongs to cannot introduce the site differently.
	let outcome = match opengraph::run(
		&root,
		&root.join("data").join("public"),
		&root.join("contents"),
		force,
	) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	for (slug, error) in &outcome.failed {
		eprintln!("fail  {slug}: {error}");
	}
	println!(
		"{} rendered, {} already present, {} failed",
		outcome.rendered,
		outcome.skipped,
		outcome.failed.len()
	);
	if outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Give every asset a category and a handful of tags.
fn classify_images(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());
	let runner = match selected_runner(args, i18n::runner::DEFAULT_VISION) {
		Ok(runner) => runner,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(classify::run(classify::Options {
		repository: &root,
		runner,
		force,
		limit,
		shell: task::registry::Shell::Cli,
		sink: Box::new(task::progress::Terminal::new()),
	})) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("could not write: {error}");
			return ExitCode::FAILURE;
		}
	};

	for (cid, error) in &outcome.failed {
		eprintln!("fail  {cid}: {error}");
	}
	for cid in &outcome.unreadable {
		eprintln!("warn  no original on hand for {cid}");
	}
	if !outcome.minted.is_empty() {
		println!("new tags: {}", outcome.minted.join(", "));
	}
	if let Some(reason) = &outcome.exhausted {
		println!("stopped: {reason}");
	}
	println!(
		"{} classified, {} already done, {} failed",
		outcome.classified,
		outcome.skipped,
		outcome.failed.len()
	);
	if outcome.classified > 0 {
		println!("{} tokens, ${:.2}", outcome.tokens, outcome.usd);
	}
	if outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// Report what the articles reference and `data/public` cannot answer for.
///
/// Always succeeds. This is a report, and a report that can fail a build is a gate wearing a
/// report's name.
fn check_assets() -> ExitCode {
	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let gaps = match check::report(
		&root,
		&root.join("data").join("public"),
		&root.join("contents"),
	) {
		Ok(gaps) => gaps,
		Err(error) => {
			eprintln!("could not read articles: {error}");
			return ExitCode::FAILURE;
		}
	};

	if gaps.is_empty() {
		println!("every referenced asset is present");
		return ExitCode::SUCCESS;
	}

	for gap in &gaps {
		let action = gap
			.action
			.map(|action| format!(" -- run cms {}", action.command()))
			.unwrap_or_default();
		println!(
			"{}  {}: {}{action}",
			gap.level.label(),
			gap.what,
			gap.detail
		);
	}
	let warnings = gaps
		.iter()
		.filter(|gap| gap.level == check::Level::Warn)
		.count();
	println!("{} missing, {warnings} of them images", gaps.len());
	ExitCode::SUCCESS
}

/// Drop everything in `data/public` that no article asks for.
///
/// Dry by default. The listing is the review, and `--live` is the answer to it.
fn collect_garbage(args: &[String]) -> ExitCode {
	let live = args.iter().any(|arg| arg == "--live");

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let public = root.join("data").join("public");

	let sweep = match gc::plan(&root, &public, &root.join("contents")) {
		Ok(sweep) => sweep,
		Err(error) => {
			eprintln!("could not plan: {error}");
			return ExitCode::FAILURE;
		}
	};

	if sweep.orphans.is_empty() && sweep.entries.is_empty() {
		println!("nothing to collect");
		return ExitCode::SUCCESS;
	}

	for path in &sweep.orphans {
		let shown = path.strip_prefix(&root).unwrap_or(path);
		println!("drop  {}", shown.display());
	}
	for cid in &sweep.entries {
		println!("drop  {} from metadata.json", cid);
	}
	println!(
		"{} objects, {} manifest entries, {:.1} MiB",
		sweep.orphans.len(),
		sweep.entries.len(),
		sweep.bytes as f64 / (1024.0 * 1024.0)
	);

	if !live {
		println!("dry run -- pass --live to delete");
		return ExitCode::SUCCESS;
	}
	if let Err(error) = gc::apply(&root, &sweep) {
		eprintln!("could not delete: {error}");
		return ExitCode::FAILURE;
	}
	println!("collected");
	ExitCode::SUCCESS
}

/// Collect the licence of everything the deployables are built out of.
///
/// Runs locally and nowhere else. The crate half reads the cargo registry cache, which no CI
/// container has and no Rust toolchain on the site's build image would populate, so the record
/// has to be produced here and committed. That settles the npm half too: one command, one
/// record, one diff to review, rather than half the answer arriving at build time.
///
/// A package that declares no licence at all fails the run. It is the one finding that needs a
/// person -- everything else the record can state plainly -- and a report that scrolls past is
/// how a dependency with no terms ends up shipped.
fn collect_licenses() -> ExitCode {
	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let public = root.join("data").join("public");

	let mut found = match licenses::npm::collect(&root) {
		Ok(found) => found,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let npm = found.len();
	match licenses::cargo::collect(&root) {
		Ok(crates) => found.extend(crates),
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	}
	println!("{npm} npm packages, {} crates", found.len() - npm);

	let assertions = match licenses::read_assertions(&root) {
		Ok(assertions) => assertions,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let written = match licenses::write(&public, found, &assertions) {
		Ok(written) => written,
		Err(error) => {
			eprintln!("could not publish licence texts: {error}");
			return ExitCode::FAILURE;
		}
	};

	let document = match licenses::full_document(&public, &written.record) {
		Ok(document) => document,
		Err(error) => {
			eprintln!("could not assemble the full notice: {error}");
			return ExitCode::FAILURE;
		}
	};
	if let Err(error) = image::store::write(&licenses::full_path(&public), document.as_bytes()) {
		eprintln!("could not write the full notice: {error}");
		return ExitCode::FAILURE;
	}

	let record_path = licenses::record_path(&root);
	let json = match serde_json::to_string_pretty(&written.record) {
		Ok(json) => json,
		Err(error) => {
			eprintln!("could not serialise the record: {error}");
			return ExitCode::FAILURE;
		}
	};
	if let Err(error) = image::store::write(&record_path, format!("{json}\n").as_bytes()) {
		eprintln!("could not write {}: {error}", record_path.display());
		return ExitCode::FAILURE;
	}

	println!(
		"{} unique texts, {} KiB in the full notice",
		written.objects,
		document.len() / 1024
	);
	for purl in &written.stale {
		println!("stale assertion, the package now declares its own or is gone: {purl}");
	}
	if !written.textless.is_empty() {
		println!(
			"{} packages declare terms but ship no text",
			written.textless.len()
		);
	}
	println!("wrote data/build/licenses.json");

	if !written.undeclared.is_empty() {
		eprintln!();
		for purl in &written.undeclared {
			eprintln!("no license declared: {purl}");
		}
		eprintln!(
			"{} packages declare no license at all -- decide about each before shipping them",
			written.undeclared.len()
		);
		return ExitCode::FAILURE;
	}
	ExitCode::SUCCESS
}

/// Derive and publish every image the articles ask for, then rewrite what they say.
///
/// With no file arguments this follows the articles. Named files are imported ahead of the
/// article that will use them, which is how `--original` gets attached to a photograph before
/// anything references it.
fn process_images(args: &[String]) -> ExitCode {
	let mut force = false;
	let mut keep_original = false;
	let mut only: Vec<std::path::PathBuf> = Vec::new();

	for arg in args {
		match arg.as_str() {
			"--force" => force = true,
			"--original" => keep_original = true,
			other => only.push(std::path::PathBuf::from(other)),
		}
	}

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let originals = root.join("data").join("image");
	let public = root.join("data").join("public");
	let articles = root.join("contents");

	let options = image::run::Options {
		force,
		keep_original,
		only: &only,
	};
	let outcome = match image::run::run(&root, &originals, &public, &articles, &options) {
		Ok(outcome) => outcome,
		Err(error) => {
			eprintln!("could not write: {error}");
			return ExitCode::FAILURE;
		}
	};

	for (path, error) in &outcome.failed {
		eprintln!("fail  {}: {error}", path.display());
	}
	// Reported, not fatal. An article may be written before its picture is dropped in, and
	// `cms check` is where the whole list lives.
	for value in &outcome.missing {
		eprintln!("warn  no original for {value}");
	}
	println!(
		"{} derived, {} sidecars rewritten, {} unchanged, {} failed, {} references rewritten",
		outcome.processed,
		outcome.migrated,
		outcome.skipped,
		outcome.failed.len(),
		outcome.rewritten
	);

	if outcome.failed.is_empty() {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	}
}

/// The `cms tn` command: which passages a translation will have to keep and explain.
///
/// Whether a passage needs a note depends on whether the rest of the article already carries
/// its meaning, which is a judgement about the whole text. Translation happens one block at a
/// time and structurally cannot make it -- four articles produced no notes at all until this
/// was split out. So a strong model reads the article whole, and what it finds is reviewed
/// before it steers anything. See spec/i18n.md.
///
/// FIXME: this is the operation, not an adapter for one. spec/architecture.md gives every CMS
/// capability one in-process application operation with a CLI and a GUI adapter over it, and the
/// work below -- argument handling aside -- belongs beside the module it drives rather than in
/// this file. It is left here deliberately rather than exempted in the spec: moving it is the
/// same edit as putting it under the task substrate, and both wait on the desktop shell reaching
/// the point where it offers this command. Whoever gets there first should do the two together.
fn scan_notes(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let mut only: Vec<std::path::PathBuf> = Vec::new();
	let mut runner = i18n::runner::DEFAULT_VISION;
	let mut skip = false;
	for (at, arg) in args.iter().enumerate() {
		if skip {
			skip = false;
			continue;
		}
		match arg.as_str() {
			"--force" => {}
			"--model-id" | "--effort" => skip = true,
			"--model" => {
				skip = true;
				match args
					.get(at + 1)
					.and_then(|name| i18n::runner::Runner::parse(name))
				{
					Some(chosen) => runner = chosen,
					None => {
						eprintln!("--model takes {}", i18n::runner::CHOICES);
						return ExitCode::FAILURE;
					}
				}
			}
			other => only.push(std::path::PathBuf::from(other)),
		}
	}
	let model_override = match selected_model_override(args, runner) {
		Ok(model) => model,
		Err(code) => return code,
	};

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};

	let contents = root.join("contents");
	let path = i18n::tn::path_for(&root);
	let mut table = match i18n::tn::load(&path) {
		Ok(table) => table,
		Err(error) => {
			eprintln!("could not read {}: {error}", path.display());
			return ExitCode::FAILURE;
		}
	};
	// Pages without `lang` are not articles. Keep them out of both new scans and the durable
	// registry, including records written by older versions of this command.
	let recorded = table.articles.len();
	table.articles.retain(|key, _| {
		std::fs::read_to_string(contents.join(key))
			.map(|source| summary::lang_of(&source).is_some())
			// A missing source cannot prove the record belongs to a page. Preserve paid history
			// until a source file exists that positively identifies itself as one.
			.unwrap_or(true)
	});
	if table.articles.len() != recorded
		&& let Err(error) = i18n::tn::save(&path, &table)
	{
		eprintln!("could not write {}: {error}", path.display());
		return ExitCode::FAILURE;
	}

	// Named articles, or every one not yet read. An article scanned and found to need nothing
	// still counts as read, which is the distinction the table records so that a rerun does not
	// pay to learn the same nothing twice.
	let wanted: Vec<std::path::PathBuf> = if only.is_empty() {
		match refs::markdown_under(&contents) {
			Ok(all) => all,
			Err(error) => {
				eprintln!("could not read {}: {error}", contents.display());
				return ExitCode::FAILURE;
			}
		}
	} else {
		only
			.into_iter()
			.map(|item| {
				if item.is_absolute() {
					item
				} else {
					root.join(item)
				}
			})
			.collect()
	}
	.into_iter()
	.filter(|article| {
		std::fs::read_to_string(article)
			.ok()
			.and_then(|source| summary::lang_of(&source))
			.is_some()
	})
	.collect();

	let mut suggested = 0usize;
	let mut spent = 0u64;
	let mut read = 0usize;

	// Counted over everything named, including articles already read: a bar that shrank as it
	// skipped would report a total that had never been true.
	let progress = task::progress::Progress::new_terminal(wanted.len() as u64);
	for article in &wanted {
		let key = article
			.strip_prefix(&contents)
			.unwrap_or(article)
			.to_string_lossy()
			.replace('\\', "/");
		progress.set_message(key.clone());
		if !force && table.scanned(&key) {
			progress.inc(1);
			continue;
		}
		let text = match std::fs::read_to_string(article) {
			Ok(text) => text,
			Err(error) => {
				progress.suspend(|| eprintln!("fail  {key}: {error}"));
				progress.inc(1);
				continue;
			}
		};
		let (found, model, tokens) =
			match runtime.block_on(i18n::tn::scan(&text, runner, model_override.as_deref())) {
				Ok(result) => result,
				Err(error) => {
					progress.suspend(|| eprintln!("fail  {key}: {error}"));
					progress.inc(1);
					continue;
				}
			};
		spent += tokens;
		read += 1;

		let segments = i18n::segment::split(&text);
		let attached = i18n::tn::attach(&segments, &found);
		let mut entries = std::collections::BTreeMap::new();
		progress.suspend(|| {
			println!("{key}");
			if attached.is_empty() {
				println!("  nothing worth a note");
			}
			for (id, _, spans) in &attached {
				println!("  {}", &id[..12.min(id.len())]);
				for span in spans {
					println!("    {}  --  {}", span.phrase, span.guidance);
				}
			}
		});
		for (id, source, spans) in attached {
			suggested += spans.len();
			entries.insert(id, i18n::tn::Entry { source, spans });
		}
		// Recorded on sight, findings or none. The scan is paid for either way, so printing
		// without writing would mean reading the article twice to act on it once. Review is
		// deleting an entry you disagree with, which costs nothing; re-scanning does not.
		table.articles.insert(
			key,
			i18n::tn::Article {
				provider: runner.provider().to_owned(),
				model: i18n::model::normalise(&model),
				at: image::manifest::now(),
				tokens,
				segments: entries,
			},
		);
		// Written after each article rather than once at the end. An interrupted run otherwise
		// discards every article it had already paid to read, which this repository has been
		// bitten by before: a paid result is a purchase, not an intermediate.
		if let Err(error) = i18n::tn::save(&path, &table) {
			progress.suspend(|| eprintln!("could not write {}: {error}", path.display()));
			return ExitCode::FAILURE;
		}
		progress.inc(1);
	}
	progress.finish_and_clear();

	if read == 0 {
		println!("every article already read; pass --force to read one again");
		return ExitCode::SUCCESS;
	}
	println!("{read} read, {suggested} suggestions in data/tn.yaml; delete any you disagree with");
	println!("{spent} tokens");
	ExitCode::SUCCESS
}

/// The `cms embed` command: the crate trees and repository facts the articles show.
///
/// Fetched here rather than in the browser, so a page renders from a checkout with no proxy
/// route, no request per reader and no key. Both records rebuild from what git already holds,
/// which is what puts them under `data/build/`. See spec/architecture.md.
///
/// FIXME: this is the operation, not an adapter for one. spec/architecture.md gives every CMS
/// capability one in-process application operation with a CLI and a GUI adapter over it, and the
/// work below -- argument handling aside -- belongs beside the module it drives rather than in
/// this file. It is left here deliberately rather than exempted in the spec: moving it is the
/// same edit as putting it under the task substrate, and both wait on the desktop shell reaching
/// the point where it offers this command. Whoever gets there first should do the two together.
fn fetch_embeds(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let mut crates = embed::Crates::default();
	let mut repos = embed::Repos::default();
	if !force {
		if let Ok(text) = std::fs::read_to_string(embed::crates_path(&root)) {
			crates = serde_json::from_str(&text).unwrap_or_default();
		}
		if let Ok(text) = std::fs::read_to_string(embed::repos_path(&root)) {
			repos = serde_json::from_str(&text).unwrap_or_default();
		}
	}

	let articles = match refs::markdown_under(&root.join("contents")) {
		Ok(articles) => articles,
		Err(error) => {
			eprintln!("could not read contents: {error}");
			return ExitCode::FAILURE;
		}
	};
	let mut want = embed::Wanted::default();
	for path in &articles {
		if let Ok(text) = std::fs::read_to_string(path) {
			let found = embed::wanted(&text);
			want.crates.extend(found.crates);
			want.repos.extend(found.repos);
		}
	}
	want.crates.sort();
	want.crates.dedup();
	want.repos.sort();
	want.repos.dedup();

	let todo: Vec<&String> = want
		.crates
		.iter()
		.filter(|name| !crates.crates.contains_key(*name))
		.collect();
	let todo_repos: Vec<&String> = want
		.repos
		.iter()
		.filter(|name| !repos.repos.contains_key(*name))
		.collect();

	let progress = task::progress::Progress::new_terminal((todo.len() + todo_repos.len()) as u64);
	let mut failed = 0usize;
	for name in todo {
		progress.set_message(name.clone());
		match embed::fetch::krate(name) {
			Some(resolved) => {
				progress.suspend(|| {
					println!(
						"  {name} {} -- {} deps",
						resolved.version,
						resolved.deps.len()
					);
				});
				crates.crates.insert(name.clone(), resolved);
			}
			None => {
				failed += 1;
				progress.suspend(|| eprintln!("fail  {name}: not on the index"));
			}
		}
		progress.inc(1);
	}
	for name in todo_repos {
		progress.set_message(name.clone());
		match embed::fetch::repo(name) {
			Some(found) => {
				progress.suspend(|| println!("  {name} -- {} stars", found.stars));
				repos.repos.insert(name.clone(), found);
			}
			None => {
				failed += 1;
				progress.suspend(|| eprintln!("fail  {name}: GitHub did not answer"));
			}
		}
		progress.inc(1);
	}
	progress.finish_and_clear();

	if let Err(error) = image::store::write(
		&embed::crates_path(&root),
		serde_json::to_string_pretty(&crates)
			.unwrap_or_default()
			.as_bytes(),
	) {
		eprintln!("could not write crates.json: {error}");
		return ExitCode::FAILURE;
	}
	if let Err(error) = image::store::write(
		&embed::repos_path(&root),
		serde_json::to_string_pretty(&repos)
			.unwrap_or_default()
			.as_bytes(),
	) {
		eprintln!("could not write repos.json: {error}");
		return ExitCode::FAILURE;
	}

	println!(
		"{} crates, {} repositories, {failed} failed",
		crates.crates.len(),
		repos.repos.len()
	);
	ExitCode::SUCCESS
}

fn x_command(args: &[String]) -> ExitCode {
	match args.first().map(String::as_str) {
		Some("user") => x_user(&args[1..]),
		Some("keyword") => x_keyword(&args[1..]),
		Some("thread") => x_thread(&args[1..]),
		Some("semantic") => x_semantic(&args[1..]),
		Some(other) => {
			eprintln!("unknown x command: {other}");
			x_usage();
			ExitCode::FAILURE
		}
		None => {
			x_usage();
			ExitCode::FAILURE
		}
	}
}

fn x_usage() {
	eprintln!("usage: cms x <user|keyword|thread|semantic>");
}

fn x_user(args: &[String]) -> ExitCode {
	let mut count = x::DEFAULT_COUNT;
	let mut query = Vec::new();
	let mut index = 0;
	while index < args.len() {
		match args[index].as_str() {
			"--count" => match next_value(args, index, "--count") {
				Ok(value) => {
					count = match value.parse() {
						Ok(value) => value,
						Err(_) => {
							eprintln!("--count takes a positive integer");
							return ExitCode::FAILURE;
						}
					};
					index += 2;
				}
				Err(code) => return code,
			},
			other if other.starts_with('-') => {
				eprintln!("unknown option: {other}");
				return ExitCode::FAILURE;
			}
			other => {
				query.push(other);
				index += 1;
			}
		}
	}
	let query = query.join(" ");
	run_x(x::users(&query, count), "user search")
}

fn x_keyword(args: &[String]) -> ExitCode {
	let mut limit = x::DEFAULT_LIMIT;
	let mut mode = x::Mode::default();
	let mut query = Vec::new();
	let mut index = 0;
	while index < args.len() {
		match args[index].as_str() {
			"--limit" => match next_value(args, index, "--limit") {
				Ok(value) => {
					limit = match value.parse() {
						Ok(value) => value,
						Err(_) => {
							eprintln!("--limit takes a positive integer");
							return ExitCode::FAILURE;
						}
					};
					index += 2;
				}
				Err(code) => return code,
			},
			"--mode" => match next_value(args, index, "--mode") {
				Ok(value) => {
					mode = match x::Mode::parse(value) {
						Some(mode) => mode,
						None => {
							eprintln!("--mode takes Top or Latest");
							return ExitCode::FAILURE;
						}
					};
					index += 2;
				}
				Err(code) => return code,
			},
			other if other.starts_with('-') => {
				eprintln!("unknown option: {other}");
				return ExitCode::FAILURE;
			}
			other => {
				query.push(other);
				index += 1;
			}
		}
	}
	let query = query.join(" ");
	run_x(x::keyword(&query, limit, mode), "keyword search")
}

fn x_thread(args: &[String]) -> ExitCode {
	let mut post_id = None;
	for arg in args {
		if arg.starts_with('-') {
			eprintln!("unknown option: {arg}");
			return ExitCode::FAILURE;
		}
		if post_id.replace(arg.as_str()).is_some() {
			eprintln!("x thread takes one post id");
			return ExitCode::FAILURE;
		}
	}
	let Some(post_id) = post_id else {
		eprintln!("x thread takes a post id");
		return ExitCode::FAILURE;
	};
	run_x(x::thread(post_id), "thread")
}

fn x_semantic(args: &[String]) -> ExitCode {
	let mut options = x::Semantic::new("");
	let mut query = Vec::new();
	let mut index = 0;
	while index < args.len() {
		match args[index].as_str() {
			"--limit" => match next_value(args, index, "--limit") {
				Ok(value) => {
					options.limit = match value.parse() {
						Ok(value) => value,
						Err(_) => {
							eprintln!("--limit takes a positive integer");
							return ExitCode::FAILURE;
						}
					};
					index += 2;
				}
				Err(code) => return code,
			},
			"--from" => match next_value(args, index, "--from") {
				Ok(value) => {
					options.from_date = Some(value.to_owned());
					index += 2;
				}
				Err(code) => return code,
			},
			"--to" => match next_value(args, index, "--to") {
				Ok(value) => {
					options.to_date = Some(value.to_owned());
					index += 2;
				}
				Err(code) => return code,
			},
			"--user" => match next_value(args, index, "--user") {
				Ok(value) => {
					options.usernames.push(value.to_owned());
					index += 2;
				}
				Err(code) => return code,
			},
			"--exclude-user" => match next_value(args, index, "--exclude-user") {
				Ok(value) => {
					options.exclude_usernames.push(value.to_owned());
					index += 2;
				}
				Err(code) => return code,
			},
			"--min-score" => match next_value(args, index, "--min-score") {
				Ok(value) => {
					options.min_score = match value.parse() {
						Ok(value) => value,
						Err(_) => {
							eprintln!("--min-score takes a number");
							return ExitCode::FAILURE;
						}
					};
					index += 2;
				}
				Err(code) => return code,
			},
			other if other.starts_with('-') => {
				eprintln!("unknown option: {other}");
				return ExitCode::FAILURE;
			}
			other => {
				query.push(other);
				index += 1;
			}
		}
	}
	options.query = query.join(" ");
	run_x(x::semantic(options), "semantic search")
}

fn next_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, ExitCode> {
	match args.get(index + 1).map(String::as_str) {
		Some(value) if !value.starts_with('-') => Ok(value),
		_ => {
			eprintln!("{option} takes a value");
			Err(ExitCode::FAILURE)
		}
	}
}

fn run_x<F, T>(work: F, what: &str) -> ExitCode
where
	F: std::future::Future<Output = Result<T, x::Error>>,
	T: serde::Serialize,
{
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	match runtime.block_on(work) {
		Ok(value) => match serde_json::to_string_pretty(&value) {
			Ok(json) => {
				println!("{json}");
				ExitCode::SUCCESS
			}
			Err(error) => {
				eprintln!("could not encode {what}: {error}");
				ExitCode::FAILURE
			}
		},
		Err(error) => {
			eprintln!("{error}");
			ExitCode::FAILURE
		}
	}
}

fn usage() {
	eprintln!("usage: cms <command>");
	eprintln!();
	eprintln!("commands:");
	eprintln!("  overview                    print the workspace overview as JSON");
	eprintln!("  articles                    print the article listing and translation coverage");
	eprintln!("  derived                     print what each derived record class still owes");
	eprintln!("  tasks                       print the catalogue of long-running operations");
	eprintln!("  runs                        print what is running right now, machine-wide");
	eprintln!("  port                        print the port the web UI will bind");
	eprintln!("  image [--force] [--original] [file...]");
	eprintln!("                              derive what the articles reference, then rewrite them");
	eprintln!("  favicon [--force] [domain...]");
	eprintln!("                              collect the icons the linkcards need");
	eprintln!("  alt [--model M] [--force] [--limit N]");
	eprintln!("                              describe assets that have no description yet");
	eprintln!("  og [--force]                render an OpenGraph card per page per language");
	eprintln!("  segments                    write article segment ids and source ranges");
	eprintln!(
		"  i18n [--model M] [--model-id ID] [--effort E] [--parallel N] [--locale L] [--force] [--check] [--frontmatter] [--limit N] [article...]"
	);
	eprintln!("                              translate article segments into every locale");
	eprintln!("  tn [--model M] [--model-id ID] [--effort E] [--force] [article...]");
	eprintln!(
		"  embed [--force]              fetch the crate and repository data the articles embed"
	);
	eprintln!("                              suggest passages a translation would have to gloss");
	eprintln!("  locale [--model M] [--model-id ID] [--effort E] [--force] [--limit N]");
	eprintln!("                              translate tag labels and image descriptions");
	eprintln!("  summary [--model M] [--model-id ID] [--effort E] [--force] [--limit N]");
	eprintln!("                              write a reader-facing summary for each article");
	eprintln!("  tag [--model M] [--force] [--limit N]");
	eprintln!("                              give each asset a category and tags");
	eprintln!("  licenses                    record the licence of every dependency the apps ship");
	eprintln!("  check                       list referenced assets that are not present");
	eprintln!("  gc [--live]                 drop published assets no article asks for");
	eprintln!("  x user <query> [--count N]  search X users");
	eprintln!("  x keyword <query> [--limit N] [--mode Top|Latest]");
	eprintln!("                             search X posts by keyword");
	eprintln!("  x thread <post-id>          fetch an X post and its replies");
	eprintln!(
		"  x semantic <query> [--limit N] [--from DATE] [--to DATE] [--user NAME] [--exclude-user NAME] [--min-score N]"
	);
	eprintln!("                             search X posts by meaning");
}
