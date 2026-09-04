//! Who is already doing this piece of work.
//!
//! Claims are per item -- a content id, an article, a (segment, locale) pair -- and atomic across
//! processes. A claimed item is skipped, never waited for: waiting is an ordering, ordering is the
//! scheduler's job, and there is no scheduler. See spec/tasks.md.
//!
//! ## Why a lock and not a status
//!
//! A claim that records "taken" as a written value survives the process that wrote it. `SIGKILL`,
//! a panic or a power cut then leaves an item claimed forever, every later run skips it, and the
//! only repair is somebody deleting files and guessing whether that was safe.
//!
//! So the file carries the metadata and an exclusive lock on it carries the fact. A claimant holds
//! the lock for as long as it works; anyone else takes the claim by taking the lock, and taking it
//! successfully *means* the previous holder is gone. The kernel drops the lock when a process
//! dies, however it dies, which is why this needs no heartbeat, no timeout and no daemon.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

/// What a claim file records. None of it is trusted for liveness; see the module note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Held {
	pub task: String,
	/// The work item, in the form the task itself uses. Written so a person reading the directory
	/// can tell what is claimed without reversing the file name.
	pub key: String,
	pub pid: u32,
	/// ISO 8601 UTC, when the claim was taken.
	pub at: String,
}

/// A claim this process holds. Releasing happens on drop, including while unwinding.
#[derive(Debug)]
pub struct Claim {
	path: PathBuf,
	// Held open for the lock, which is released when this closes. Never read after construction.
	file: Option<File>,
}

impl Claim {
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl Drop for Claim {
	fn drop(&mut self) {
		// The lock goes with the handle either way; removing the file is tidiness so a directory
		// listing shows live work rather than a season of corpses. A failure here is not worth
		// reporting: the next claimant reclaims a stale file by taking its lock.
		drop(self.file.take());
		let _ = std::fs::remove_file(&self.path);
	}
}

/// Why a claim could not be taken.
#[derive(Debug)]
pub enum Denied {
	/// Somebody live holds it. Carries what their file said, which may be stale in every field
	/// except the one that matters -- that it is theirs.
	Taken(Box<Held>),
	Io(std::io::Error),
}

impl std::fmt::Display for Denied {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Taken(held) => {
				write!(formatter, "{} is already claimed by cms {} (pid {})", held.key, held.task, held.pid)
			}
			Self::Io(error) => error.fmt(formatter),
		}
	}
}

impl From<std::io::Error> for Denied {
	fn from(error: std::io::Error) -> Self {
		Self::Io(error)
	}
}

/// Where claims live for a repository.
pub fn directory(repository: &Path) -> PathBuf {
	repository.join(".cms").join("claims")
}

/// The file name for an item key.
///
/// Hashed because a key is arbitrary -- an article path with slashes, a segment id and a locale --
/// and file names are not. This is not the keying that was rejected for the state directory
/// itself: nothing needs to identify an item *from* its file name, because the key is written
/// inside the file, so the hash costs no legibility here.
fn file_name(task: &str, key: &str) -> String {
	format!("{}.claim", crate::i18n::segment::id_of(&format!("{task}\u{0}{key}")))
}

fn now() -> String {
	crate::image::manifest::now()
}

/// Take the claim on one item, or report who has it.
///
/// A file left by a dead process is reclaimed rather than respected: its lock is free, which is
/// the only evidence of death this design accepts or needs.
pub fn take(repository: &Path, task: &str, key: &str) -> Result<Claim, Denied> {
	let directory = directory(repository);
	std::fs::create_dir_all(&directory)?;
	let path = directory.join(file_name(task, key));

	let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path)?;

	// The whole decision. Everything else in this function is bookkeeping around it.
	if file.try_lock().is_err() {
		let mut existing = String::new();
		let mut reader = File::open(&path)?;
		let _ = reader.read_to_string(&mut existing);
		let held = serde_json::from_str::<Held>(&existing).unwrap_or(Held {
			task: task.to_owned(),
			key: key.to_owned(),
			pid: 0,
			at: String::new(),
		});
		return Err(Denied::Taken(Box::new(held)));
	}

	// Written after the lock is ours, so a reader never sees another process's metadata under our
	// lock. Truncated first because a reclaimed file still holds the dead holder's record.
	let held =
		Held { task: task.to_owned(), key: key.to_owned(), pid: std::process::id(), at: now() };
	let mut file = file;
	file.set_len(0)?;
	file.rewind()?;
	file.write_all(serde_json::to_string(&held)?.as_bytes())?;
	file.flush()?;

	Ok(Claim { path, file: Some(file) })
}

impl From<serde_json::Error> for Denied {
	fn from(error: serde_json::Error) -> Self {
		Self::Io(std::io::Error::other(error.to_string()))
	}
}

/// Every claim currently held for a repository, dead ones omitted.
///
/// Liveness is tested the same way it is taken: an entry whose lock can be acquired belonged to a
/// process that is gone. This does not reclaim them -- reading what is running must not mutate
/// what is running -- so a corpse is simply left out of the answer.
pub fn live(repository: &Path) -> std::io::Result<Vec<Held>> {
	let directory = directory(repository);
	let Ok(entries) = std::fs::read_dir(&directory) else {
		return Ok(Vec::new());
	};
	let mut found = Vec::new();
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|value| value.to_str()) != Some("claim") {
			continue;
		}
		let Ok(file) = File::open(&path) else {
			continue;
		};
		if file.try_lock().is_ok() {
			// Ours now, which means nobody's. Drop the lock without touching the file.
			let _ = file.unlock();
			continue;
		}
		let mut text = String::new();
		let mut reader = match File::open(&path) {
			Ok(reader) => reader,
			Err(_) => continue,
		};
		if reader.read_to_string(&mut text).is_err() {
			continue;
		}
		if let Ok(held) = serde_json::from_str::<Held>(&text) {
			found.push(held);
		}
	}
	found.sort_by(|left, right| left.key.cmp(&right.key));
	Ok(found)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A directory that removes itself, however the test ends.
	///
	/// `TempDir` deletes on drop, which the hand-rolled predecessor could not: a panicking test
	/// left its directory behind, and the name carried the process id because two tests choosing
	/// the same one would otherwise share a directory. Both problems belonged to the workaround.
	fn temp() -> tempfile::TempDir {
		tempfile::tempdir().expect("temp")
	}

	#[test]
	fn a_claim_excludes_a_second_taker() {
		let temporary = temp();
		let root = temporary.path();
		let first = take(&root, "favicon", "example.com").expect("first");
		match take(&root, "favicon", "example.com") {
			Err(Denied::Taken(held)) => {
				assert_eq!(held.key, "example.com");
				assert_eq!(held.pid, std::process::id());
			}
			other => panic!("expected the second take to be denied, got {other:?}"),
		}
		drop(first);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn different_items_of_one_task_do_not_contend() {
		let temporary = temp();
		let root = temporary.path();
		let one = take(&root, "i18n", "a.md#seg1#ja-JP").expect("one");
		let two = take(&root, "i18n", "a.md#seg2#ja-JP").expect("two");
		drop((one, two));
		std::fs::remove_dir_all(root).ok();
	}

	/// The same key under two tasks is two items. `alt` and `tag` both work per content id, and
	/// treating those as one claim would serialise the pair this design exists to keep parallel.
	#[test]
	fn the_same_key_under_two_tasks_is_two_claims() {
		let temporary = temp();
		let root = temporary.path();
		let alt = take(&root, "alt", "44b6081d").expect("alt");
		let tag = take(&root, "tag", "44b6081d").expect("tag");
		drop((alt, tag));
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn releasing_lets_the_next_taker_through() {
		let temporary = temp();
		let root = temporary.path();
		let first = take(&root, "favicon", "example.com").expect("first");
		drop(first);
		take(&root, "favicon", "example.com").expect("second");
		std::fs::remove_dir_all(root).ok();
	}

	/// The case the whole design is shaped around: a claim file whose process died. It is left
	/// behind with its metadata intact and no lock, and must be reclaimed rather than respected.
	#[test]
	fn a_claim_left_by_a_dead_process_is_reclaimed() {
		let temporary = temp();
		let root = temporary.path();
		let directory = directory(&root);
		std::fs::create_dir_all(&directory).expect("dir");
		let path = directory.join(file_name("favicon", "ghost.example"));
		std::fs::write(
			&path,
			serde_json::to_string(&Held {
				task: "favicon".to_owned(),
				key: "ghost.example".to_owned(),
				pid: 999_999,
				at: "2020-01-01T00:00:00Z".to_owned(),
			})
			.expect("json"),
		)
		.expect("write");

		let reclaimed = take(&root, "favicon", "ghost.example").expect("reclaim");
		assert_eq!(reclaimed.path(), path);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn a_dead_claim_is_left_out_of_the_live_listing() {
		let temporary = temp();
		let root = temporary.path();
		let directory = directory(&root);
		std::fs::create_dir_all(&directory).expect("dir");
		std::fs::write(
			directory.join(file_name("favicon", "ghost.example")),
			serde_json::to_string(&Held {
				task: "favicon".to_owned(),
				key: "ghost.example".to_owned(),
				pid: 999_999,
				at: String::new(),
			})
			.expect("json"),
		)
		.expect("write");

		let held = take(&root, "favicon", "alive.example").expect("alive");
		let listed = live(&root).expect("live");
		assert_eq!(
			listed.iter().map(|entry| entry.key.as_str()).collect::<Vec<_>>(),
			vec!["alive.example"]
		);
		drop(held);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn nothing_claimed_lists_nothing() {
		let temporary = temp();
		let root = temporary.path();
		assert!(live(&root).expect("live").is_empty());
		std::fs::remove_dir_all(root).ok();
	}
}
