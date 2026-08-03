//! The one progress bar every long-running command draws.
//!
//! Four commands had grown their own copy of the same twelve lines, with the same template
//! string repeated four times. That is not a cost worth paying twice over, and it is how a
//! fifth command ends up with no bar at all -- there was nothing to call, so `cms tn` simply
//! printed nothing for two minutes while it read five articles.
//!
//! The template is a decision rather than a detail: a fixed-width bar, counts, and a message
//! that gets whatever room is left. Long titles and CJK previews both live in that last field,
//! which is why it is the one allowed to flex.

use indicatif::{ProgressBar, ProgressStyle};

/// A bar for a run of `total` items.
///
/// Falls back to the default style rather than failing: a malformed template is a cosmetic
/// problem, and refusing to run a paid batch over one would be the wrong trade.
pub fn bar(total: u64) -> ProgressBar {
	let bar = ProgressBar::new(total);
	bar.set_style(
		ProgressStyle::with_template("  {bar:28} {pos}/{len}  {wide_msg}")
			.unwrap_or_else(|_| ProgressStyle::default_bar()),
	);
	bar
}

/// A line of text short enough to sit in the message field beside the bar.
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

	#[test]
	fn a_preview_is_measured_in_columns_rather_than_characters() {
		// Twenty-two CJK glyphs occupy forty-four columns, which is the whole budget; counting
		// characters would have let twice that through and wrapped the terminal row.
		let cjk = "不许 Cargo 再摸鱼了，来看看实践中的 Rust 开发配置调优吧";
		let out = preview(cjk, 20);
		let columns: usize = out
			.chars()
			.map(|c| if (c as u32) > 0x2e80 { 2 } else { 1 })
			.sum();
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
