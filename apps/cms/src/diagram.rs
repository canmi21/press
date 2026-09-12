//! The `cms diagram` command: describing a drawing that is written as text.
//!
//! An `svg-canvas` or `mermaid` fence is a picture the corpus stores as source. Nothing else in
//! the pipeline can read it: the block is `Kind::Code`, so it is never translated and never
//! reaches a reader who cannot see it, never reaches the search index, and reaches the
//! translator of the paragraph beside it as the words "a code block, not shown".
//!
//! So it is described, the way an image is. The difference is where the description comes from:
//! an image is described from its pixels and needs a model that can see, while a diagram is
//! described from the source that draws it, which is text. That makes this closer to `summary`
//! than to `alt`, and it is why the two do not share a command.
//!
//! The description belongs to the drawing rather than to any article carrying it, so it is keyed
//! by the hash of the block -- the segment id, which is already what addresses that block
//! everywhere else -- and one drawing used twice is described once. See spec/i18n.md.

use crate::i18n::runner::{self, Refusal, Runner};
use crate::i18n::segment::{self, Kind as SegmentKind};
use crate::i18n::store::Translation;
use crate::task::{Record, claim, progress, registry, writer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many descriptions are in flight at once. Same reasoning as `alt` and `summary`:
/// politeness and rate limits, not local resources.
pub const PARALLEL: usize = 4;

pub const VERSION: u32 = 1;

/// The locale a generated description is written in.
///
/// English, as with an image description, and for a reason this case makes plainer: the labels
/// inside these drawings are English even in the Chinese articles, so a description written in
/// any other language would be translating them on the way out. `cms locale` fills the rest.
pub const SOURCE_LOCALE: &str = "en-US";

/// Which fence languages draw a picture.
///
/// Both are diagrams whose source happens to be text, and both are invisible to everything
/// downstream for the same reason. A fence language not in this list is code, and code is not
/// described.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
	SvgCanvas,
	Mermaid,
}

impl Kind {
	fn parse(language: &str) -> Option<Self> {
		match language {
			"svg-canvas" => Some(Self::SvgCanvas),
			"mermaid" => Some(Self::Mermaid),
			_ => None,
		}
	}

	/// What the drawing is called when the model is told what it is about to read.
	fn subject(self) -> &'static str {
		match self {
			Self::SvgCanvas => "hand-written SVG",
			Self::Mermaid => "Mermaid diagram source",
		}
	}
}

/// One drawing's record: what kind it is, and what it says, per locale.
///
/// The same `Translation` shape every other derived value here uses. A value produced by a model
/// is worth nothing without a record of which model, when, and at what price, and `review` is the
/// one field a machine never sets.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
	pub kind: Option<Kind>,
	/// FNV-1a over the drawing's source, which is how the site finds this record.
	///
	/// The key is a BLAKE3 content id, and the site cannot compute one: reimplementing the real
	/// hash in TypeScript is the duplication the segment layout exists to avoid, and the same
	/// rule holds here. So the record carries the cheap checksum the site already computes for
	/// the segment layout, over the fence's payload rather than the whole block -- retitling a
	/// fence does not change the drawing.
	#[serde(default)]
	pub fingerprint: String,
	#[serde(default)]
	pub description: BTreeMap<String, Translation>,
}

/// Every described drawing, keyed by the hash of the block that draws it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Store {
	#[serde(default = "default_version")]
	pub version: u32,
	#[serde(default)]
	pub diagrams: BTreeMap<String, Entry>,
}

fn default_version() -> u32 {
	VERSION
}

/// Where the descriptions live.
pub fn store_path(repository: &Path) -> PathBuf {
	repository.join("data").join("diagram.json")
}

/// The store, empty when the repository has none yet.
///
/// A parse failure is an error rather than an empty store, for the reason `media` has the same
/// rule: every description in it was paid for and every save rewrites the whole file, so reading
/// a broken one as empty erases what it could not read.
pub fn load(path: &Path) -> std::io::Result<Store> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Store::default()),
		Err(error) => return Err(error),
	};
	serde_json::from_str(&text)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

pub fn save(path: &Path, store: &Store) -> std::io::Result<()> {
	// Tabs, because `.editorconfig` says tabs and this file is read by people reviewing what a
	// model wrote. `to_string_pretty` would indent with two spaces.
	let mut out = Vec::new();
	let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
	let mut serialiser = serde_json::Serializer::with_formatter(&mut out, formatter);
	store.serialize(&mut serialiser).map_err(std::io::Error::other)?;
	out.push(b'\n');
	crate::image::store::write(path, &out)
}

/// The fence language a block opens with, if it is a fenced block at all.
///
/// Read off the source rather than carried on the segment: the segment's kind says "code" and
/// stops there, which is all translation needs to know and less than this needs.
fn language_of(source: &str) -> Option<&str> {
	let first = source.lines().next()?;
	let rest = first.strip_prefix("```").or_else(|| first.strip_prefix("~~~"))?;
	let language = rest.split_whitespace().next()?;
	(!language.is_empty()).then_some(language)
}

/// A drawing the command can act on.
pub struct Drawing {
	pub id: String,
	pub kind: Kind,
	/// The whole fence, which is what the id hashes and what the model is shown.
	pub source: String,
	/// What the fence draws, without the two fence lines around it. What the site sees.
	pub payload: String,
	/// Where it was first found, for a report. The description belongs to the drawing, not here.
	pub article: PathBuf,
}

impl Drawing {
	pub fn fingerprint(&self) -> String {
		crate::i18n::layout::fingerprint(self.payload.as_bytes())
	}
}

/// A fence without its two fence lines.
///
/// What remark hands the site as a code node's `value`, which is the string the site fingerprints
/// to find this drawing's description. The two have to agree exactly, so this is the narrowest
/// possible reading: drop the first line and the last, join the rest.
fn payload_of(source: &str) -> String {
	let mut lines: Vec<&str> = source.lines().collect();
	if lines.len() < 2 {
		return String::new();
	}
	lines.remove(0);
	lines.pop();
	lines.join("\n")
}

/// Every distinct drawing in the corpus, in the order they would be read.
///
/// Distinct by id, so a drawing repeated across articles appears once. The first article to
/// carry it is the one named, which is arbitrary and only ever used to print a line.
pub fn collect(contents: &Path) -> std::io::Result<Vec<Drawing>> {
	let mut articles = Vec::new();
	let mut stack = vec![contents.to_path_buf()];
	while let Some(dir) = stack.pop() {
		let Ok(entries) = std::fs::read_dir(&dir) else {
			continue;
		};
		for entry in entries.flatten() {
			let path = entry.path();
			if path.is_dir() {
				stack.push(path);
			} else if path.extension().is_some_and(|ext| ext == "md") {
				articles.push(path);
			}
		}
	}
	articles.sort();

	let mut found = Vec::new();
	let mut seen = std::collections::HashSet::new();
	for article in articles {
		let Ok(source) = std::fs::read_to_string(&article) else {
			continue;
		};
		let Ok(segments) = segment::split(&source) else {
			continue;
		};
		for segment in segments {
			if segment.kind != SegmentKind::Code {
				continue;
			}
			let Some(kind) = language_of(&segment.source).and_then(Kind::parse) else {
				continue;
			};
			if !seen.insert(segment.id.clone()) {
				continue;
			}
			let payload = payload_of(&segment.source);
			found.push(Drawing {
				id: segment.id,
				kind,
				payload,
				source: segment.source,
				article: article.clone(),
			});
		}
	}
	Ok(found)
}

/// What the model is asked for.
///
/// The framing is the whole instruction, and it is the one `alt` arrived at: "describe this"
/// produces a caption naming the subject, while asking for what a person who cannot see it would
/// need produces what is actually useful. What differs here is that the source is the drawing --
/// the model is reading coordinates and labels, not looking at a picture -- so it is told to
/// report what the drawing says rather than what the markup contains.
struct Asked {
	request: crate::i18n::prompt::Request,
	/// The fence around the source, kept because the answer sometimes ends with it. See `describe`.
	source_boundary: String,
}

fn prompt(kind: Kind, source: &str) -> Asked {
	let source_boundary = crate::i18n::prompt::boundary();
	let output_boundary = crate::i18n::prompt::boundary();
	let subject = kind.subject();
	let text = format!(
		"Below is the source of a diagram from an article, written as {subject}. Describe the \
		 diagram it draws for someone who cannot see it.\n\n\
		 Read it as a picture, not as markup. Give the shape of it first -- a pipeline, a \
		 comparison of two paths, a tree, a matrix -- because that frames everything after it. \
		 Then the content: every label as it is written, what connects to what and in which \
		 direction, and any grouping or division the drawing makes. Where two rows or branches \
		 are evidently being contrasted, say what the contrast is. Say nothing about SVG \
		 elements, coordinates, colours or classes: those draw the picture and are not in it.\n\n\
		 Two to four sentences, in English, as flowing prose rather than a list. Do not open \
		 with \"A diagram of\" or \"This diagram shows\" -- start with the content.\n\n\
		 This is a single-turn text transformation: everything needed is below. Do not inspect \
		 files, repository rules or version control, and do not describe how you will work.\n\n\
		 Output exactly two copies of the output boundary with the description between them. \
		 Write nothing inside those boundaries except the description: no preamble, quotes, or \
		 markdown whatsoever.\n\n\
		 Output boundary:\n{output_boundary}\n\n\
		 {source_boundary}\n{source}\n{source_boundary}"
	);
	Asked {
		request: crate::i18n::prompt::Request { text, boundary: output_boundary },
		source_boundary,
	}
}

#[derive(Debug, Default)]
pub struct Outcome {
	pub spent: crate::alt::Spend,
	pub written: usize,
	/// Drawings that already carried a description in the source locale.
	pub skipped: usize,
	/// Drawings whose description a person vouched for. Never regenerated, `--force` included.
	pub reviewed: usize,
	/// Drawings still owed one, held back by `--limit`.
	pub deferred: usize,
	pub failed: Vec<(String, String)>,
	/// Drawings another run holds a claim on, left to it rather than described twice.
	pub claimed_elsewhere: usize,
}

/// Which drawings still want a description in the source locale.
fn pending(drawings: Vec<Drawing>, store: &Store, force: bool) -> (Vec<Drawing>, usize, usize) {
	let mut found = Vec::new();
	let mut skipped = 0;
	let mut reviewed = 0;
	for drawing in drawings {
		// A description somebody has read and vouched for is not the machine's to replace, and
		// `--force` does not change that: the flag means "the model's last answer was wrong",
		// not "discard a person's judgement".
		if let Some(existing) =
			store.diagrams.get(&drawing.id).and_then(|e| e.description.get(SOURCE_LOCALE))
		{
			if existing.review {
				reviewed += 1;
				continue;
			}
			if !force {
				skipped += 1;
				continue;
			}
		}
		found.push(drawing);
	}
	(found, skipped, reviewed)
}

/// The generated description, with everything needed to say where it came from.
struct Generated {
	spend: crate::alt::Spend,
	entry: Translation,
}

/// The description between the boundaries, taking either one as the closing mark.
///
/// `prompt::bounded_reply` wants the output boundary on both sides, and is right to for a reply
/// that is prose about prose. This asks for prose about a fenced source, and a model that has
/// just read a fence closes with the fence it read: every reply measured here opened with the
/// output boundary and ended with the source one, or slipped the source one in before it.
///
/// The opening mark is the one that matters, and it is still required. It is what says the text
/// after it is the model's answer rather than something the source persuaded it to write. Which
/// mark ends the answer says nothing about where the answer came from, so the first of either
/// closes it.
fn described(reply: &str, output: &str, source: &str) -> Option<String> {
	let after = reply.split_once(output)?.1;
	let end =
		[after.find(output), after.find(source)].into_iter().flatten().min().unwrap_or(after.len());
	let text = after[..end].trim();
	(!text.is_empty()).then(|| text.to_owned())
}

async fn describe(
	runner: Runner,
	model_override: Option<String>,
	drawing: &Drawing,
) -> Result<Generated, Refusal> {
	let model = model_override.as_deref().unwrap_or_else(|| runner.model_for(SegmentKind::Prose, 0));
	// Stamped before the request rather than after it, so `at` says when the drawing was read
	// and not when the queue happened to drain.
	let at = crate::image::manifest::now();
	let started = std::time::Instant::now();
	let asked = prompt(drawing.kind, &drawing.source);
	let answer = runner::ask(runner, &asked.request.text, model).await?;
	let seconds = started.elapsed().as_secs_f64();

	let text = described(&answer.text, &asked.request.boundary, &asked.source_boundary)
		.ok_or_else(|| Refusal::Failed("the description carried no output boundary".to_owned()))?;
	Ok(Generated {
		entry: Translation {
			text,
			provider: runner.provider().to_owned(),
			model: answer.model,
			at,
			seconds,
			tokens: answer.tokens,
			review: false,
		},
		spend: crate::alt::Spend {
			// One total is all the runner reports; see the same note in `alt`.
			input: answer.tokens,
			output: 0,
			cache_read: 0,
			cache_written: 0,
			usd: answer.usd,
		},
	})
}

pub struct Options<'a> {
	pub repository: &'a Path,
	pub runner: Runner,
	pub model_override: Option<String>,
	pub force: bool,
	pub limit: Option<usize>,
	pub shell: registry::Shell,
	/// Where to report progress. The CLI passes a terminal bar; the desktop passes its own.
	pub sink: Box<dyn progress::Sink>,
}

pub async fn run(options: Options<'_>) -> std::io::Result<Outcome> {
	let Options { repository, runner, model_override, force, limit, shell, sink } = options;
	let contents = repository.join("contents");
	let path = store_path(repository);
	let drawings = collect(&contents)?;

	let writer = writer::Writer::start(repository, Record::Diagrams)?;
	// What kind of fence each drawing is, and the checksum the site finds it by, are read off the
	// source rather than bought, so they are brought up to date on every run instead of only when
	// a description is written. A drawing whose source changed is a new id and a new row; this is
	// for the rows already here, including any written before the field existed.
	let facts: Vec<(String, Kind, String)> =
		drawings.iter().map(|d| (d.id.clone(), d.kind, d.fingerprint())).collect();
	let facts_path = path.clone();
	writer.apply(move || {
		let mut store = load(&facts_path)?;
		let mut changed = store.version != VERSION;
		store.version = VERSION;
		for (id, kind, fingerprint) in facts {
			let Some(entry) = store.diagrams.get_mut(&id) else {
				continue;
			};
			changed |= entry.kind != Some(kind) || entry.fingerprint != fingerprint;
			entry.kind = Some(kind);
			entry.fingerprint = fingerprint;
		}
		if changed { save(&facts_path, &store) } else { Ok(()) }
	})?;

	let (mut todo, skipped, reviewed) = pending(drawings, &load(&path)?, force);
	let wanted = todo.len();
	if let Some(limit) = limit {
		todo.truncate(limit);
	}
	let mut outcome =
		Outcome { skipped, reviewed, deferred: wanted - todo.len(), ..Outcome::default() };

	let progress = crate::task::start(repository, "diagram", shell, todo.len() as u64, sink)?;

	let mut queue = todo.into_iter();
	let mut running = Vec::new();
	// The claim on each drawing in flight, released once its description is on disk. Keyed by the
	// drawing's id rather than by an article, because that is what is being bought. See
	// spec/tasks.md.
	let mut held: std::collections::HashMap<String, claim::Claim> = std::collections::HashMap::new();

	loop {
		while running.len() < PARALLEL {
			let Some(drawing) = queue.next() else {
				break;
			};
			match claim::take(repository, "diagram", &drawing.id) {
				Ok(claim) => {
					held.insert(drawing.id.clone(), claim);
				}
				Err(claim::Denied::Taken(_)) => {
					outcome.claimed_elsewhere += 1;
					progress.inc(1);
					continue;
				}
				Err(claim::Denied::Io(error)) => return Err(error),
			}
			let model_override = model_override.clone();
			running.push(tokio::spawn(async move {
				let result = describe(runner, model_override, &drawing).await;
				(drawing, result)
			}));
		}
		if running.is_empty() {
			break;
		}
		let finished = running.remove(0);
		let (id, name, facts, result) = match finished.await {
			Ok((drawing, result)) => {
				let name = format!("{} {}", drawing.article.display(), &drawing.id[..8]);
				let facts = (drawing.kind, drawing.fingerprint());
				(drawing.id, name, Some(facts), result)
			}
			Err(error) => (String::new(), String::new(), None, Err(Refusal::Failed(error.to_string()))),
		};

		match result {
			Ok(generated) => {
				outcome.spent.add(generated.spend);
				let model = generated.entry.model.clone();
				let seconds = generated.entry.seconds;
				let store_path = path.clone();
				let entry = generated.entry;
				let key = id.clone();
				// Written as it arrives rather than gathered and saved at the end: each of these
				// was paid for, and one interrupt would otherwise discard the whole run. Re-read
				// inside the writer so a description another process added since is not lost.
				let applied = writer.apply(move || {
					let mut store = load(&store_path)?;
					store.version = VERSION;
					let record = store.diagrams.entry(key).or_default();
					if let Some((kind, fingerprint)) = facts {
						record.kind = Some(kind);
						record.fingerprint = fingerprint;
					}
					// Only the source locale is touched. Translations of a previous description
					// are left for the translating pass to notice and replace, rather than
					// dropped here where nothing would report the gap.
					record.description.insert(SOURCE_LOCALE.to_owned(), entry);
					save(&store_path, &store)
				});
				match applied {
					Ok(()) => {
						outcome.written += 1;
						progress.suspend(&mut || println!("  {name}  [{model}, {seconds:.1}s]"));
					}
					Err(error) => outcome.failed.push((name, error.to_string())),
				}
			}
			Err(error) => outcome.failed.push((name, error.to_string())),
		}

		held.remove(&id);
		progress.inc(1);
	}
	progress.finish_and_clear();
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn only_a_fence_that_draws_something_is_a_diagram() {
		assert_eq!(
			language_of("```svg-canvas\n<svg/>\n```").and_then(Kind::parse),
			Some(Kind::SvgCanvas)
		);
		assert_eq!(language_of("```mermaid\ngraph TD\n```").and_then(Kind::parse), Some(Kind::Mermaid));
		// A fence language is not a drawing just because it is a fence.
		assert_eq!(language_of("```rust\nfn main() {}\n```").and_then(Kind::parse), None);
		assert_eq!(language_of("```\nplain\n```").and_then(Kind::parse), None);
		// The meta a fence may carry after its language is not part of the language.
		assert_eq!(
			language_of("```svg-canvas the pipeline\n<svg/>\n```").and_then(Kind::parse),
			Some(Kind::SvgCanvas)
		);
		assert_eq!(language_of("Not a fence at all").and_then(Kind::parse), None);
	}

	/// One drawing, one description, however many articles carry it.
	///
	/// The id is the hash of the block, so two articles drawing the same thing address one
	/// record -- which is the property that makes the description belong to the drawing rather
	/// than to a place it appears.
	#[test]
	fn a_drawing_carried_twice_is_collected_once() {
		let temporary = tempfile::tempdir().expect("temp");
		let contents = temporary.path().join("contents");
		std::fs::create_dir_all(&contents).expect("contents");
		let drawing = "```svg-canvas\n<svg viewBox=\"0 0 10 10\"></svg>\n```";
		for name in ["a.md", "b.md"] {
			std::fs::write(
				contents.join(name),
				format!("---\nlang: en\n---\n\nSome prose.\n\n{drawing}\n"),
			)
			.expect("article");
		}
		std::fs::write(
			contents.join("c.md"),
			"---\nlang: en\n---\n\n```mermaid\ngraph TD\nA-->B\n```\n",
		)
		.expect("article");

		let found = collect(&contents).expect("collect");
		assert_eq!(found.len(), 2, "two distinct drawings across three articles");
		assert_eq!(found.iter().filter(|d| d.kind == Kind::SvgCanvas).count(), 1);
		assert_eq!(found.iter().filter(|d| d.kind == Kind::Mermaid).count(), 1);
	}

	/// Whichever boundary the model closes with, the answer is what sits after the opening one.
	#[test]
	fn either_boundary_closes_the_description() {
		let out = "OUTPUT_MARK";
		let src = "SOURCE_MARK";
		// What the instruction asks for.
		assert_eq!(
			described("OUTPUT_MARK\nA pipeline.\nOUTPUT_MARK", out, src).as_deref(),
			Some("A pipeline.")
		);
		// What a model that has just read a fenced source actually sends.
		assert_eq!(
			described("OUTPUT_MARK\nA pipeline.\nSOURCE_MARK", out, src).as_deref(),
			Some("A pipeline.")
		);
		// A source mark slipped in ahead of the closing output mark is still the end of it.
		assert_eq!(
			described("OUTPUT_MARK\nA pipeline.\nSOURCE_MARK\ntrailing\nOUTPUT_MARK", out, src)
				.as_deref(),
			Some("A pipeline.")
		);
		// No opening mark is a refusal, not a description: nothing says where this text began.
		assert_eq!(described("A pipeline, unfenced.", out, src), None);
		assert_eq!(described("OUTPUT_MARK\n \nOUTPUT_MARK", out, src), None);
	}

	/// The site fingerprints what remark hands it, so this has to hand back exactly that.
	#[test]
	fn the_payload_is_the_fence_without_its_fence_lines() {
		assert_eq!(payload_of("```mermaid\ngraph TD\nA-->B\n```"), "graph TD\nA-->B");
		assert_eq!(payload_of("```svg-canvas\n<svg/>\n```"), "<svg/>");
		// An empty fence has a payload, and it is empty rather than missing.
		assert_eq!(payload_of("```svg-canvas\n```"), "");
		assert_eq!(payload_of("```"), "");
	}

	#[test]
	fn a_broken_store_is_an_error_rather_than_an_empty_one() {
		// Every description in it was paid for and every save rewrites the whole file, so
		// reading a broken one as empty erases what it could not read.
		let temporary = tempfile::tempdir().expect("temp");
		let path = temporary.path().join("diagram.json");
		std::fs::write(&path, "{\"diagrams\": [not a map").expect("write");
		let error = load(&path).expect_err("a broken store must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
	}

	/// A description a person vouched for is not the machine's to replace, `--force` included.
	#[test]
	fn a_reviewed_description_is_never_regenerated() {
		let temporary = tempfile::tempdir().expect("temp");
		let contents = temporary.path().join("contents");
		std::fs::create_dir_all(&contents).expect("contents");
		std::fs::write(
			contents.join("a.md"),
			"---\nlang: en\n---\n\n```mermaid\ngraph TD\nA-->B\n```\n",
		)
		.expect("article");

		let id = collect(&contents).expect("collect").remove(0).id;
		let mut store = Store::default();
		store.diagrams.entry(id).or_default().description.insert(
			SOURCE_LOCALE.to_owned(),
			Translation {
				text: "Checked by a person.".to_owned(),
				provider: "anthropic".to_owned(),
				model: "claude-opus-5".to_owned(),
				at: "2026-09-12T00:00:00Z".to_owned(),
				seconds: 1.0,
				tokens: 10,
				review: true,
			},
		);

		let drawings = collect(&contents).expect("collect");
		let (todo, skipped, reviewed) = pending(drawings, &store, true);
		assert!(todo.is_empty());
		assert_eq!(reviewed, 1);
		assert_eq!(skipped, 0);
	}
}
