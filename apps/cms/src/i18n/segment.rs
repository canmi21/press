//! Cutting an article into the units a translation is addressed by.
//!
//! A segment is a markdown block, and its id is the hash of its canonical text. That is the whole
//! synchronisation mechanism: edit a paragraph and only that paragraph's id changes, so only
//! its translations go stale. Move a paragraph and nothing changes at all, because order lives
//! in the article and never in the sidecar. See spec/i18n.md.

use crate::document::{self, Malformed};
use std::collections::BTreeMap;

const TRANSLATABLE_FRONTMATTER: [&str; 3] = ["title", "subtitle", "description"];

/// Where a segment is spliced back into the article.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
	Frontmatter,
	Body,
}

/// What a block is, which decides whether it is translated and by which model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
	/// Prose. Carries register, idiom and the things only a careful model gets right.
	Prose,
	/// A heading. Structural text, short, no register to lose.
	Heading,
	/// A quotation. Translated, but its language is the author's choice to preserve.
	Quote,
	/// Code. Never translated, never sent -- including any comments inside it. A code block is
	/// an executable fact, and editing it would raise the question of where to stop.
	Code,
	/// A directive. Structure only, never translated.
	Directive,
}

impl Kind {
	pub fn translatable(self) -> bool {
		!matches!(self, Self::Code | Self::Directive)
	}

	/// Whether this block is structural enough that a light model is not a gamble.
	///
	/// Decided by what the block is rather than by scoring its content. Length and punctuation
	/// density are weak proxies for difficulty, and the things that actually make a passage
	/// hard -- a pun, a register, a cultural reference -- are exactly what cannot be counted.
	pub fn is_light(self) -> bool {
		self == Self::Heading
	}
}

#[derive(Debug, Clone)]
pub struct Segment {
	pub id: String,
	pub kind: Kind,
	/// The prose sent for translation. Body prose is byte-identical to its source span;
	/// frontmatter prose is the YAML scalar's decoded value.
	pub source: String,
	pub region: Region,
	/// Byte offsets in the complete source article. Stored in the build artifact.
	pub start: usize,
	pub end: usize,
	/// Line number in the article, for reporting only. Never stored.
	pub line: usize,
}

/// The sentinel wrapped around anything that must survive translation untouched.
///
/// `⟦⟧` are U+27E6 and U+27E7, mathematical white square brackets. They are chosen for one
/// property: prose does not contain them. A `$1`-style marker would be ambiguous in an article
/// about shells or regular expressions, which is exactly the kind this site publishes.
pub const OPEN: char = '⟦';
pub const CLOSE: char = '⟧';

/// A block with its unstranslatable parts lifted out.
#[derive(Debug, Clone)]
pub struct Masked {
	pub text: String,
	pub slots: Vec<String>,
}

impl Masked {
	/// Put the lifted parts back where the markers ended up.
	pub fn restore(&self, translated: &str) -> String {
		let mut out = translated.to_owned();
		for (index, original) in self.slots.iter().enumerate() {
			out = out.replace(&marker(index), original);
		}
		out
	}

	/// Whether a translation still carries every marker exactly once.
	///
	/// The point of masking is not to hope the model leaves code alone but to be able to prove
	/// it did. A lost or duplicated marker fails here and the segment is retried, rather than
	/// silently producing prose with a mangled identifier in it.
	pub fn intact(&self, translated: &str) -> bool {
		(0..self.slots.len()).all(|index| translated.matches(&marker(index)).count() == 1)
	}
}

pub fn marker(index: usize) -> String {
	format!("{OPEN}tk:{index}{CLOSE}")
}

/// Lift inline code out of a block, leaving markers in its place.
///
/// Without this a paragraph would have to be split around every `` `Cargo.toml` `` -- 180 of
/// them across these articles -- which fragments the context a translator needs and costs more
/// in requests than it saves in tokens.
pub fn mask(source: &str) -> Masked {
	let mut text = String::with_capacity(source.len());
	let mut slots = Vec::new();
	let mut rest = source;

	while let Some(start) = rest.find('`') {
		let after = &rest[start + 1..];
		let Some(end) = after.find('`') else {
			break;
		};
		text.push_str(&rest[..start]);
		text.push_str(&marker(slots.len()));
		slots.push(rest[start..start + 1 + end + 1].to_owned());
		rest = &after[end + 1..];
	}
	text.push_str(rest);

	Masked { text, slots }
}

/// The id of a block: the hash of its canonical bytes, truncated the same way asset ids are.
///
/// The TypeScript write path makes the article canonical before Rust sees it. Reimplementing
/// remark here would create a second canonical form instead of strengthening the first. See
/// spec/i18n.md.
pub fn id_of(source: &str) -> String {
	crate::image::cid(source.as_bytes())
}

/// Split allowlisted frontmatter values and body blocks; the frontmatter block itself is never
/// a segment.
pub fn split(article: &str) -> Result<Vec<Segment>, Malformed> {
	let document::Document {
		frontmatter,
		frontmatter_start,
		body,
		body_start,
	} = document::split(article)?;

	let mut segments = match frontmatter {
		Some(source) => frontmatter_segments(source, frontmatter_start)?,
		None => Vec::new(),
	};
	let mut block: Vec<&str> = Vec::new();
	let mut fenced = false;
	let mut start_line = 0;
	let mut block_start = body_start;
	let mut block_end = body_start;
	let mut cursor = body_start;
	let body_line = article[..body_start]
		.bytes()
		.filter(|byte| *byte == b'\n')
		.count();

	for (offset, line) in body.split('\n').enumerate() {
		let line_start = cursor;
		let line_end = line_start + line.len();
		cursor = line_end.saturating_add(1);
		if line.trim_start().starts_with("```") {
			// A fence toggles: inside one, blank lines are content rather than separators.
			if fenced {
				block.push(line);
				block_end = line_end;
				push(
					article,
					&mut segments,
					&mut block,
					body_line + start_line,
					block_start,
					block_end,
				);
				fenced = false;
				continue;
			}
			push(
				article,
				&mut segments,
				&mut block,
				body_line + start_line,
				block_start,
				block_end,
			);
			fenced = true;
			start_line = offset;
			block_start = line_start;
			block_end = line_end;
			block.push(line);
			continue;
		}
		if fenced {
			block.push(line);
			block_end = line_end;
			continue;
		}
		if line.trim().is_empty() {
			push(
				article,
				&mut segments,
				&mut block,
				body_line + start_line,
				block_start,
				block_end,
			);
			continue;
		}
		if block.is_empty() {
			start_line = offset;
			block_start = line_start;
		}
		block.push(line);
		block_end = line_end;
	}
	push(
		article,
		&mut segments,
		&mut block,
		body_line + start_line,
		block_start,
		block_end,
	);
	Ok(segments)
}

fn frontmatter_segments(
	frontmatter: &str,
	absolute_start: usize,
) -> Result<Vec<Segment>, Malformed> {
	let values: serde_yaml_ng::Value =
		serde_yaml_ng::from_str(frontmatter).map_err(|error| Malformed::NotYaml(error.to_string()))?;
	let Some(values) = values.as_mapping() else {
		return Err(Malformed::NotAMapping);
	};

	let mut lines = Vec::new();
	let mut cursor = 0usize;
	for line in frontmatter.split_inclusive('\n') {
		let text = line.strip_suffix('\n').unwrap_or(line);
		lines.push((cursor, cursor + text.len(), text));
		cursor += line.len();
	}
	if frontmatter.is_empty() {
		return Ok(Vec::new());
	}

	let mut segments = Vec::new();
	for (index, (line_start, line_end, line)) in lines.iter().copied().enumerate() {
		if line.starts_with(char::is_whitespace) || line.starts_with('#') {
			continue;
		}
		let Some(colon) = line.find(':') else {
			continue;
		};
		let key = &line[..colon];
		if !TRANSLATABLE_FRONTMATTER.contains(&key) {
			continue;
		}
		let source = match values.get(serde_yaml_ng::Value::String(key.to_owned())) {
			Some(serde_yaml_ng::Value::String(source)) => source,
			Some(_) => {
				return Err(Malformed::NotText(key.to_owned()));
			}
			None => continue,
		};
		if source.is_empty() {
			continue;
		}

		let mut end = line_end;
		for (_, continuation_end, continuation) in lines.iter().skip(index + 1).copied() {
			if !continuation.is_empty() && !continuation.starts_with(char::is_whitespace) {
				break;
			}
			end = continuation_end;
		}
		segments.push(Segment {
			id: id_of(source),
			kind: if matches!(key, "title" | "subtitle") {
				Kind::Heading
			} else {
				Kind::Prose
			},
			source: source.clone(),
			region: Region::Frontmatter,
			start: absolute_start + line_start + colon + 1,
			end: absolute_start + end,
			line: index + 2,
		});
	}
	Ok(segments)
}

fn push(
	article: &str,
	into: &mut Vec<Segment>,
	block: &mut Vec<&str>,
	line: usize,
	start: usize,
	end: usize,
) {
	if block.is_empty() {
		return;
	}
	let source = block.join("\n");
	block.clear();
	debug_assert_eq!(article.get(start..end), Some(source.as_str()));
	let trimmed = source.trim();
	if trimmed.is_empty() {
		return;
	}
	let kind = if trimmed.starts_with("```") {
		Kind::Code
	} else if trimmed.starts_with('#') {
		Kind::Heading
	} else if trimmed.starts_with('>') {
		Kind::Quote
	} else if trimmed.starts_with("::") {
		Kind::Directive
	} else {
		Kind::Prose
	};
	into.push(Segment {
		id: id_of(&source),
		kind,
		source,
		region: Region::Body,
		start,
		end,
		line: line + 1,
	});
}

/// Every segment worth translating, keyed by id, deduplicated.
pub fn translatable(article: &str) -> Result<BTreeMap<String, Segment>, Malformed> {
	Ok(
		split(article)?
			.into_iter()
			.filter(|segment| segment.kind.translatable())
			.map(|segment| (segment.id.clone(), segment))
			.collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Frontmatter a person mistyped is an ordinary event, not a broken invariant.
	///
	/// It used to abort the process with a message naming the fault and not the file, which left
	/// a binary search through the corpus as the only way to find out which article it was. The
	/// caller knows the path; this only has to hand back something it can attach one to.
	#[test]
	fn frontmatter_that_does_not_parse_is_an_error_rather_than_a_crash() {
		let error = split("---\ntitle: [unclosed\n---\n\nBody.").expect_err("refused");
		assert!(matches!(error, Malformed::NotYaml(_)));
		assert!(error.to_string().contains("not valid YAML"));
	}

	#[test]
	fn frontmatter_that_is_not_a_mapping_is_named_as_such() {
		let error = split("---\n- just\n- a list\n---\n\nBody.").expect_err("refused");
		assert_eq!(error, Malformed::NotAMapping);
	}

	#[test]
	fn a_translatable_key_holding_something_other_than_text_names_the_key() {
		let error = split("---\ntitle:\n  nested: value\n---\n\nBody.").expect_err("refused");
		assert_eq!(error, Malformed::NotText("title".to_owned()));
		assert!(error.to_string().contains("`title`"));
	}

	#[test]
	fn an_edit_changes_only_its_own_segment() {
		// The whole synchronisation design rests on this: a small change must not invalidate
		// the translations of everything around it.
		let before = split("first para\n\nsecond para\n\nthird para").expect("segments");
		let after = split("first para\n\nsecond para edited\n\nthird para").expect("segments");
		assert_eq!(before[0].id, after[0].id);
		assert_ne!(before[1].id, after[1].id);
		assert_eq!(before[2].id, after[2].id);
	}

	#[test]
	fn moving_a_paragraph_changes_nothing() {
		// Order lives in the article, so the sidecar has no opinion about it.
		let a = split("alpha\n\nbeta").expect("segments");
		let b = split("beta\n\nalpha").expect("segments");
		assert_eq!(a[0].id, b[1].id);
		assert_eq!(a[1].id, b[0].id);
	}

	#[test]
	fn the_id_is_the_hash_of_the_canonical_bytes_on_disk() {
		let source = "one two three";
		assert_eq!(id_of(source), crate::image::cid(source.as_bytes()));
		assert_ne!(id_of(source), id_of("one two\nthree"));
	}

	#[test]
	fn a_fence_holds_its_blank_lines_together() {
		let segments =
			split("intro\n\n```rust\nfn a() {}\n\nfn b() {}\n```\n\noutro").expect("segments");
		assert_eq!(segments.len(), 3);
		assert_eq!(segments[1].kind, Kind::Code);
		assert!(segments[1].source.contains("fn b()"));
	}

	#[test]
	fn code_is_never_sent_anywhere() {
		assert!(!Kind::Code.translatable());
		let kept = translatable("prose\n\n```\nlet x = 1;\n```").expect("segments");
		assert_eq!(kept.len(), 1);
	}

	#[test]
	fn directives_are_never_sent_anywhere() {
		assert!(!Kind::Directive.translatable());
		let kept = translatable("prose\n\n::image{src=\"asset.avif\"}").expect("segments");
		assert_eq!(kept.len(), 1);
	}

	#[test]
	fn inline_code_leaves_a_marker_and_comes_back() {
		let masked = mask("set `opt-level` in `Cargo.toml` now");
		assert_eq!(masked.slots.len(), 2);
		assert!(!masked.text.contains('`'));

		// Word order changes in translation; the markers move with it and still restore.
		let translated = format!("{} を {} に設定", marker(1), marker(0));
		assert!(masked.intact(&translated));
		assert_eq!(
			masked.restore(&translated),
			"`Cargo.toml` を `opt-level` に設定"
		);
	}

	#[test]
	fn a_dropped_marker_is_caught_rather_than_shipped() {
		// A model that swallows one is the failure this exists to detect. Silently restoring
		// what is left would put a half-translated identifier into prose.
		let masked = mask("use `serde` and `jiff`");
		assert!(!masked.intact(&format!("{} を使う", marker(0))));
		assert!(!masked.intact(&format!("{}{}{}", marker(0), marker(1), marker(1))));
	}

	#[test]
	fn structure_decides_the_model_not_a_difficulty_score() {
		assert!(Kind::Heading.is_light());
		// Prose and quotations carry register and idiom, which is what a light model loses.
		assert!(!Kind::Prose.is_light());
		assert!(!Kind::Quote.is_light());
		assert!(!Kind::Directive.is_light());
	}

	#[test]
	fn frontmatter_is_not_a_segment() {
		let article = "---\ntitle: A\nlang: zh\n---\n\nbody text";
		let segments = split(article).expect("segments");
		assert_eq!(segments.len(), 2);
		assert_eq!(segments[0].source, "A");
		assert_eq!(segments[0].region, Region::Frontmatter);
		assert!(
			!segments
				.iter()
				.any(|segment| segment.source.contains("title:"))
		);
		assert_eq!(&article[segments[0].start..segments[0].end], " A");
		assert_eq!(segments[1].source, "body text");
		assert_eq!(segments[1].region, Region::Body);
		assert_eq!(&article[segments[1].start..segments[1].end], "body text");
	}

	#[test]
	fn editing_a_title_invalidates_only_that_title() {
		let before = split(
			"---\ntitle: Before\nsubtitle: Same subtitle\ndescription: Same description\nlang: zh\n---\n\nSame body",
		).expect("segments");
		let after = split(
			"---\ntitle: After\nsubtitle: Same subtitle\ndescription: Same description\nlang: zh\n---\n\nSame body",
		).expect("segments");
		assert_eq!(before.len(), 4);
		assert_eq!(after.len(), 4);
		assert_ne!(before[0].id, after[0].id);
		for index in 1..before.len() {
			assert_eq!(before[index].id, after[index].id);
		}
	}

	#[test]
	fn a_non_allowlisted_frontmatter_key_is_never_translatable() {
		let live = translatable(
			"---\ntitle: Visible\nlang: zh\ncreated: 2026-08-02\nviews: 5\nfuture: Never send me\n---\n\nBody",
		).expect("segments");
		let sources = live
			.values()
			.map(|segment| segment.source.as_str())
			.collect::<Vec<_>>();
		assert_eq!(sources.len(), 2);
		assert!(sources.contains(&"Visible"));
		assert!(sources.contains(&"Body"));
		assert!(!sources.contains(&"zh"));
		assert!(!sources.contains(&"Never send me"));
	}

	#[test]
	fn folded_frontmatter_is_one_semantic_segment_with_one_lexical_span() {
		let article = "---\ndescription:\n  first line\n  second line\nlang: zh\n---\n\nBody";
		let segments = split(article).expect("segments");
		assert_eq!(segments[0].source, "first line second line");
		assert_eq!(
			&article[segments[0].start..segments[0].end],
			"\n  first line\n  second line"
		);
	}
}
