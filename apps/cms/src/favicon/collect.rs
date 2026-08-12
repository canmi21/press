//! Collecting favicons as a task, below both shells.
//!
//! The first operation to run on the task substrate, and it was chosen for what it costs to get
//! wrong: no model is asked, nothing is overwritten that cannot be fetched again, and a domain
//! that fails leaves the others alone. See spec/tasks.md.
//!
//! The shape every later migration copies:
//!
//! - publish a registry entry, so a second CMS can see this run before starting its own
//! - claim each item, and **skip** what somebody else already holds rather than waiting
//! - fetch outside any lock, because that is the slow part
//! - hand the write to the record's writer, which is the only thing that touches the record

use std::path::Path;

use crate::refs::Wanted;
use crate::task::{Record, claim, progress, registry, writer};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
	pub collected: usize,
	/// Already present, and this was not a forced run.
	pub skipped: usize,
	/// Domain and why, one line each. A dead site must not end the run.
	pub failed: Vec<(String, String)>,
	/// Items another live process was already collecting.
	pub claimed_elsewhere: Vec<String>,
}

pub struct Options<'a> {
	pub repository: &'a Path,
	pub wanted: &'a [Wanted],
	pub force: bool,
	pub shell: registry::Shell,
	/// Where to report progress. The CLI passes a terminal bar; the desktop passes its own.
	pub sink: Box<dyn progress::Sink>,
}

/// Collect every icon in `wanted` that this process can claim.
pub fn run(options: Options<'_>) -> std::io::Result<Outcome> {
	let Options {
		repository,
		wanted,
		force,
		shell,
		sink,
	} = options;
	let public = repository.join("data").join("public");

	// Published before any work, so a second process asking "is favicon running" during the first
	// fetch gets yes rather than a gap.
	let entry = registry::publish(repository, "favicon", shell, wanted.len() as u64)?;
	let progress = progress::Progress::new(
		wanted.len() as u64,
		Box::new(Both {
			first: sink,
			second: Box::new(registry::Published::new(entry)),
		}),
	);
	let writer = writer::Writer::start(repository, Record::PublicFavicon)?;

	let mut outcome = Outcome::default();
	for icon in wanted {
		let domain = icon.domain.clone();
		progress.set_message(domain.clone());

		// Taken for the length of this domain only. Holding one claim for the whole run would
		// make two processes with overlapping lists do nothing in parallel.
		let claim = match claim::take(repository, "favicon", &domain) {
			Ok(claim) => claim,
			Err(claim::Denied::Taken(_)) => {
				outcome.claimed_elsewhere.push(domain);
				progress.inc(1);
				continue;
			}
			Err(claim::Denied::Io(error)) => return Err(error),
		};

		// Re-read now that the claim is held. Another run may have collected this domain between
		// the list being built and this item being reached -- claims stop two runs doing an item
		// at the same time, not one run doing what another already finished. Measured with two
		// concurrent processes over five domains: one was fetched twice without this. A forced
		// run has nothing to re-read, since `--force` means redo it. See spec/tasks.md.
		if !force && public.join("favicon").join(&domain).is_dir() {
			outcome.skipped += 1;
			progress.inc(1);
			continue;
		}

		// Outside the writer, deliberately: this is seconds of somebody else's server, and the
		// record must not be held across it.
		let fetched = match &icon.source {
			Some(url) => {
				match crate::favicon::fetch_named(&public, &domain, url, icon.tone.as_deref(), force) {
					Ok(fetched) => fetched,
					Err(error) => {
						outcome.failed.push((domain, error.to_string()));
						progress.inc(1);
						continue;
					}
				}
			}
			None => match crate::favicon::fetch_for(&public, &domain, force) {
				Ok(fetched) => fetched,
				Err(error) => {
					outcome.failed.push((domain, error.to_string()));
					progress.inc(1);
					continue;
				}
			},
		};

		let Some(icons) = fetched else {
			outcome.skipped += 1;
			progress.inc(1);
			continue;
		};

		let public_for_write = public.clone();
		let domain_for_write = domain.clone();
		let applied = writer.apply(move || {
			crate::favicon::write_fetched(&public_for_write, &domain_for_write, &icons)
				.map(|_| ())
				.map_err(std::io::Error::other)
		});
		match applied {
			Ok(()) => outcome.collected += 1,
			Err(error) => outcome.failed.push((domain, error.to_string())),
		}

		// Released here rather than at the end of the loop body's scope, to say that the claim
		// covers fetching and writing this domain and nothing after.
		drop(claim);
		progress.inc(1);
	}

	progress.finish_and_clear();
	Ok(outcome)
}

/// Reports to two sinks. A run is watched by whoever started it and by the registry at once.
struct Both {
	first: Box<dyn progress::Sink>,
	second: Box<dyn progress::Sink>,
}

impl progress::Sink for Both {
	fn started(&self, total: u64) {
		self.first.started(total);
		self.second.started(total);
	}

	fn advanced(&self, done: u64, total: u64, message: &str) {
		self.first.advanced(done, total, message);
		self.second.advanced(done, total, message);
	}

	fn finished(&self) {
		self.first.finished();
		self.second.finished();
	}

	/// Only the first sink can be drawing on a terminal; the registry writes to a file and has
	/// nothing to move out of the way.
	fn suspend(&self, body: &mut dyn FnMut()) {
		self.first.suspend(body);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn temp(name: &str) -> std::path::PathBuf {
		let path = std::env::temp_dir().join(format!("cms-collect-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	fn wanted(domain: &str) -> Wanted {
		Wanted {
			domain: domain.to_owned(),
			source: None,
			tone: None,
		}
	}

	/// A domain already collected is skipped without reaching the network, which is what makes a
	/// rerun cheap. Creating the directory is how this repository records "asked already".
	#[test]
	fn an_already_collected_domain_is_skipped() {
		let root = temp("skip");
		std::fs::create_dir_all(root.join("data/public/favicon/example.com")).expect("dir");
		let outcome = run(Options {
			repository: &root,
			wanted: &[wanted("example.com")],
			force: false,
			shell: registry::Shell::Cli,
			sink: Box::new(progress::Silent),
		})
		.expect("run");
		assert_eq!(outcome.skipped, 1);
		assert_eq!(outcome.collected, 0);
		assert!(outcome.failed.is_empty());
		std::fs::remove_dir_all(root).ok();
	}

	/// The contention case, without a second process: a claim held by this test stands in for one
	/// held by another CMS. The item is reported as somebody else's and the run continues.
	#[test]
	fn an_item_claimed_elsewhere_is_left_alone() {
		let root = temp("claimed");
		std::fs::create_dir_all(root.join("data/public/favicon/free.example")).expect("dir");
		let held = claim::take(&root, "favicon", "taken.example").expect("claim");

		let outcome = run(Options {
			repository: &root,
			wanted: &[wanted("taken.example"), wanted("free.example")],
			force: false,
			shell: registry::Shell::Cli,
			sink: Box::new(progress::Silent),
		})
		.expect("run");

		assert_eq!(outcome.claimed_elsewhere, vec!["taken.example".to_owned()]);
		// The rest of the list still ran.
		assert_eq!(outcome.skipped, 1);
		drop(held);
		std::fs::remove_dir_all(root).ok();
	}

	/// The run is visible to another reader while it happens, and gone afterwards.
	#[test]
	fn the_run_publishes_itself_and_cleans_up() {
		let root = temp("published");
		std::fs::create_dir_all(root.join("data/public/favicon/example.com")).expect("dir");
		assert!(
			registry::running(&root, "favicon")
				.expect("before")
				.is_none()
		);
		run(Options {
			repository: &root,
			wanted: &[wanted("example.com")],
			force: false,
			shell: registry::Shell::Desktop,
			sink: Box::new(progress::Silent),
		})
		.expect("run");
		assert!(
			registry::running(&root, "favicon")
				.expect("after")
				.is_none()
		);
		std::fs::remove_dir_all(root).ok();
	}

	/// The gap two concurrent processes exposed: an item finished by somebody else after the list
	/// was built must be dropped once the claim is held, or the work is simply done twice. The
	/// directory appearing mid-run stands in for the other process having collected it.
	#[test]
	fn an_item_finished_by_someone_else_is_dropped_after_claiming() {
		let root = temp("recheck");
		let collected = root.join("data/public/favicon/late.example");
		std::fs::create_dir_all(&collected).expect("dir");

		let outcome = run(Options {
			repository: &root,
			wanted: &[wanted("late.example")],
			force: false,
			shell: registry::Shell::Cli,
			sink: Box::new(progress::Silent),
		})
		.expect("run");

		// Never reached the network: no failure, and nothing collected.
		assert_eq!(outcome.skipped, 1);
		assert_eq!(outcome.collected, 0);
		assert!(outcome.failed.is_empty());
		std::fs::remove_dir_all(root).ok();
	}

	/// A claim taken for one domain is released before the run ends, so a second run can take it.
	#[test]
	fn claims_do_not_outlive_the_item() {
		let root = temp("release");
		std::fs::create_dir_all(root.join("data/public/favicon/example.com")).expect("dir");
		run(Options {
			repository: &root,
			wanted: &[wanted("example.com")],
			force: false,
			shell: registry::Shell::Cli,
			sink: Box::new(progress::Silent),
		})
		.expect("run");
		assert!(claim::live(&root).expect("live").is_empty());
		std::fs::remove_dir_all(root).ok();
	}
}
