//! The `cms summary` command: telling a reader what an article is about without telling them
//! what it concludes.
//!
//! Everything it produces lands in a generated sidecar beside the article. The `.md` is prose a
//! person wrote and no command edits it -- which is also why the homepage, which carries a
//! hand-written `summary` in its frontmatter and is never translated, is not an article here.
//!
//! Not the description: that one is sized for a search result and reads as a label, while this
//! is read by somebody standing in front of the article deciding whether to spend twenty minutes
//! on it. See spec/i18n.md.

use crate::document::Fields;
use crate::i18n::runner::{self, Refusal, Runner};
use crate::i18n::segment::Kind;
use crate::i18n::store::Translation;
use crate::task::{Record, claim, progress, registry, writer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many summaries are in flight at once. Same reasoning as `alt`: politeness, not resources.
pub const PARALLEL: usize = 4;

pub const VERSION: u32 = 1;

/// An article's summary, per locale, with what each one cost.
///
/// The same `Translation` shape the translations use, for the same reason: a value produced by a
/// model is worth nothing without a record of which model, when, and at what price. `review` is
/// the one field a machine never sets.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sidecar {
	#[serde(default = "default_version")]
	pub version: u32,
	/// Locale to summary. The article's own language is written here first; the rest are
	/// translated from it.
	#[serde(default)]
	pub summary: BTreeMap<String, Translation>,
}

fn default_version() -> u32 {
	VERSION
}

/// Where an article's summaries live, beside its translations.
pub fn sidecar_for(article: &Path) -> PathBuf {
	article.with_extension("summary.yaml")
}

/// An article's summary sidecar, empty when it has none yet.
///
/// A parse failure is an error rather than an empty sidecar, for the reason the translation
/// sidecar has the same rule: the summaries in it were paid for and every save rewrites the whole
/// file, so reading a broken one as empty erases what it could not read.
pub fn load(path: &Path) -> std::io::Result<Sidecar> {
	let text = match std::fs::read_to_string(path) {
		Ok(text) => text,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Sidecar::default()),
		Err(error) => return Err(error),
	};
	serde_yaml_ng::from_str(&text)
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))
}

/// The public locale an article's `lang` frontmatter names.
///
/// Frontmatter writes the short form (`zh`, `en`); the sidecar is keyed the way every other
/// locale-addressed record here is keyed. Traditional Chinese is matched by script before the
/// language falls through, which is the same rule the site applies.
pub fn source_locale(lang: &str) -> Option<&'static str> {
	let lower = lang.to_ascii_lowercase();
	let mut parts = lower.split('-');
	let primary = parts.next()?;
	let rest: Vec<&str> = parts.collect();
	Some(match primary {
		"zh" => {
			if rest
				.iter()
				.any(|part| *part == "hant" || matches!(*part, "tw" | "hk" | "mo"))
			{
				"zh-TW"
			} else {
				"zh-CN"
			}
		}
		"en" => "en-US",
		"ja" => "ja-JP",
		"ko" => "ko-KR",
		"de" => "de-DE",
		"fr" => "fr-FR",
		"es" => "es-ES",
		_ => return None,
	})
}

fn prompt(body: &str, lang: &str) -> String {
	format!(
		"Below is an article, written in `{lang}`. Write a summary of it, in that same \
		 language.\n\n\
		 Who it is for: somebody who has found this article and is deciding whether to read it. \
		 They want to know what ground it covers and whether it is worth their time. They have \
		 not read it yet.\n\n\
		 Length: at most four sentences, and about 150 characters in Chinese, Japanese or \
		 Korean, or about 70 words otherwise. Longer and more specific than the one-line \
		 `description` already in the frontmatter, far shorter than the article. Do not walk \
		 through it section by section.\n\n\
		 Be concrete about the question, and only about the question. Name the problem it \
		 starts from, the alternatives it weighs and rejects, and the constraints it works \
		 under. Use real names for those -- the tools, the flags, the prior art. That is what \
		 tells a reader whether this article is about their problem.\n\n\
		 Withhold the article's own answer. If it proposes a design, do not name that design or \
		 explain how it works. If it recommends settings, do not give the values. If it \
		 measures something, do not give the numbers that decide the question. You may say that \
		 it arrives at a design, a recommendation or a measurement, and characterise it in a \
		 word -- surprising, modest, expensive, narrower than expected -- but its content stays \
		 in the article.\n\n\
		 Do not bolt a teaser onto the end. A closing line like \"and reaches a surprising \
		 conclusion\" after a summary that already gave everything away is the precise failure \
		 this is guarding against; the restraint has to happen throughout, by not writing those \
		 sentences at all. The other failure is saying nothing: \"covers several approaches and \
		 draws conclusions\" withholds the question too, and is worthless. Everything about \
		 what is asked, almost nothing about what is found.\n\n\
		 Write about the article, not about its author. Never give the author a pronoun: a \
		 personal essay is written in the first person and says nothing about how its writer \
		 should be referred to, so \"the author decides\" or \"the article settles on\" is \
		 right and \"he decides\" or \"she tries\" is inventing a fact about a real person.\n\n\
		 Write flowing prose in the article's own register. No heading, no list, no \"This \
		 article...\" opener, no closing invitation to read on. Reply with the summary alone: \
		 no preamble, no quotes, and no markdown whatsoever -- no asterisks, no backticks, no \
		 headings.\n\n\
		 --- article begins ---\n{body}\n--- article ends ---"
	)
}

fn plain(text: &str) -> String {
	let mut out = String::with_capacity(text.len());
	let mut chars = text.chars().peekable();
	let mut at_line_start = true;

	while let Some(ch) = chars.next() {
		match ch {
			// A heading is a label the summary was told not to write, so the whole line goes
			// rather than just its hashes.
			'#' if at_line_start => {
				for skipped in chars.by_ref() {
					if skipped == '\n' {
						break;
					}
				}
			}
			// A bullet is a marker on content that is kept.
			'-' | '+' | '>' if at_line_start => {
				while chars.peek().is_some_and(|next| *next == ch || *next == ' ') {
					chars.next();
				}
			}
			'*' | '_' | '`' => {
				// Emphasis and code fences are markers, never content, in a one-paragraph
				// summary. A doubled marker is consumed with its partner.
				while chars.peek() == Some(&ch) {
					chars.next();
				}
			}
			'[' => at_line_start = false,
			']' => {
				// `[text](url)` keeps the text and drops the target.
				if chars.peek() == Some(&'(') {
					for skipped in chars.by_ref() {
						if skipped == ')' {
							break;
						}
					}
				}
				at_line_start = false;
			}
			'\n' | '\r' | '\t' | ' ' => {
				let mut broke_line = matches!(ch, '\n' | '\r');
				while let Some(next) = chars.peek().copied() {
					if !matches!(next, '\n' | '\r' | '\t' | ' ') {
						break;
					}
					broke_line |= matches!(next, '\n' | '\r');
					chars.next();
				}
				let before = out.chars().last();
				let after = chars.peek().copied();
				let cjk = |c: Option<char>| c.is_some_and(is_cjk);
				if !out.is_empty() && after.is_some() && !(cjk(before) && cjk(after)) {
					out.push(' ');
				}
				// A bullet on the next line is still at a line start; the newline it followed
				// has already been folded away.
				at_line_start = broke_line;
			}
			// Punctuation that follows a CJK character belongs to that script. Models mix the
			// two widths within one sentence, and a half-width comma between Chinese clauses
			// reads as a typo rather than as a style. Judged by the preceding character alone,
			// so `Next.js` and `19.2` keep their ASCII dots.
			',' | '.' | ':' | ';' | '?' | '!' if out.chars().last().is_some_and(is_cjk) => {
				out.push(match ch {
					',' => '，',
					'.' => '。',
					':' => '：',
					';' => '；',
					'?' => '？',
					_ => '！',
				});
				at_line_start = false;
			}
			other => {
				out.push(other);
				at_line_start = false;
			}
		}
	}
	out.trim().to_owned()
}

/// Scripts that are written without spaces between words.
fn is_cjk(ch: char) -> bool {
	matches!(ch as u32,
		0x3000..=0x303F | 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF
		| 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

#[derive(Debug, Default)]
pub struct Outcome {
	pub spent: crate::alt::Spend,
	pub written: usize,
	/// Articles that already had a summary in their own language.
	pub skipped: usize,
	/// Articles whose summary a person vouched for. Never regenerated, `--force` included.
	pub reviewed: usize,
	/// Articles still owed one, held back by `--limit`.
	pub deferred: usize,
	pub failed: Vec<(String, String)>,
	/// Articles another run holds a claim on, left to it rather than summarised twice.
	pub claimed_elsewhere: usize,
}

/// An article the command can act on.
struct Article {
	path: PathBuf,
	/// The locale the summary is written in, from the article's `lang`.
	locale: &'static str,
	lang: String,
}

/// The prose, without the frontmatter block the model has no use for.
fn body_of(source: &str) -> &str {
	match crate::document::split(source) {
		Ok(document) => document.prose(),
		// A file whose fence never closes has no readable body either; handing back the whole
		// text is what every caller here already did, and the malformed reading is reported by
		// whoever asks for the fields rather than twice.
		Err(_) => source,
	}
}

/// A string field from the frontmatter block, if the file opens with one.
///
/// One parser rather than one per field: the second caller is where a copy starts disagreeing
/// with the first about what counts as frontmatter, and the answer to that question decides
/// whether a file is an article at all.
/// The `lang` an article declares, if it declares one.
///
/// The presence of this key is what separates an article from a page: a page has no language to
/// translate out of, so it is neither summarised nor translated. See spec/i18n.md.
pub fn lang_of(fields: &Fields) -> Option<&str> {
	fields.get("lang").map(String::as_str)
}

/// The `title` an article declares, if it declares one.
pub fn title_of(fields: &Fields) -> Option<&str> {
	fields.get("title").map(String::as_str)
}

/// The `subtitle` an article declares, if it declares one.
pub fn subtitle_of(fields: &Fields) -> Option<&str> {
	fields.get("subtitle").map(String::as_str)
}

/// The best authored timestamp for ordering an article by its latest change.
///
/// `lastmod` when the author set one, and the creation date otherwise. The rule lives here rather
/// than at each call site so the two keys cannot be consulted in different orders in two places.
pub fn modified_of(fields: &Fields) -> Option<&str> {
	fields
		.get("lastmod")
		.or_else(|| fields.get("created"))
		.map(String::as_str)
}

/// Which articles still want a summary in their own language.
///
/// A page without `lang` is not an article -- the homepage is the standing example, and its
/// hand-written summary is neither generated nor translated.
fn pending(contents: &Path, force: bool) -> std::io::Result<(Vec<Article>, usize, usize)> {
	let mut found = Vec::new();
	let mut skipped = 0;
	let mut reviewed = 0;
	let mut stack = vec![contents.to_path_buf()];
	while let Some(dir) = stack.pop() {
		let Ok(entries) = std::fs::read_dir(&dir) else {
			continue;
		};
		for entry in entries.flatten() {
			let path = entry.path();
			if path.is_dir() {
				stack.push(path);
				continue;
			}
			if path.extension().is_none_or(|ext| ext != "md") {
				continue;
			}
			let Ok(source) = std::fs::read_to_string(&path) else {
				continue;
			};
			let fields = crate::document::fields_of(&source, &path)?;
			let Some(lang) = lang_of(&fields) else {
				continue;
			};
			let lang = lang.to_owned();
			let Some(locale) = source_locale(&lang) else {
				continue;
			};

			// A summary somebody has read and vouched for is not the machine's to replace, and
			// `--force` does not change that: the flag means "the model's last answer was
			// wrong", not "discard a person's judgement".
			if let Some(existing) = load(&sidecar_for(&path))?.summary.get(locale) {
				if existing.review {
					reviewed += 1;
					continue;
				}
				if !force {
					skipped += 1;
					continue;
				}
			}
			found.push(Article { path, locale, lang });
		}
	}
	found.sort_by(|a, b| a.path.cmp(&b.path));
	Ok((found, skipped, reviewed))
}

/// The generated summary, with everything needed to say where it came from.
struct Generated {
	locale: &'static str,
	spend: crate::alt::Spend,
	entry: Translation,
}

async fn summarise(
	runner: Runner,
	model_override: Option<String>,
	article: &Article,
) -> Result<Generated, Refusal> {
	let source =
		std::fs::read_to_string(&article.path).map_err(|error| Refusal::Failed(error.to_string()))?;
	let model = model_override
		.as_deref()
		.unwrap_or_else(|| runner.model_for(Kind::Prose, 0));

	// Stamped before the request rather than after it, so `at` says when the article was read
	// and not when the queue happened to drain.
	let at = crate::image::manifest::now();
	let started = std::time::Instant::now();
	let answer = runner::ask(runner, &prompt(body_of(&source), &article.lang), model).await?;
	let seconds = started.elapsed().as_secs_f64();

	let text = plain(&answer.text);
	if text.is_empty() {
		return Err(Refusal::Failed("empty summary".to_owned()));
	}
	Ok(Generated {
		locale: article.locale,
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
	let Options {
		repository,
		runner,
		model_override,
		force,
		limit,
		shell,
		sink,
	} = options;
	let contents = repository.join("contents");
	let (mut todo, skipped, reviewed) = pending(&contents, force)?;
	let wanted = todo.len();
	if let Some(limit) = limit {
		todo.truncate(limit);
	}
	let mut outcome = Outcome {
		skipped,
		reviewed,
		deferred: wanted - todo.len(),
		..Outcome::default()
	};

	let progress = crate::task::start(repository, "summary", shell, todo.len() as u64, sink)?;
	let writer = writer::Writer::start(repository, Record::Summaries)?;

	let mut queue = todo.into_iter();
	let mut running = Vec::new();

	// The claim on each article in flight, released once its summary is on disk. Keyed by the
	// path below `contents`, so two checkouts of the same repository do not collide and two runs
	// over one do. See spec/tasks.md.
	let mut held: std::collections::HashMap<PathBuf, claim::Claim> = std::collections::HashMap::new();

	loop {
		while running.len() < PARALLEL {
			let Some(article) = queue.next() else {
				break;
			};
			let key = article
				.path
				.strip_prefix(&contents)
				.unwrap_or(&article.path)
				.to_path_buf();
			// Claimed before anything is spent: an article another run is summarising right now
			// is left to it rather than paid for twice.
			match claim::take(repository, "summary", &key.display().to_string()) {
				Ok(claim) => {
					held.insert(article.path.clone(), claim);
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
				let result = summarise(runner, model_override, &article).await;
				(article.path, result)
			}));
		}
		if running.is_empty() {
			break;
		}
		let finished = running.remove(0);
		let (path, result) = match finished.await {
			Ok(result) => result,
			Err(error) => (PathBuf::new(), Err(Refusal::Failed(error.to_string()))),
		};

		let name = path.display().to_string();
		match result {
			Ok(generated) => {
				outcome.spent.add(generated.spend);
				let model = generated.entry.model.clone();
				let seconds = generated.entry.seconds;
				let sidecar_path = sidecar_for(&path);
				// Written as it arrives rather than gathered and saved at the end: each of these
				// was paid for, and one interrupt used to discard the whole run. Re-read inside
				// the writer so a translation another process added since is not overwritten.
				let locale = generated.locale.to_owned();
				let entry = generated.entry;
				let applied = writer.apply(move || {
					let mut sidecar = load(&sidecar_path)?;
					sidecar.version = VERSION;
					// Only the source locale is touched. Translations of a previous summary are
					// left for the translating pass to notice and replace, rather than dropped
					// here where nothing would report the gap.
					sidecar.summary.insert(locale, entry);
					let text = serde_yaml_ng::to_string(&sidecar).map_err(std::io::Error::other)?;
					std::fs::write(&sidecar_path, text)
				});
				match applied {
					Ok(()) => {
						outcome.written += 1;
						progress.suspend(&mut || println!("  {name}  [{model}, {seconds:.1}s]"));
					}
					Err(error) => outcome.failed.push((name.clone(), error.to_string())),
				}
			}
			Err(error) => outcome.failed.push((name, error.to_string())),
		}

		held.remove(&path);
		progress.inc(1);
	}
	progress.finish_and_clear();
	Ok(outcome)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// An article another run holds a claim on is left to it rather than summarised twice.
	///
	/// Every candidate is claimed up front, so nothing is spawned and no runner is reached --
	/// which is what makes the property testable without one.
	#[tokio::test]
	async fn an_article_another_run_claimed_is_not_summarised_again() {
		let root = std::env::temp_dir().join(format!("cms-summary-claimed-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&root);
		let contents = root.join("contents");
		std::fs::create_dir_all(&contents).expect("contents");
		std::fs::write(
			contents.join("a.md"),
			"---\nlang: en\ntitle: A\n---\n\nSome body text that wants a summary.\n",
		)
		.expect("article");

		let held = claim::take(&root, "summary", "a.md").expect("claim");
		let outcome = run(Options {
			repository: &root,
			runner: Runner::Claude,
			model_override: None,
			force: false,
			limit: None,
			shell: registry::Shell::Cli,
			sink: Box::new(progress::Silent),
		})
		.await
		.expect("run");
		drop(held);

		assert_eq!(outcome.claimed_elsewhere, 1);
		assert_eq!(outcome.written, 0);
		assert!(outcome.failed.is_empty());
		let _ = std::fs::remove_dir_all(&root);
	}

	/// A broken sidecar stops the walk rather than reading as a missing summary, which would
	/// make the article a candidate and buy the summary it already has a second time.
	#[test]
	fn a_broken_sidecar_is_an_error_rather_than_an_absent_summary() {
		let path =
			std::env::temp_dir().join(format!("cms-summary-{}.summary.yaml", std::process::id()));
		std::fs::write(&path, "summary: [not a map\n").expect("write");
		let error = load(&path).expect_err("a broken sidecar must not read as empty");
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
		let _ = std::fs::remove_file(&path);
	}

	#[test]
	fn the_prompt_asks_for_the_question_and_withholds_the_answer() {
		let text = prompt("body text here", "zh");
		assert!(text.contains("body text here"));
		assert!(text.contains("`zh`"));
		// Both halves have to survive editing. Losing the second turns this into `description`
		// at greater length; losing the first turns it into a category label.
		assert!(text.contains("its content stays"));
		assert!(text.contains("Do not bolt a teaser"));
		// A model that picks a pronoun for a first-person essay has invented a fact about a
		// real person, which is worse than any wording problem here.
		assert!(text.contains("Never give the author a pronoun"));
	}

	#[test]
	fn markdown_never_reaches_the_sidecar() {
		assert_eq!(plain("a **bold** and `code` word"), "a bold and code word");
		assert_eq!(
			plain("see [the docs](https://x.test/a) now"),
			"see the docs now"
		);
		assert_eq!(plain("## Heading\n\nBody"), "Body");
		assert_eq!(plain("- one\n- two"), "one two");
		assert_eq!(plain("_em_ and __strong__"), "em and strong");
	}

	#[test]
	fn a_wrapped_cjk_line_joins_without_a_space() {
		assert_eq!(plain("这篇文章\n讲编译"), "这篇文章讲编译");
		assert_eq!(plain("ends here\nand continues"), "ends here and continues");
		assert_eq!(plain("中文\nEnglish"), "中文 English");
	}

	#[test]
	fn punctuation_takes_the_width_of_the_script_before_it() {
		assert_eq!(plain("框架里,一个字段"), "框架里，一个字段");
		// The space goes too: full-width punctuation carries its own trailing space.
		assert_eq!(plain("说起. 然后"), "说起。然后");
		// English keeps its own punctuation, and so do version numbers and package names
		// sitting inside Chinese prose.
		assert_eq!(plain("uses Next.js here"), "uses Next.js here");
		assert_eq!(plain("React 19.2 的 PPR"), "React 19.2 的 PPR");
		assert_eq!(plain("ends here."), "ends here.");
	}

	#[test]
	fn the_frontmatter_is_not_sent() {
		let source = "---\ntitle: A\nlang: zh\n---\n\nThe prose.\n";
		assert_eq!(body_of(source), "The prose.\n");
		let fields = crate::document::fields(source).expect("fields");
		assert_eq!(lang_of(&fields), Some("zh"));
	}

	#[test]
	fn an_article_locale_follows_the_script_rather_than_the_region() {
		assert_eq!(source_locale("zh"), Some("zh-CN"));
		assert_eq!(source_locale("zh-Hant"), Some("zh-TW"));
		assert_eq!(source_locale("zh-HK"), Some("zh-TW"));
		assert_eq!(source_locale("en-US"), Some("en-US"));
		// The eight are what may be read, not a promise about what may be written.
		assert_eq!(source_locale("it"), None);
	}

	#[test]
	fn a_page_without_lang_is_not_an_article() {
		let dir = std::env::temp_dir().join("cms-summary-pending");
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		// The homepage shape: a hand-written summary and no language of its own.
		std::fs::write(
			dir.join("homepage.md"),
			"---\ntitle: A\nsummary: Written by hand.\n---\n\nBody\n",
		)
		.unwrap();
		std::fs::write(
			dir.join("post.md"),
			"---\ntitle: B\nlang: zh\n---\n\nBody\n",
		)
		.unwrap();
		let (todo, _, _) = pending(&dir, false).expect("pending");
		assert_eq!(todo.len(), 1);
		assert!(todo[0].path.ends_with("post.md"));
		let _ = std::fs::remove_dir_all(&dir);
	}

	#[test]
	fn a_reviewed_summary_survives_force() {
		let dir = std::env::temp_dir().join("cms-summary-reviewed");
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		let article = dir.join("post.md");
		std::fs::write(&article, "---\nlang: zh\n---\n\nBody\n").unwrap();
		let mut sidecar = Sidecar::default();
		sidecar.summary.insert(
			"zh-CN".to_owned(),
			Translation {
				text: "Vouched for.".to_owned(),
				provider: "openai".to_owned(),
				model: "m".to_owned(),
				at: "2026-08-03T00:00:00Z".to_owned(),
				seconds: 1.0,
				tokens: 1,
				review: true,
			},
		);
		std::fs::write(
			sidecar_for(&article),
			serde_yaml_ng::to_string(&sidecar).unwrap(),
		)
		.unwrap();

		let (todo, _, reviewed) = pending(&dir, true).expect("pending");
		assert!(todo.is_empty());
		assert_eq!(reviewed, 1);
		let _ = std::fs::remove_dir_all(&dir);
	}
}
