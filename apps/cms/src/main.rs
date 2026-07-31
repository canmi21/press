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
mod image;
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
	let merged_path = root.join(image::run::MERGED);
	let mut merged = image::run::load(&merged_path);

	// A runtime only for this command. Everything else here is a local file walk that gains
	// nothing from one; this is the single place where the work is waiting on somebody else.
	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(error) => {
			eprintln!("could not start a runtime: {error}");
			return ExitCode::FAILURE;
		}
	};
	let outcome = runtime.block_on(alt::run(&mut merged, &originals, force, limit));

	for (cid, error) in &outcome.failed {
		eprintln!("fail  {cid}: {error}");
	}
	// Reported rather than fatal: an asset whose original is gone can still be served, it just
	// cannot be looked at again.
	for cid in &outcome.unreadable {
		eprintln!("warn  no original on hand for {cid}");
	}

	if outcome.described > 0 {
		merged.generated = image::manifest::now();
		let json = serde_json::to_string_pretty(&merged).unwrap_or_default();
		if let Err(error) = image::store::write(&merged_path, format!("{json}\n").as_bytes()) {
			eprintln!("could not write the manifest: {error}");
			return ExitCode::FAILURE;
		}
		// The published record carries the description too, so the metadata API answers with it
		// without the site having to be rebuilt first.
		for (cid, media) in &merged.assets {
			if let Err(error) = image::run::republish(&public, cid, media) {
				eprintln!("could not publish {cid}: {error}");
				return ExitCode::FAILURE;
			}
		}
	}

	println!(
		"{} described, {} already had one, {} left by --limit, {} failed",
		outcome.described,
		outcome.skipped,
		outcome.deferred,
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
		println!("drop  {} from assets.json", cid);
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
	eprintln!("  check                       list referenced assets that are not present");
	eprintln!("  gc [--live]                 drop published assets no article asks for");
}
