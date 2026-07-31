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

use cosmic_text::FontSystem;
use cosmic_text::fontdb;
use layout::Card;
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

/// Where a card is published, mirroring the article tree.
pub fn card_path(public: &Path, slug: &str) -> PathBuf {
	public.join("opengraph").join(format!("{slug}.png"))
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

/// Render a card for every article, skipping those already published unless `force`.
pub fn run(
	repo: &Path,
	public: &Path,
	articles: &Path,
	site: &str,
	force: bool,
) -> Result<Outcome, String> {
	let mut fonts = load_fonts(repo)?;
	let mut outcome = Outcome::default();

	for path in crate::refs::markdown_under(articles).map_err(|e| e.to_string())? {
		let Some(article) = article_of(articles, &path) else {
			continue;
		};
		let target = card_path(public, &article.slug);
		if !force && target.is_file() {
			outcome.skipped += 1;
			continue;
		}

		let date = article.created.as_deref().and_then(short_date);
		let card = Card {
			site,
			title: &article.title,
			subtitle: article.subtitle.as_deref(),
			category: article.category.as_deref(),
			date: date.as_deref(),
		};

		let pixels = layout::render(&mut fonts, FAMILY, &card);
		match encode(&pixels) {
			Ok(png) => match crate::image::store::write(&target, &png) {
				Ok(()) => outcome.rendered += 1,
				Err(error) => outcome.failed.push((article.slug, error.to_string())),
			},
			Err(error) => outcome.failed.push((article.slug, error)),
		}
	}
	Ok(outcome)
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
	fn a_card_is_published_where_the_article_sits() {
		assert!(
			card_path(Path::new("/p"), "development/a-thing")
				.ends_with("opengraph/development/a-thing.png")
		);
	}
}
