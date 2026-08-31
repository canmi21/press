//! Where a run reports how far it has got.
//!
//! This was an `indicatif::ProgressBar` handed straight to five commands, which made the terminal
//! the only thing a long run could talk to. A desktop client watching the same operation had
//! nowhere to read from, and the operation could not be moved below the shells without taking a
//! terminal bar with it. See spec/tasks.md.
//!
//! The shape stays a counter and a message because that is what the five call sites already say,
//! and a migration that also redesigns what progress *means* cannot be checked against the
//! behaviour it replaced. What changes is only where the counter goes.

use std::sync::Mutex;

/// Somewhere progress can be written.
///
/// `Send + Sync` because a run may report from several worker threads at once; the implementation
/// decides how to serialise, since a terminal bar and an event channel have different answers.
pub trait Sink: Send + Sync {
	/// Called once, before any advance.
	fn started(&self, total: u64);
	/// The current count, out of the total, with whatever the run is working on now.
	fn advanced(&self, done: u64, total: u64, message: &str);
	/// The run is over. A sink that drew something clears it.
	fn finished(&self);

	/// Run `body` without whatever this sink drew getting in the way.
	///
	/// A terminal bar redraws over the cursor, so a line printed while it is on screen lands in
	/// the middle of it. A sink that draws nothing has nothing to move, which is why the default
	/// is simply to call the closure -- most sinks want this and none of them should have to say
	/// so.
	fn suspend(&self, body: &mut dyn FnMut()) {
		body();
	}
}

/// A run's progress, counted here and reported to whatever is listening.
///
/// The count lives in this struct rather than in the sink so that every sink agrees about it, and
/// so a sink can be swapped for a silent one in a test without losing the arithmetic that the
/// assertions are about.
pub struct Progress {
	total: u64,
	state: Mutex<State>,
	sink: Box<dyn Sink>,
}

#[derive(Default)]
struct State {
	done: u64,
	message: String,
}

impl Progress {
	pub fn new(total: u64, sink: Box<dyn Sink>) -> Self {
		sink.started(total);
		Self { total, state: Mutex::new(State::default()), sink }
	}

	/// A run whose progress nobody is watching.
	pub fn silent(total: u64) -> Self {
		Self::new(total, Box::new(Silent))
	}

	/// A run drawing the terminal bar, for the commands the CLI still owns end to end.
	pub fn new_terminal(total: u64) -> Self {
		Self::new(total, Box::new(Terminal::new()))
	}

	pub fn total(&self) -> u64 {
		self.total
	}

	pub fn done(&self) -> u64 {
		self.state.lock().map(|state| state.done).unwrap_or(0)
	}

	pub fn set_message(&self, message: impl Into<String>) {
		let Ok(mut state) = self.state.lock() else {
			return;
		};
		state.message = message.into();
		self.sink.advanced(state.done, self.total, &state.message);
	}

	pub fn inc(&self, by: u64) {
		let Ok(mut state) = self.state.lock() else {
			return;
		};
		// Saturating rather than wrapping: a miscounted total is a cosmetic bug, and a counter
		// that wraps to zero at the end of a paid run looks like the run restarted.
		state.done = state.done.saturating_add(by);
		self.sink.advanced(state.done, self.total, &state.message);
	}

	pub fn finish_and_clear(&self) {
		self.sink.finished();
	}

	/// Print, or otherwise interrupt, without the sink's own output colliding with it.
	pub fn suspend<F: FnMut()>(&self, mut body: F) {
		self.sink.suspend(&mut body);
	}
}

/// Reports nowhere. The default for tests and for a run nobody asked to watch.
pub struct Silent;

impl Sink for Silent {
	fn started(&self, _total: u64) {}
	fn advanced(&self, _done: u64, _total: u64, _message: &str) {}
	fn finished(&self) {}
}

/// Draws the bar the commands have always drawn.
///
/// The template is a decision rather than a detail: a fixed-width bar, counts, and a message that
/// gets whatever room is left. Long titles and CJK previews both live in that last field, which is
/// why it is the one allowed to flex.
pub struct Terminal {
	bar: indicatif::ProgressBar,
}

impl Terminal {
	pub fn new() -> Self {
		Self { bar: indicatif::ProgressBar::hidden() }
	}
}

impl Default for Terminal {
	fn default() -> Self {
		Self::new()
	}
}

impl Sink for Terminal {
	fn started(&self, total: u64) {
		self.bar.set_length(total);
		// Falls back to the default style rather than failing: a malformed template is a cosmetic
		// problem, and refusing to run a paid batch over one would be the wrong trade.
		self.bar.set_style(
			indicatif::ProgressStyle::with_template("  {bar:28} {pos}/{len}  {wide_msg}")
				.unwrap_or_else(|_| indicatif::ProgressStyle::default_bar()),
		);
		self.bar.set_draw_target(indicatif::ProgressDrawTarget::stderr());
	}

	fn advanced(&self, done: u64, _total: u64, message: &str) {
		self.bar.set_position(done);
		self.bar.set_message(message.to_owned());
	}

	fn finished(&self) {
		self.bar.finish_and_clear();
	}

	fn suspend(&self, body: &mut dyn FnMut()) {
		self.bar.suspend(body);
	}
}

/// A line short enough to sit in the message field beside the bar.
///
/// Measured in display columns rather than characters: a CJK glyph occupies two, so counting
/// characters overflows the row on exactly the articles this site publishes.
pub fn preview(source: &str, columns: usize) -> String {
	let line = source.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
	let flat = line.split_whitespace().collect::<Vec<_>>().join(" ");
	let width = |c: char| if (c as u32) > 0x2e80 { 2 } else { 1 };
	let mut out = String::new();
	let mut used = 0usize;
	for c in flat.chars() {
		if used + width(c) > columns {
			out.push('…');
			break;
		}
		used += width(c);
		out.push(c);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicU64, Ordering};

	#[derive(Default)]
	struct Counting {
		advances: AtomicU64,
		last: Mutex<(u64, u64, String)>,
		finished: AtomicU64,
	}

	impl Sink for Arc<Counting> {
		fn started(&self, _total: u64) {}
		fn advanced(&self, done: u64, total: u64, message: &str) {
			self.advances.fetch_add(1, Ordering::Relaxed);
			if let Ok(mut last) = self.last.lock() {
				*last = (done, total, message.to_owned());
			}
		}
		fn finished(&self) {
			self.finished.fetch_add(1, Ordering::Relaxed);
		}
	}

	#[test]
	fn the_count_is_kept_here_rather_than_in_the_sink() {
		let sink = Arc::new(Counting::default());
		let progress = Progress::new(4, Box::new(Arc::clone(&sink)));
		progress.inc(1);
		progress.inc(2);
		assert_eq!(progress.done(), 3);
		assert_eq!(sink.last.lock().expect("last").0, 3);
	}

	/// Setting a message is an advance report too. A run that names what it is working on before
	/// finishing the first item would otherwise show nothing for the length of that item, which on
	/// a translation is minutes.
	#[test]
	fn a_message_reaches_the_sink_without_waiting_for_an_increment() {
		let sink = Arc::new(Counting::default());
		let progress = Progress::new(2, Box::new(Arc::clone(&sink)));
		progress.set_message("article.md");
		assert_eq!(sink.last.lock().expect("last").2, "article.md");
		assert_eq!(progress.done(), 0);
	}

	/// A total that undercounts must not make the bar look like it started over.
	#[test]
	fn the_counter_saturates_rather_than_wrapping() {
		let progress = Progress::silent(1);
		progress.inc(u64::MAX);
		progress.inc(5);
		assert_eq!(progress.done(), u64::MAX);
	}

	#[test]
	fn finishing_is_reported_once() {
		let sink = Arc::new(Counting::default());
		let progress = Progress::new(1, Box::new(Arc::clone(&sink)));
		progress.finish_and_clear();
		assert_eq!(sink.finished.load(Ordering::Relaxed), 1);
	}

	#[test]
	fn a_preview_is_measured_in_columns_rather_than_characters() {
		// Twenty-two CJK glyphs occupy forty-four columns, which is the whole budget; counting
		// characters would have let twice that through and wrapped the terminal row.
		let cjk = "不许 Cargo 再摸鱼了，来看看实践中的 Rust 开发配置调优吧";
		let out = preview(cjk, 20);
		let columns: usize = out.chars().map(|c| if (c as u32) > 0x2e80 { 2 } else { 1 }).sum();
		assert!(columns <= 21, "{out} used {columns} columns");
		assert!(out.ends_with('…'));
	}

	#[test]
	fn a_short_line_is_left_alone() {
		assert_eq!(preview("short", 40), "short");
	}

	#[test]
	fn a_preview_takes_the_first_line_with_anything_on_it() {
		assert_eq!(preview("\n\n  first real line\nsecond", 40), "first real line");
	}
}
