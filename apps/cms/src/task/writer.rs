//! The one place a record store is written.
//!
//! Workers compute in parallel and never write. Each finishes an item, hands over the mutation it
//! produced, and takes the next one. This applies them, one at a time. See spec/tasks.md.
//!
//! ## Two different races, two different answers
//!
//! **Inside a process**, the queue is the answer: one thread owns the store, so no two workers can
//! interleave a read-modify-write of the same file however many of them finish at once.
//!
//! **Between processes**, the queue says nothing -- the desktop client, a command in a terminal
//! and eventually the schedule are separate programs with separate queues. So applying takes an
//! exclusive lock on the record, held for that one apply. This is the lock the design is willing
//! to block on, because it is held for a file write rather than for a model call: microseconds
//! against minutes, which is the whole reason contention was moved off the compute path.
//!
//! A mutation is flushed as it is applied rather than batched to the end of a run. What is being
//! protected is paid output, and the write is cheap next to the call that produced the value.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;

use super::Record;

/// A unit of work for the writer: whatever the caller needs done while holding the record.
type Job = Box<dyn FnOnce() -> std::io::Result<()> + Send>;

enum Message {
	Apply(Job, mpsc::Sender<std::io::Result<()>>),
	Stop,
}

/// Serialised access to one record store.
///
/// Dropping it stops the thread after the queue drains, so mutations already handed over are
/// applied rather than discarded -- the opposite would lose exactly the paid results this exists
/// to protect.
pub struct Writer {
	record: Record,
	sender: Option<mpsc::Sender<Message>>,
	thread: Option<JoinHandle<()>>,
}

/// Where the cross-process lock for a record lives.
pub fn lock_path(repository: &Path, record: Record) -> PathBuf {
	repository.join(".cms").join("records").join(format!("{}.lock", name_of(record)))
}

/// A stable file-name-safe name per record. Written out rather than derived from the enum's
/// `Debug`, which would silently rename a lock file if the variant were ever renamed and let two
/// versions of this program disagree about which file guards which record.
fn name_of(record: Record) -> &'static str {
	match record {
		Record::Articles => "articles",
		Record::Translations => "translations",
		Record::Summaries => "summaries",
		Record::Notes => "notes",
		Record::Media => "media",
		Record::Tags => "tags",
		Record::Segments => "segments",
		Record::Embeds => "embeds",
		Record::PublicImage => "public-image",
		Record::PublicFavicon => "public-favicon",
		Record::PublicOpengraph => "public-opengraph",
		Record::PublicLicense => "public-license",
	}
}

impl Writer {
	/// Start the writer for one record.
	pub fn start(repository: &Path, record: Record) -> std::io::Result<Self> {
		let path = lock_path(repository, record);
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		let (sender, receiver) = mpsc::channel::<Message>();
		let thread = std::thread::Builder::new()
			.name(format!("cms-writer-{}", name_of(record)))
			.spawn(move || {
				for message in receiver {
					let (job, reply) = match message {
						Message::Apply(job, reply) => (job, reply),
						Message::Stop => break,
					};
					let result = with_record_lock(&path, job);
					// A caller that stopped listening is not an error worth failing the run over;
					// the mutation was still applied, which is the part that mattered.
					let _ = reply.send(result);
				}
			})?;
		Ok(Self { record, sender: Some(sender), thread: Some(thread) })
	}

	pub fn record(&self) -> Record {
		self.record
	}

	/// Hand a mutation over and wait for it to be applied.
	///
	/// Waiting here is waiting on a file write, not on the work that produced the value, so it
	/// does not put a worker back behind another worker's model call.
	pub fn apply<F>(&self, job: F) -> std::io::Result<()>
	where
		F: FnOnce() -> std::io::Result<()> + Send + 'static,
	{
		let (reply, answer) = mpsc::channel();
		let sender =
			self.sender.as_ref().ok_or_else(|| std::io::Error::other("the writer is stopping"))?;
		sender
			.send(Message::Apply(Box::new(job), reply))
			.map_err(|_| std::io::Error::other("the writer has stopped"))?;
		answer.recv().map_err(|_| std::io::Error::other("the writer stopped before applying"))?
	}
}

impl Drop for Writer {
	fn drop(&mut self) {
		if let Some(sender) = self.sender.take() {
			let _ = sender.send(Message::Stop);
		}
		if let Some(thread) = self.thread.take() {
			let _ = thread.join();
		}
	}
}

/// Run one job with the record held against every other process.
fn with_record_lock(path: &Path, job: Job) -> std::io::Result<()> {
	let file =
		std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
	// Blocking, unlike a claim. A claim asks "is somebody else doing this work", where the answer
	// "yes" means there is nothing to do; here the work is ours and only the file is contended.
	file.lock()?;
	let result = job();
	// Released explicitly so the order is visible: the mutation is durable before anybody else can
	// read the record. Dropping the handle would do the same, later and less obviously.
	let _ = file.unlock();
	result
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	fn temp(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("cms-writer-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	#[test]
	fn mutations_are_applied() {
		let root = temp("applied");
		let writer = Writer::start(&root, Record::Media).expect("writer");
		let counter = Arc::new(AtomicUsize::new(0));
		for _ in 0..8 {
			let counter = Arc::clone(&counter);
			writer
				.apply(move || {
					counter.fetch_add(1, Ordering::SeqCst);
					Ok(())
				})
				.expect("apply");
		}
		assert_eq!(counter.load(Ordering::SeqCst), 8);
		drop(writer);
		std::fs::remove_dir_all(root).ok();
	}

	/// The race this layer exists for: several workers finishing at once, each reading the store,
	/// changing it and writing it back. Unserialised, the interleaving loses updates -- which on a
	/// translation run is paid output silently vanishing.
	#[test]
	fn concurrent_read_modify_write_loses_nothing() {
		let root = temp("serial");
		let store = root.join("store");
		std::fs::write(&store, "0").expect("seed");
		let writer = Arc::new(Writer::start(&root, Record::Media).expect("writer"));

		let mut workers = Vec::new();
		for _ in 0..8 {
			let writer = Arc::clone(&writer);
			let store = store.clone();
			workers.push(std::thread::spawn(move || {
				for _ in 0..25 {
					let store = store.clone();
					writer
						.apply(move || {
							let current: u32 =
								std::fs::read_to_string(&store)?.trim().parse().map_err(std::io::Error::other)?;
							// Widening the window on purpose: without serialisation this loses
							// updates every run rather than occasionally.
							std::thread::sleep(std::time::Duration::from_micros(50));
							std::fs::write(&store, (current + 1).to_string())
						})
						.expect("apply");
				}
			}));
		}
		for worker in workers {
			worker.join().expect("join");
		}

		assert_eq!(std::fs::read_to_string(&store).expect("read").trim(), "200");
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn a_failing_mutation_reports_to_its_caller() {
		let root = temp("failure");
		let writer = Writer::start(&root, Record::Tags).expect("writer");
		let result = writer.apply(|| Err(std::io::Error::other("no")));
		assert!(result.is_err());
		// The writer survives a failed mutation; one bad item must not end a paid run.
		writer.apply(|| Ok(())).expect("still alive");
		drop(writer);
		std::fs::remove_dir_all(root).ok();
	}

	/// Queued work is applied before the writer goes away. Discarding it would throw away results
	/// that have already been paid for.
	#[test]
	fn dropping_the_writer_drains_what_was_handed_over() {
		let root = temp("drain");
		let counter = Arc::new(AtomicUsize::new(0));
		{
			let writer = Writer::start(&root, Record::Notes).expect("writer");
			for _ in 0..4 {
				let counter = Arc::clone(&counter);
				writer
					.apply(move || {
						counter.fetch_add(1, Ordering::SeqCst);
						Ok(())
					})
					.expect("apply");
			}
		}
		assert_eq!(counter.load(Ordering::SeqCst), 4);
		std::fs::remove_dir_all(root).ok();
	}

	#[test]
	fn each_record_has_its_own_lock_file() {
		let root = temp("paths");
		let media = lock_path(&root, Record::Media);
		let tags = lock_path(&root, Record::Tags);
		assert_ne!(media, tags);
		assert!(media.ends_with("media.lock"));
		std::fs::remove_dir_all(root).ok();
	}
}
