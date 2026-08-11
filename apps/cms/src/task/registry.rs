//! What is running right now, readable by anything on the machine.
//!
//! A run publishes an entry when it starts, updates its progress as it goes, and the entry ceases
//! to be live the moment the process holding it stops existing. Any other process -- a second
//! desktop window, a command typed in a terminal, the schedule -- reads the directory to find out
//! what is already happening before deciding whether to start anything. See spec/tasks.md.
//!
//! ## Reading never takes a lock it could keep
//!
//! Liveness is tested by trying the entry's lock and immediately releasing it. Succeeding means
//! the writer is gone. Reading the registry therefore cannot block on a live run, cannot make one
//! wait, and cannot leave anything behind if the reader dies mid-listing.
//!
//! The consequence worth stating: an entry's *contents* can be stale by the age of the last
//! update, so progress is a hint. Its *existence under a held lock* is not a hint -- that is the
//! only fact here, and it is the one callers are supposed to branch on.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

/// A run, as published for everyone else to read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Run {
	/// The task's catalogue id.
	pub task: String,
	pub pid: u32,
	/// Which shell started it, for a reader deciding whether it can be interrupted.
	pub shell: Shell,
	/// ISO 8601 UTC.
	pub started: String,
	/// Items finished and items expected. A hint; see the module note.
	pub done: u64,
	pub total: u64,
	/// What the run was working on when it last reported.
	pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
	/// A command somebody typed, or a script.
	Cli,
	/// The resident desktop client.
	Desktop,
}

/// A published entry, live for as long as this value exists.
pub struct Entry {
	path: PathBuf,
	file: File,
	run: Run,
}

impl Entry {
	pub fn run(&self) -> &Run {
		&self.run
	}

	/// Republish with the counter moved on.
	///
	/// Failure is deliberately swallowed by the progress sink that calls this: a registry that
	/// cannot be updated is an observability problem, and aborting a paid run over one would be
	/// the wrong trade.
	pub fn update(&mut self, done: u64, total: u64, message: &str) -> std::io::Result<()> {
		self.run.done = done;
		self.run.total = total;
		self.run.message = message.to_owned();
		self.write()
	}

	fn write(&mut self) -> std::io::Result<()> {
		let text = serde_json::to_string(&self.run).map_err(std::io::Error::other)?;
		self.file.set_len(0)?;
		self.file.rewind()?;
		self.file.write_all(text.as_bytes())?;
		self.file.flush()
	}
}

impl Drop for Entry {
	fn drop(&mut self) {
		// The lock goes with the handle; removing the file keeps a listing showing live work
		// rather than a season of corpses. A crash skips this and leaves an unlocked file, which
		// is exactly the case `live` is built to ignore.
		let _ = std::fs::remove_file(&self.path);
	}
}

pub fn directory(repository: &Path) -> PathBuf {
	repository.join(".cms").join("runs")
}

/// Publish a run. The entry stays live until the returned value is dropped or the process dies.
pub fn publish(repository: &Path, task: &str, shell: Shell, total: u64) -> std::io::Result<Entry> {
	let directory = directory(repository);
	std::fs::create_dir_all(&directory)?;
	// Keyed by pid and task, so one process running two different tasks publishes two entries and
	// a pid reused by the operating system after a crash cannot collide with a live run.
	let path = directory.join(format!("{}-{task}.run", std::process::id()));
	let file = OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(false)
		.open(&path)?;
	if file.try_lock().is_err() {
		return Err(std::io::Error::other(
			"this process already publishes a run for that task",
		));
	}
	let mut entry = Entry {
		path,
		file,
		run: Run {
			task: task.to_owned(),
			pid: std::process::id(),
			shell,
			started: crate::image::manifest::now(),
			done: 0,
			total,
			message: String::new(),
		},
	};
	entry.write()?;
	Ok(entry)
}

/// Every run alive on this machine for this repository.
pub fn live(repository: &Path) -> std::io::Result<Vec<Run>> {
	let Ok(entries) = std::fs::read_dir(directory(repository)) else {
		return Ok(Vec::new());
	};
	let mut found = Vec::new();
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|value| value.to_str()) != Some("run") {
			continue;
		}
		let Ok(file) = File::open(&path) else {
			continue;
		};
		if file.try_lock().is_ok() {
			// Taking it means nobody holds it, so the publisher is gone. Released at once: a
			// reader must not become the thing that keeps a dead entry locked.
			let _ = file.unlock();
			continue;
		}
		let mut text = String::new();
		let Ok(mut reader) = File::open(&path) else {
			continue;
		};
		if reader.read_to_string(&mut text).is_err() {
			continue;
		}
		if let Ok(run) = serde_json::from_str::<Run>(&text) {
			found.push(run);
		}
	}
	found.sort_by(|left, right| left.started.cmp(&right.started));
	Ok(found)
}

/// Whether a task is already running, and by whom.
///
/// The question a second CMS asks before deciding to trigger anything.
pub fn running(repository: &Path, task: &str) -> std::io::Result<Option<Run>> {
	Ok(live(repository)?.into_iter().find(|run| run.task == task))
}

/// A progress sink that republishes the registry entry as the run advances.
pub struct Published {
	entry: std::sync::Mutex<Entry>,
}

impl Published {
	pub fn new(entry: Entry) -> Self {
		Self {
			entry: std::sync::Mutex::new(entry),
		}
	}
}

impl super::progress::Sink for Published {
	fn started(&self, _total: u64) {}

	fn advanced(&self, done: u64, total: u64, message: &str) {
		if let Ok(mut entry) = self.entry.lock() {
			// Swallowed on purpose; see `Entry::update`.
			let _ = entry.update(done, total, message);
		}
	}

	fn finished(&self) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn temp(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("cms-registry-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	#[test]
	fn a_published_run_is_visible_to_a_reader() {
		let root = temp("visible");
		let entry = publish(&root, "favicon", Shell::Cli, 8).expect("publish");
		let listed = live(&root).expect("live");
		assert_eq!(listed.len(), 1);
		assert_eq!(listed[0].task, "favicon");
		assert_eq!(listed[0].total, 8);
		drop(entry);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn progress_reaches_the_reader() {
		let root = temp("progress");
		let mut entry = publish(&root, "i18n", Shell::Desktop, 100).expect("publish");
		entry
			.update(42, 100, "less-is-more.md ja-JP")
			.expect("update");
		let listed = live(&root).expect("live");
		assert_eq!(listed[0].done, 42);
		assert_eq!(listed[0].message, "less-is-more.md ja-JP");
		drop(entry);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn a_finished_run_stops_being_listed() {
		let root = temp("finished");
		let entry = publish(&root, "favicon", Shell::Cli, 1).expect("publish");
		drop(entry);
		assert!(live(&root).expect("live").is_empty());
		std::fs::remove_dir_all(root).ok();
	}

	/// The case that decides whether the whole registry is trustworthy: a file left behind by a
	/// process that died without cleaning up. It has plausible contents and no lock, and must be
	/// read as absent rather than as a run in progress.
	#[test]
	fn an_entry_left_by_a_dead_process_is_not_live() {
		let root = temp("corpse");
		let directory = directory(&root);
		std::fs::create_dir_all(&directory).expect("dir");
		std::fs::write(
			directory.join("999999-i18n.run"),
			serde_json::to_string(&Run {
				task: "i18n".to_owned(),
				pid: 999_999,
				shell: Shell::Cli,
				started: "2020-01-01T00:00:00Z".to_owned(),
				done: 3,
				total: 100,
				message: "half a translation".to_owned(),
			})
			.expect("json"),
		)
		.expect("write");

		assert!(live(&root).expect("live").is_empty());
		assert!(running(&root, "i18n").expect("running").is_none());
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn running_names_the_holder() {
		let root = temp("holder");
		let entry = publish(&root, "alt", Shell::Desktop, 24).expect("publish");
		let found = running(&root, "alt").expect("running").expect("some");
		assert_eq!(found.pid, std::process::id());
		assert_eq!(found.shell, Shell::Desktop);
		assert!(running(&root, "tag").expect("running").is_none());
		drop(entry);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn one_process_publishes_two_different_tasks_at_once() {
		let root = temp("two");
		let alt = publish(&root, "alt", Shell::Cli, 1).expect("alt");
		let tag = publish(&root, "tag", Shell::Cli, 1).expect("tag");
		assert_eq!(live(&root).expect("live").len(), 2);
		drop((alt, tag));
		std::fs::remove_dir_all(root).ok();
	}
}
