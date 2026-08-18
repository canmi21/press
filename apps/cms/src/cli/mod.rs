//! The command-line adapter for shared CMS operations. See spec/architecture/cms.md.
//!
//! What it accepts is declared in [args], so an argument nothing names is refused rather than
//! ignored. The functions here take the parsed values and do no parsing of their own.

mod args;

use crate::{
	alt, articles, check, classify, derived, embed, favicon, gc, i18n, image, licenses, locale,
	opengraph, overview, paths, port, refs, summary, task, twitter,
};
use anyhow::Context as _;
use args::{Cli, Command, ModelArgs, TwitterCommand};
use clap::Parser;
use std::process::ExitCode;

pub fn run() -> ExitCode {
	let cli = match Cli::try_parse() {
		Ok(cli) => cli,
		// clap has already written the message -- a help request, a version, or a rejection that
		// names what was wrong. Its own exit code carries which of those it was.
		Err(error) => {
			let _ = error.print();
			return if error.use_stderr() {
				ExitCode::FAILURE
			} else {
				ExitCode::SUCCESS
			};
		}
	};

	match dispatch(cli.command) {
		Ok(code) => code,
		// `{:#}` is anyhow's flattened form: the failure and every context it collected on the way
		// up, on one line. Nothing above this point printed anything, so this is the only place a
		// person is told what went wrong -- and the only place the chain stops being a chain. See
		// spec/code.md.
		Err(error) => {
			eprintln!("{error:#}");
			ExitCode::FAILURE
		}
	}
}

/// Run one command.
///
/// `Err` means the command could not run. `Ok(ExitCode::FAILURE)` means it ran and has something
/// to report -- items that failed inside a batch that otherwise finished. Collapsing the two would
/// make `cms alt` on a library where one description failed indistinguishable from `cms alt` in a
/// directory that is not a repository.
fn dispatch(command: Command) -> anyhow::Result<ExitCode> {
	match command {
		Command::Overview => print_overview(),
		Command::Articles => print_articles(),
		Command::Derived => print_derived(),
		Command::Tasks => print_tasks(),
		Command::Runs => print_runs(),
		Command::Port => print_port(),
		Command::Segments => write_segment_layout(),
		Command::Check => check_assets(),
		Command::Licenses => collect_licenses(),
		Command::Favicon { force, domains } => fetch_favicons(force, &domains),
		Command::Image {
			force,
			original,
			files,
		} => process_images(force, original, &files),
		Command::Og { force } => render_cards(force),
		Command::Alt {
			model,
			force,
			limit,
		} => describe_images(&model, force, limit),
		Command::Tag {
			model,
			force,
			limit,
		} => classify_images(&model, force, limit),
		Command::Summary {
			model,
			force,
			limit,
		} => summarise_articles(&model, force, limit),
		Command::I18n {
			model,
			force,
			check,
			frontmatter,
			limit,
			parallel,
			locale,
			articles,
		} => translate_articles(I18nArgs {
			model: &model,
			force,
			check,
			frontmatter,
			limit,
			parallel,
			locale: &locale,
			articles: &articles,
		}),
		Command::Tn {
			model,
			force,
			articles,
		} => scan_notes(&model, force, &articles),
		Command::Embed { force } => fetch_embeds(force),
		Command::Locale {
			model,
			force,
			limit,
		} => translate_locales(&model, force, limit),
		Command::Gc { live } => collect_garbage(live),
		Command::Twitter { command } => twitter_command(command),
	}
}

/// `cms i18n` takes eight of them, which is past the point where positional parameters read.
struct I18nArgs<'a> {
	model: &'a ModelArgs,
	force: bool,
	check: bool,
	frontmatter: bool,
	limit: Option<usize>,
	parallel: Option<usize>,
	locale: &'a [String],
	articles: &'a [std::path::PathBuf],
}

fn print_overview() -> anyhow::Result<ExitCode> {
	match overview::snapshot() {
		Ok(snapshot) => match serde_json::to_string_pretty(&snapshot) {
			Ok(json) => {
				println!("{json}");
				Ok(ExitCode::SUCCESS)
			}
			Err(error) => {
				eprintln!("could not encode overview: {error}");
				Ok(ExitCode::FAILURE)
			}
		},
		Err(error) => {
			eprintln!("could not read overview: {error}");
			Ok(ExitCode::FAILURE)
		}
	}
}

fn print_articles() -> anyhow::Result<ExitCode> {
	match articles::listing() {
		Ok(listing) => match serde_json::to_string_pretty(&listing) {
			Ok(json) => {
				println!("{json}");
				Ok(ExitCode::SUCCESS)
			}
			Err(error) => {
				eprintln!("could not encode the article listing: {error}");
				Ok(ExitCode::FAILURE)
			}
		},
		Err(error) => {
			eprintln!("could not read the article listing: {error}");
			Ok(ExitCode::FAILURE)
		}
	}
}

fn print_derived() -> anyhow::Result<ExitCode> {
	match derived::report() {
		Ok(report) => match serde_json::to_string_pretty(&report) {
			Ok(json) => {
				println!("{json}");
				Ok(ExitCode::SUCCESS)
			}
			Err(error) => {
				eprintln!("could not encode the derived report: {error}");
				Ok(ExitCode::FAILURE)
			}
		},
		Err(error) => {
			eprintln!("could not read the derived report: {error}");
			Ok(ExitCode::FAILURE)
		}
	}
}

fn print_tasks() -> anyhow::Result<ExitCode> {
	match serde_json::to_string_pretty(task::CATALOG) {
		Ok(json) => {
			println!("{json}");
			Ok(ExitCode::SUCCESS)
		}
		Err(error) => {
			eprintln!("could not encode the task catalogue: {error}");
			Ok(ExitCode::FAILURE)
		}
	}
}

fn print_port() -> anyhow::Result<ExitCode> {
	match port::from_env() {
		Ok(port) => {
			println!("{port}");
			Ok(ExitCode::SUCCESS)
		}
		Err(error) => {
			eprintln!("{error}");
			Ok(ExitCode::FAILURE)
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
fn fetch_favicons(force: bool, domains: &[String]) -> anyhow::Result<ExitCode> {
	let inputs: Vec<&str> = domains.iter().map(String::as_str).collect();

	let root = paths::repo_root()?;

	let wanted: Vec<refs::Wanted> = if inputs.is_empty() {
		match refs::scan(&root.join("contents")) {
			Ok(scan) => scan.wanted(),
			Err(error) => {
				eprintln!("could not read articles: {error}");
				return Ok(ExitCode::FAILURE);
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
		return Ok(ExitCode::SUCCESS);
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
			return Ok(ExitCode::FAILURE);
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
	Ok(ExitCode::SUCCESS)
}

/// What is running anywhere on this machine for this repository.
fn print_runs() -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;
	let runs = task::registry::live(&root).context("could not read the run registry")?;
	match serde_json::to_string_pretty(&runs) {
		Ok(json) => {
			println!("{json}");
			Ok(ExitCode::SUCCESS)
		}
		Err(error) => {
			eprintln!("could not encode the run registry: {error}");
			Ok(ExitCode::FAILURE)
		}
	}
}

/// Write a reader-facing summary into every article that has none.
///
/// The value lands in a sidecar beside the article, in the article's own language. `cms locale`
/// translates it into the other locales afterwards.
fn summarise_articles(
	model: &ModelArgs,
	force: bool,
	limit: Option<usize>,
) -> anyhow::Result<ExitCode> {
	// Not `DEFAULT_TEXT`. This is the one text task carrying a constraint the model has to hold
	// against its own training -- summarise, but withhold the conclusion -- and the open-weight
	// default measurably does not: it handed over the whole design and then appended "reaches a
	// surprising conclusion", and gave a first-person essay's author a pronoun the article never
	// uses. Translation has no comparable trap, which is why that one stays on the cheap model.
	let runner = model.runner(i18n::runner::Runner::Codex);
	let model_override = model.overrides(runner).map_err(anyhow::Error::msg)?;

	let root = paths::repo_root()?;

	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
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
			return Ok(ExitCode::FAILURE);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Describe every asset that has no description yet.
///
/// The description is written into the manifest, so it belongs to the picture rather than to
/// whichever article happened to be open when it was generated. Every reference inherits it,
/// including ones written later.
fn describe_images(
	model: &ModelArgs,
	force: bool,
	limit: Option<usize>,
) -> anyhow::Result<ExitCode> {
	let runner = model.runner(i18n::runner::DEFAULT_VISION);

	let root = paths::repo_root()?;
	let originals = root.join("data").join("image");
	let public = root.join("data").join("public");
	let merged = match image::run::load(&root.join(image::run::MERGED)) {
		Ok(merged) => merged,
		Err(error) => {
			eprintln!("could not read {}: {error}", image::run::MERGED);
			return Ok(ExitCode::FAILURE);
		}
	};
	// A runtime only for this command. Everything else here is a local file walk that gains
	// nothing from one; this is the single place where the work is waiting on somebody else.
	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
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
			return Ok(ExitCode::FAILURE);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Translate every article segment that has no translation yet.
///
/// One request covers one segment's missing locales, so an edited paragraph costs one call while
/// a partial repair does not repay for completed languages.
fn translate_articles(args: I18nArgs<'_>) -> anyhow::Result<ExitCode> {
	let I18nArgs {
		model,
		force,
		check,
		frontmatter,
		limit,
		parallel,
		locale,
		articles,
	} = args;
	let scope = if frontmatter {
		i18n::Scope::Frontmatter
	} else {
		i18n::Scope::All
	};
	let parallel =
		i18n::parallelism(parallel.map(|n| n.to_string()).as_deref()).map_err(anyhow::Error::msg)?;
	let locales = i18n::selected_locales(locale).map_err(anyhow::Error::msg)?;
	let only = articles.to_vec();
	let runner = model.runner(i18n::runner::DEFAULT_TEXT);
	let model_override = model.overrides(runner).map_err(anyhow::Error::msg)?;

	let root = paths::repo_root()?;
	if let Err(error) = i18n::layout::sync(&root) {
		eprintln!("could not write {}: {error}", i18n::layout::FILE);
		return Ok(ExitCode::FAILURE);
	}

	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
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
			return Ok(ExitCode::FAILURE);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Materialise the Rust segment ids and source ranges for builds that do not have Rust.
fn write_segment_layout() -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;
	match i18n::layout::sync(&root) {
		Ok(true) => println!("wrote {}", i18n::layout::FILE),
		Ok(false) => println!("{} unchanged", i18n::layout::FILE),
		Err(error) => {
			eprintln!("could not write {}: {error}", i18n::layout::FILE);
			return Ok(ExitCode::FAILURE);
		}
	}
	Ok(ExitCode::SUCCESS)
}

/// Translate tag labels and image descriptions from their English source text.
fn translate_locales(
	model: &ModelArgs,
	force: bool,
	limit: Option<usize>,
) -> anyhow::Result<ExitCode> {
	let runner = model.runner(i18n::runner::DEFAULT_TEXT);
	let model_override = model.overrides(runner).map_err(anyhow::Error::msg)?;

	let root = paths::repo_root()?;
	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
	let outcome = match runtime.block_on(locale::run(locale::Options {
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
			eprintln!("could not write: {error}");
			return Ok(ExitCode::FAILURE);
		}
	};

	for (id, error) in &outcome.failed {
		eprintln!("fail  {id}: {error}");
	}
	if outcome.claimed_elsewhere > 0 {
		eprintln!(
			"note  {} left to a run already translating them",
			outcome.claimed_elsewhere
		);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Render one OpenGraph card per article.
///
/// Nothing references these: the page emits `/opengraph/{slug}.png` and no article writes the
/// URL down, so there is no reference to rewrite and the slug is the name.
fn render_cards(force: bool) -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;

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
			return Ok(ExitCode::FAILURE);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Give every asset a category and a handful of tags.
fn classify_images(
	model: &ModelArgs,
	force: bool,
	limit: Option<usize>,
) -> anyhow::Result<ExitCode> {
	let runner = model.runner(i18n::runner::DEFAULT_VISION);

	let root = paths::repo_root()?;
	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
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
			return Ok(ExitCode::FAILURE);
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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
	}
}

/// Report what the articles reference and `data/public` cannot answer for.
///
/// Always succeeds. This is a report, and a report that can fail a build is a gate wearing a
/// report's name.
fn check_assets() -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;

	let gaps = match check::report(
		&root,
		&root.join("data").join("public"),
		&root.join("contents"),
	) {
		Ok(gaps) => gaps,
		Err(error) => {
			eprintln!("could not read articles: {error}");
			return Ok(ExitCode::FAILURE);
		}
	};

	if gaps.is_empty() {
		println!("every referenced asset is present");
		return Ok(ExitCode::SUCCESS);
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
	Ok(ExitCode::SUCCESS)
}

/// Drop everything in `data/public` that no article asks for.
///
/// Dry by default. The listing is the review, and `--live` is the answer to it.
fn collect_garbage(live: bool) -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;
	let public = root.join("data").join("public");

	let sweep = gc::plan(&root, &public, &root.join("contents")).context("could not plan")?;

	if sweep.orphans.is_empty() && sweep.entries.is_empty() {
		println!("nothing to collect");
		return Ok(ExitCode::SUCCESS);
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
		return Ok(ExitCode::SUCCESS);
	}
	if let Err(error) = gc::apply(&root, &sweep) {
		eprintln!("could not delete: {error}");
		return Ok(ExitCode::FAILURE);
	}
	println!("collected");
	Ok(ExitCode::SUCCESS)
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
fn collect_licenses() -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;
	let public = root.join("data").join("public");

	let mut found = licenses::npm::collect(&root).map_err(anyhow::Error::msg)?;
	let npm = found.len();
	found.extend(licenses::cargo::collect(&root).map_err(anyhow::Error::msg)?);
	println!("{npm} npm packages, {} crates", found.len() - npm);

	let assertions = licenses::read_assertions(&root).map_err(anyhow::Error::msg)?;

	let written =
		licenses::write(&public, found, &assertions).context("could not publish licence texts")?;

	let document = licenses::full_document(&public, &written.record)
		.context("could not assemble the full notice")?;
	if let Err(error) = image::store::write(&licenses::full_path(&public), document.as_bytes()) {
		eprintln!("could not write the full notice: {error}");
		return Ok(ExitCode::FAILURE);
	}

	let record_path = licenses::record_path(&root);
	let json =
		serde_json::to_string_pretty(&written.record).context("could not serialise the record")?;
	if let Err(error) = image::store::write(&record_path, format!("{json}\n").as_bytes()) {
		eprintln!("could not write {}: {error}", record_path.display());
		return Ok(ExitCode::FAILURE);
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
		return Ok(ExitCode::FAILURE);
	}
	Ok(ExitCode::SUCCESS)
}

/// Derive and publish every image the articles ask for, then rewrite what they say.
///
/// With no file arguments this follows the articles. Named files are imported ahead of the
/// article that will use them, which is how `--original` gets attached to a photograph before
/// anything references it.
fn process_images(
	force: bool,
	keep_original: bool,
	files: &[std::path::PathBuf],
) -> anyhow::Result<ExitCode> {
	let only = files.to_vec();

	let root = paths::repo_root()?;
	let originals = root.join("data").join("image");
	let public = root.join("data").join("public");
	let articles = root.join("contents");

	let options = image::run::Options {
		force,
		keep_original,
		only: &only,
	};
	let outcome =
		image::run::run(&root, &originals, &public, &articles, &options).context("could not write")?;

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
		Ok(ExitCode::SUCCESS)
	} else {
		Ok(ExitCode::FAILURE)
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
/// FIXME: this is the operation, not an adapter for one. spec/architecture/cms.md gives every CMS
/// capability one in-process application operation with a CLI and a GUI adapter over it, and the
/// work below -- argument handling aside -- belongs beside the module it drives rather than in
/// this file. It is left here deliberately rather than exempted in the spec: moving it is the
/// same edit as putting it under the task substrate, and both wait on the desktop shell reaching
/// the point where it offers this command. Whoever gets there first should do the two together.
fn scan_notes(
	model: &ModelArgs,
	force: bool,
	articles: &[std::path::PathBuf],
) -> anyhow::Result<ExitCode> {
	let only = articles.to_vec();
	let runner = model.runner(i18n::runner::DEFAULT_VISION);
	let model_override = model.overrides(runner).map_err(anyhow::Error::msg)?;

	let root = paths::repo_root()?;
	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;

	let contents = root.join("contents");
	let path = i18n::tn::path_for(&root);
	let mut table = match i18n::tn::load(&path) {
		Ok(table) => table,
		Err(error) => {
			eprintln!("could not read {}: {error}", path.display());
			return Ok(ExitCode::FAILURE);
		}
	};
	// Pages without `lang` are not articles. Keep them out of both new scans and the durable
	// registry, including records written by older versions of this command.
	let recorded = table.articles.len();
	table.articles.retain(|key, _| {
		std::fs::read_to_string(contents.join(key))
			.map(|source| {
				crate::document::fields(&source).is_ok_and(|fields| summary::lang_of(&fields).is_some())
			})
			// A missing source cannot prove the record belongs to a page. Preserve paid history
			// until a source file exists that positively identifies itself as one.
			.unwrap_or(true)
	});
	if table.articles.len() != recorded
		&& let Err(error) = i18n::tn::save(&path, &table)
	{
		eprintln!("could not write {}: {error}", path.display());
		return Ok(ExitCode::FAILURE);
	}

	// Named articles, or every one not yet read. An article scanned and found to need nothing
	// still counts as read, which is the distinction the table records so that a rerun does not
	// pay to learn the same nothing twice.
	let wanted: Vec<std::path::PathBuf> = if only.is_empty() {
		match refs::markdown_under(&contents) {
			Ok(all) => all,
			Err(error) => {
				eprintln!("could not read {}: {error}", contents.display());
				return Ok(ExitCode::FAILURE);
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
			.and_then(|source| crate::document::fields(&source).ok())
			.is_some_and(|fields| summary::lang_of(&fields).is_some())
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

		let segments = match i18n::segment::split(&text) {
			Ok(segments) => segments,
			Err(error) => {
				eprintln!("{}: {error}", path.display());
				return Ok(ExitCode::FAILURE);
			}
		};
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
			return Ok(ExitCode::FAILURE);
		}
		progress.inc(1);
	}
	progress.finish_and_clear();

	if read == 0 {
		println!("every article already read; pass --force to read one again");
		return Ok(ExitCode::SUCCESS);
	}
	println!("{read} read, {suggested} suggestions in data/tn.yaml; delete any you disagree with");
	println!("{spent} tokens");
	Ok(ExitCode::SUCCESS)
}

/// The `cms embed` command: the crate trees and repository facts the articles show.
///
/// Fetched here rather than in the browser, so a page renders from a checkout with no proxy
/// route, no request per reader and no key. Both records rebuild from what git already holds,
/// which is what puts them under `data/build/`. See spec/architecture/cms.md.
///
/// FIXME: this is the operation, not an adapter for one. spec/architecture/cms.md gives every CMS
/// capability one in-process application operation with a CLI and a GUI adapter over it, and the
/// work below -- argument handling aside -- belongs beside the module it drives rather than in
/// this file. It is left here deliberately rather than exempted in the spec: moving it is the
/// same edit as putting it under the task substrate, and both wait on the desktop shell reaching
/// the point where it offers this command. Whoever gets there first should do the two together.
fn fetch_embeds(force: bool) -> anyhow::Result<ExitCode> {
	let root = paths::repo_root()?;

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

	let articles = refs::markdown_under(&root.join("contents")).context("could not read contents")?;
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
		return Ok(ExitCode::FAILURE);
	}
	if let Err(error) = image::store::write(
		&embed::repos_path(&root),
		serde_json::to_string_pretty(&repos)
			.unwrap_or_default()
			.as_bytes(),
	) {
		eprintln!("could not write repos.json: {error}");
		return Ok(ExitCode::FAILURE);
	}

	println!(
		"{} crates, {} repositories, {failed} failed",
		crates.crates.len(),
		repos.repos.len()
	);
	Ok(ExitCode::SUCCESS)
}

fn twitter_command(command: TwitterCommand) -> anyhow::Result<ExitCode> {
	match command {
		TwitterCommand::User { query, count } => {
			let query = query.join(" ");
			run_lookup(
				twitter::users(&query, count.unwrap_or(twitter::DEFAULT_COUNT)),
				"user search",
			)
		}
		TwitterCommand::Keyword { query, limit, mode } => {
			let mode = match mode.as_deref().map(twitter::Mode::parse) {
				None => twitter::Mode::default(),
				Some(Some(mode)) => mode,
				Some(None) => {
					eprintln!("--mode takes Top or Latest");
					return Ok(ExitCode::FAILURE);
				}
			};
			let query = query.join(" ");
			run_lookup(
				twitter::keyword(&query, limit.unwrap_or(twitter::DEFAULT_LIMIT), mode),
				"keyword search",
			)
		}
		TwitterCommand::Thread { id } => run_lookup(twitter::thread(&id), "thread"),
		TwitterCommand::Semantic {
			query,
			limit,
			from,
			to,
			user,
			exclude_user,
			min_score,
		} => {
			let mut options = twitter::Semantic::new(query.join(" "));
			if let Some(limit) = limit {
				options.limit = limit;
			}
			options.from_date = from;
			options.to_date = to;
			options.usernames = user;
			options.exclude_usernames = exclude_user;
			if let Some(score) = min_score {
				options.min_score = score;
			}
			run_lookup(twitter::semantic(options), "semantic search")
		}
	}
}

fn run_lookup<F, T>(work: F, what: &str) -> anyhow::Result<ExitCode>
where
	F: std::future::Future<Output = Result<T, twitter::Error>>,
	T: serde::Serialize,
{
	let runtime = tokio::runtime::Runtime::new().context("could not start a runtime")?;
	match runtime.block_on(work) {
		Ok(value) => match serde_json::to_string_pretty(&value) {
			Ok(json) => {
				println!("{json}");
				Ok(ExitCode::SUCCESS)
			}
			Err(error) => {
				eprintln!("could not encode {what}: {error}");
				Ok(ExitCode::FAILURE)
			}
		},
		Err(error) => {
			eprintln!("{error}");
			Ok(ExitCode::FAILURE)
		}
	}
}
