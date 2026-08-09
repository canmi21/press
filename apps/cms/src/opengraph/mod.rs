//! The `cms og` command: one card per article, rendered here and published as-is.
//!
//! Named by slug rather than by content id, which is the one place this repository does not
//! content-address something. A card is not an asset an article references -- the page emits
//! `/opengraph/{slug}.png` and nothing writes that URL down -- so there is no reference to
//! rewrite and nothing to look an id up in. The cost is that the name is mutable, which is
//! why these are cached for a week rather than a year. See spec/architecture.md.
//!
//! PNG, not AVIF. Every other image here is stored AVIF, but the consumers of these are
//! crawlers for X, Slack, Discord and the rest, and they do not read it.

pub mod layout;
pub mod locale;

use crate::i18n::{segment, store};
use cosmic_text::FontSystem;
use cosmic_text::fontdb;
use layout::Card;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// The family name inside the TTF, which is what the layout asks for by name.
const FAMILY: &str = "LXGW WenKai";

/// Where the full font lives. Not the split copy under `data/public/fonts`: a subset cannot
/// answer for an arbitrary character, and a title may contain any.
const FONT: &str = "data/fonts/LXGWWenKai-Regular.ttf";

#[derive(Debug, Default)]
pub struct Outcome {
	pub rendered: usize,
	pub skipped: usize,
	pub failed: Vec<(String, String)>,
}

/// One article, reduced to what the card shows.
#[derive(Debug, PartialEq, Eq)]
pub struct Article {
	pub slug: String,
	pub title: String,
	pub subtitle: Option<String>,
	pub category: Option<String>,
	pub created: Option<String>,
}

/// Read the frontmatter fields the card needs.
///
/// `subtitle` rather than `description`: the latter is written for search results and runs to
/// a paragraph, which at this size would fill the card and crowd out the title it is meant to
/// support.
pub fn article_of(root: &Path, path: &Path) -> Option<Article> {
	let text = std::fs::read_to_string(path).ok()?;
	let front = text.strip_prefix("---\n")?.split_once("\n---")?.0;

	let mut title = None;
	let mut subtitle = None;
	let mut created = None;
	for line in front.lines() {
		let Some((key, value)) = line.split_once(':') else {
			continue;
		};
		let value = value.trim().trim_matches('"').trim_matches('\'');
		if value.is_empty() {
			continue;
		}
		match key.trim() {
			"title" => title = Some(value.to_owned()),
			"subtitle" => subtitle = Some(value.to_owned()),
			"created" => created = Some(value.to_owned()),
			_ => {}
		}
	}

	let relative = path.strip_prefix(root).ok()?.with_extension("");
	let slug = relative.to_str()?.to_owned();
	// The top directory is the category, so an article's place in the tree is the only thing
	// that has to say what it is about.
	let category = relative
		.parent()
		.and_then(|p| p.file_name())
		.and_then(|n| n.to_str())
		.map(str::to_owned);

	Some(Article {
		slug,
		title: title?,
		subtitle,
		category,
		created,
	})
}

/// `2026-04-13T19:18:28.488Z` as `Apr 13, 2026`.
///
/// Formatted here rather than shown raw because the card is read at a glance, and an ISO
/// timestamp is a thing to parse rather than a thing to read.
pub fn short_date(iso: &str) -> Option<String> {
	let stamp: jiff::Timestamp = iso.parse().ok()?;
	Some(stamp.strftime("%b %-d, %Y").to_string())
}

/// Where a card is published: one tree per view, each mirroring the article tree.
///
/// The view is a directory rather than a suffix on the name so a locale can be synced or
/// dropped as a unit, and so the fallback is one prefix substitution rather than a filename
/// rewrite. Nothing outside this repository ever sees the layout -- a reader asks for
/// `/opengraph/{slug}.png?lang=ja` and the CDN resolves it. See spec/architecture.md.
pub fn card_path(public: &Path, view: &str, slug: &str) -> PathBuf {
	public
		.join("opengraph")
		.join(view)
		.join(format!("{slug}.png"))
}

/// The translation of one frontmatter value, or `None` when this view has none.
///
/// A segment id is the hash of its own text, so the title's id can be computed from the title
/// rather than found by re-splitting the article. The source view asks for nothing: it is the
/// article's own words.
fn translated(sidecar: &store::Sidecar, view: &locale::View, source: &str) -> Option<String> {
	let tag = view.tag?;
	let text = sidecar
		.segments
		.get(&segment::id_of(source))?
		.get(tag)?
		.text
		.clone();
	(!text.trim().is_empty()).then_some(text)
}

fn load_fonts(repo: &Path) -> Result<FontSystem, String> {
	let path = repo.join(FONT);
	if !path.is_file() {
		return Err(format!(
			"{} is missing -- the full font is not published, so fetch it into data/fonts",
			path.display()
		));
	}
	let mut db = fontdb::Database::new();
	db.load_font_file(&path)
		.map_err(|error| format!("could not read {}: {error}", path.display()))?;
	Ok(FontSystem::new_with_locale_and_db("en-US".to_owned(), db))
}

/// One card to draw: everything decided, nothing rendered yet.
///
/// Enumerated before any drawing so the work can be counted, skipped and spread across threads
/// without the decisions being repeated on each one.
pub struct Job {
	/// What names the card in a report, including its view: `ja development/a-thing`.
	pub label: String,
	pub target: PathBuf,
	pub site: String,
	pub title: String,
	pub subtitle: Option<String>,
	pub category: Option<String>,
	pub date: Option<String>,
}

/// Every card an article asks for: one per view, each in that view's own words.
fn article_jobs(public: &Path, site: &str, path: &Path, article: &Article) -> Vec<Job> {
	let sidecar = store::load(&store::path_for(path));
	let date = article.created.as_deref().and_then(short_date);

	locale::VIEWS
		.iter()
		.map(|view| {
			// The source view is the article's own words; every other view falls back to them
			// when that segment has not been translated yet, because a card in the wrong
			// language still says more than no card at all.
			let title =
				translated(&sidecar, view, &article.title).unwrap_or_else(|| article.title.clone());
			let subtitle = article
				.subtitle
				.as_ref()
				.map(|text| translated(&sidecar, view, text).unwrap_or_else(|| text.clone()));

			Job {
				label: format!("{} {}", view.code, article.slug),
				target: card_path(public, view.code, &article.slug),
				site: site.to_owned(),
				title,
				subtitle,
				category: article.category.clone(),
				date: date.clone(),
			}
		})
		.collect()
}

/// Draw every job that is not already published, in parallel.
///
/// `map_init` rather than a shared font system: shaping needs `&mut FontSystem`, so the choice
/// is one per thread or a lock every glyph goes through. One per thread costs a font parse per
/// worker and nothing after that.
pub fn render_all(repo: &Path, jobs: Vec<Job>, force: bool) -> Result<Outcome, String> {
	// Parsed once here as well, so a missing font fails before any thread starts rather than
	// once per worker.
	load_fonts(repo)?;

	let (todo, skipped): (Vec<Job>, Vec<Job>) = jobs
		.into_iter()
		.partition(|job| force || !job.target.is_file());

	let results: Vec<Result<(), (String, String)>> = todo
		.par_iter()
		.map_init(
			|| load_fonts(repo).expect("font already parsed once above"),
			|fonts, job| {
				let card = Card {
					site: &job.site,
					title: &job.title,
					subtitle: job.subtitle.as_deref(),
					category: job.category.as_deref(),
					date: job.date.as_deref(),
				};
				let pixels = layout::render(fonts, FAMILY, &card);
				let png = encode(&pixels).map_err(|error| (job.label.clone(), error))?;
				crate::image::store::write(&job.target, &png)
					.map_err(|error| (job.label.clone(), error.to_string()))
			},
		)
		.collect();

	let mut outcome = Outcome {
		skipped: skipped.len(),
		..Outcome::default()
	};
	for result in results {
		match result {
			Ok(()) => outcome.rendered += 1,
			Err(failure) => outcome.failed.push(failure),
		}
	}
	Ok(outcome)
}

/// Render a card for every article, in every view.
pub fn run(
	repo: &Path,
	public: &Path,
	articles: &Path,
	site: &str,
	force: bool,
) -> Result<Outcome, String> {
	let mut jobs = Vec::new();
	for path in crate::refs::markdown_under(articles).map_err(|e| e.to_string())? {
		let Some(article) = article_of(articles, &path) else {
			continue;
		};
		jobs.extend(article_jobs(public, site, &path, &article));
	}
	render_all(repo, jobs, force)
}

fn encode(pixels: &[u8]) -> Result<Vec<u8>, String> {
	let buffer = image::RgbaImage::from_raw(layout::WIDTH, layout::HEIGHT, pixels.to_vec())
		.ok_or("bad canvas")?;
	let mut out = Vec::new();
	image::DynamicImage::ImageRgba8(buffer)
		.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
		.map_err(|error| error.to_string())?;
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reads_the_fields_the_card_shows() {
		let root = std::env::temp_dir().join(format!("cms-og-{}", std::process::id()));
		let dir = root.join("development");
		std::fs::create_dir_all(&dir).expect("dir");
		let path = dir.join("a-thing.md");
		std::fs::write(
			&path,
			"---\ntitle: A Thing\nsubtitle: About the thing\ndescription: a much longer paragraph\n\
			 created: 2026-04-13T19:18:28.488Z\n---\n\nbody\n",
		)
		.expect("write");

		let article = article_of(&root, &path).expect("article");
		assert_eq!(article.slug, "development/a-thing");
		assert_eq!(article.title, "A Thing");
		assert_eq!(article.subtitle.as_deref(), Some("About the thing"));
		// The category is where the file sits, not something restated in the frontmatter.
		assert_eq!(article.category.as_deref(), Some("development"));
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn an_article_without_a_title_has_no_card() {
		let root = std::env::temp_dir().join(format!("cms-og-untitled-{}", std::process::id()));
		std::fs::create_dir_all(&root).expect("dir");
		let path = root.join("x.md");
		std::fs::write(&path, "---\nsubtitle: only this\n---\n").expect("write");
		assert!(article_of(&root, &path).is_none());
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn dates_are_shown_rather_than_printed() {
		assert_eq!(
			short_date("2026-04-13T19:18:28.488Z").as_deref(),
			Some("Apr 13, 2026")
		);
		assert_eq!(short_date("not a date"), None);
	}

	#[test]
	fn a_card_is_published_under_its_view_where_the_article_sits() {
		assert!(
			card_path(Path::new("/p"), "mw", "development/a-thing")
				.ends_with("opengraph/mw/development/a-thing.png")
		);
		assert!(
			card_path(Path::new("/p"), "ja", "development/a-thing")
				.ends_with("opengraph/ja/development/a-thing.png")
		);
	}
}
