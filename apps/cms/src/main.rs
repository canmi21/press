//! Local content management: prepares the assets a build needs and writes them into
//! `data/public`, which is then mirrored to R2. See spec/architecture.md.
//!
//! The command line comes first and the web UI later, because the thing that actually needs
//! this today is the build, and a build calls a command rather than clicking a button. Both
//! shells will call the same modules.

use std::process::ExitCode;

mod alt;
mod check;
mod favicon;
mod gc;
mod i18n;
mod image;
mod media;
mod opengraph;
mod paths;
mod port;
mod refs;

fn main() -> ExitCode {
	let args: Vec<String> = std::env::args().skip(1).collect();
	match args.first().map(String::as_str) {
		Some("port") => print_port(),
		Some("favicon") => fetch_favicons(&args[1..]),
		Some("image") => process_images(&args[1..]),
		Some("check") => check_assets(),
		Some("og") => render_cards(&args[1..]),
		Some("i18n") => translate_articles(&args[1..]),
		Some("alt") => describe_images(&args[1..]),
		Some("gc") => collect_garbage(&args[1..]),
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
	let public = root.join("data").join("public");

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

	// One unreachable site must not abandon the rest: a run over an article's links should
	// collect what it can and report the gaps, not stop at the first dead domain.
	let mut failed = 0;
	for icon in &wanted {
		let host = &icon.domain;
		let result = match &icon.source {
			Some(url) => favicon::store_named(&public, host, url, icon.tone.as_deref(), force),
			None => favicon::store(&public, host, force),
		};
		match result {
			Ok(stored) if stored.skipped => println!("skip  {host}"),
			Ok(stored) => {
				let names: Vec<&str> = stored
					.written
					.iter()
					.filter_map(|path| path.file_name()?.to_str())
					.collect();
				println!("saved {host}/{{{}}}", names.join(", "));
			}
			Err(error) => {
				eprintln!("fail  {host}: {error}");
				failed += 1;
			}
		}
	}

	println!("{} of {} resolved", wanted.len() - failed, wanted.len());
	ExitCode::SUCCESS
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

	let root = match paths::repo_root() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};
	let originals = root.join("data").join("image");
	let public = root.join("data").join("public");
	let merged = image::run::load(&root.join(image::run::MERGED));
	let described_path = media::path_for(&root);
	let mut described = media::load(&described_path);

	// A runtime only for this command. Everything else here is a local file walk that gains
	// nothing from one; this is the single place where the work is waiting on somebody else.
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = runtime.block_on(alt::run(&merged, &mut described, &originals, force, limit));

	for (cid, error) in &outcome.failed {
		eprintln!("fail  {cid}: {error}");
	}
	// Reported rather than fatal: an asset whose original is gone can still be served, it just
	// cannot be looked at again.
	for cid in &outcome.unreadable {
		eprintln!("warn  no original on hand for {cid}");
	}

	if outcome.described > 0
		&& let Err(error) = media::save(&described_path, &described)
	{
		eprintln!("could not write media.yaml: {error}");
		return ExitCode::FAILURE;
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
/// One request covers one segment in all eight locales, so an edited paragraph costs one call
/// and updates every language together.
fn translate_articles(args: &[String]) -> ExitCode {
	let force = args.iter().any(|arg| arg == "--force");
	let limit = args
		.iter()
		.position(|arg| arg == "--limit")
		.and_then(|at| args.get(at + 1))
		.and_then(|value| value.parse::<usize>().ok());

	let mut only: Vec<std::path::PathBuf> = Vec::new();
	let mut runner = i18n::runner::Runner::Claude;
	let mut skip = false;
	for (at, arg) in args.iter().enumerate() {
		if skip {
			skip = false;
			continue;
		}
		match arg.as_str() {
			"--force" => {}
			"--limit" => skip = true,
			"--model" => {
				skip = true;
				match args
					.get(at + 1)
					.and_then(|name| i18n::runner::Runner::parse(name))
				{
					Some(chosen) => runner = chosen,
					None => {
						eprintln!("--model takes claude or gemini");
						return ExitCode::FAILURE;
					}
				}
			}
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

	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = match runtime.block_on(i18n::run(
		runner,
		&root.join("contents"),
		&only,
		limit,
		force,
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
		"{} translations across {} segments, {} failed",
		outcome.translated,
		outcome.segments,
		outcome.failed.len()
	);
	if outcome.translated > 0 {
		println!("{} tokens, ${:.2}", outcome.tokens, outcome.usd);
	}
	// A spent allowance is a normal state to stop in, not an error to report as one.
	if outcome.failed.is_empty() {
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

	// The site name on the card comes from the same file the pages read it from, under the
	// key those pages use: `name`, not `title`. An article has a title; the site has a name.
	let config = root.join("apps").join("site").join("site.config.yaml");
	let site = std::fs::read_to_string(&config)
		.ok()
		.and_then(|text| {
			text
				.lines()
				.find_map(|line| line.strip_prefix("name:"))
				.map(|value| value.trim().trim_matches('"').trim_matches('\'').to_owned())
		})
		.unwrap_or_default();

	let outcome = match opengraph::run(
		&root,
		&root.join("data").join("public"),
		&root.join("contents"),
		&site,
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
		println!("{}  {}: {}", gap.level.label(), gap.what, gap.detail);
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
		"{} derived, {} unchanged, {} failed, {} references rewritten",
		outcome.processed,
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

fn usage() {
	eprintln!("usage: cms <command>");
	eprintln!();
	eprintln!("commands:");
	eprintln!("  port                        print the port the web UI will bind");
	eprintln!("  image [--force] [--original] [file...]");
	eprintln!("                              derive what the articles reference, then rewrite them");
	eprintln!("  favicon [--force] [domain...]");
	eprintln!("                              collect the icons the linkcards need");
	eprintln!("  alt [--force] [--limit N]   describe assets that have no description yet");
	eprintln!("  og [--force]                render an OpenGraph card per article");
	eprintln!("  i18n [--model claude|gemini|gpt-oss] [--force] [--limit N] [article...]");
	eprintln!("                              translate article segments into every locale");
	eprintln!("  check                       list referenced assets that are not present");
	eprintln!("  gc [--live]                 drop published assets no article asks for");
}
