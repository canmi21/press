//! Local content management: prepares the assets a build needs and writes them into
//! `data/public`, which is then mirrored to R2. See spec/architecture.md.
//!
//! The command line comes first and the web UI later, because the thing that actually needs
//! this today is the build, and a build calls a command rather than clicking a button. Both
//! shells will call the same modules.

use std::process::ExitCode;

mod favicon;
mod paths;
mod port;

use favicon::parse::Tone;

fn main() -> ExitCode {
	let args: Vec<String> = std::env::args().skip(1).collect();
	match args.first().map(String::as_str) {
		Some("port") => print_port(),
		Some("favicon") => fetch_favicons(&args[1..]),
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

fn fetch_favicons(args: &[String]) -> ExitCode {
	let mut force = false;
	let mut tone = None;
	let mut inputs: Vec<&str> = Vec::new();

	let mut rest = args.iter();
	while let Some(arg) = rest.next() {
		match arg.as_str() {
			"--force" => force = true,
			"--tone" => match rest.next().map(String::as_str).and_then(Tone::parse) {
				Some(parsed) => tone = Some(parsed),
				None => {
					eprintln!("--tone takes dark or light");
					return ExitCode::FAILURE;
				}
			},
			other => inputs.push(other),
		}
	}

	if inputs.is_empty() {
		eprintln!("usage: cms favicon [--tone dark|light] [--force] <domain-or-url>...");
		return ExitCode::FAILURE;
	}

	let root = match paths::data_public() {
		Ok(root) => root,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let hosts = favicon::host::normalise(inputs);
	if hosts.is_empty() {
		eprintln!("no fetchable hostnames in that list");
		return ExitCode::FAILURE;
	}

	// One unreachable site must not abandon the rest: a run over an article's links should
	// collect what it can and report the gaps, not stop at the first dead domain.
	let mut failed = 0;
	for host in &hosts {
		match favicon::store(&root, host, tone, force) {
			Ok(stored) if stored.skipped => println!("skip  {host}"),
			Ok(stored) => println!("saved {host} -> {}", stored.path.display()),
			Err(error) => {
				eprintln!("fail  {host}: {error}");
				failed += 1;
			}
		}
	}

	println!("{} of {} resolved", hosts.len() - failed, hosts.len());
	ExitCode::SUCCESS
}

fn usage() {
	eprintln!("usage: cms <command>");
	eprintln!();
	eprintln!("commands:");
	eprintln!("  port                       print the port the web UI will bind");
	eprintln!("  favicon <domain-or-url>... fetch icons into data/public/favicon");
}
