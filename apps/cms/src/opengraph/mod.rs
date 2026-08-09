//! The `cms og` command: one card per page per language, rendered here and published as-is.
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
pub mod messages;

use crate::i18n::{segment, store};
use cosmic_text::FontSystem;
use cosmic_text::fontdb;
use layout::{Avatar, Card, Home};
use rayon::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Where the author's portrait lives.
///
/// Kept beside the font and for the same reason: it is bytes somebody else serves, so it is
/// fetched into `data/` once rather than requested by a local command. That also keeps the
/// address out of this crate, where `mise run refs` does not allow one to be written.
const AVATAR: &str = "data/avatar.png";

/// The identity the home card repeats, read from the file the pages read it from.
#[derive(Debug, Deserialize)]
struct SiteConfig {
	#[serde(default)]
	name: String,
	#[serde(default)]
	domain: String,
	#[serde(default)]
	author: SiteAuthor,
}

#[derive(Debug, Default, Deserialize)]
struct SiteAuthor {
	#[serde(default, rename = "fullName")]
	full_name: String,
	#[serde(default)]
	role: String,
}

pub fn config_path(repo: &Path) -> PathBuf {
	repo.join("apps").join("site").join("site.config.yaml")
}

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
	pub face: Face,
}

/// Which card this is, and the words only that kind has.
pub enum Face {
	Article {
		title: String,
		subtitle: Option<String>,
		category: Option<String>,
		date: Option<String>,
	},
	Home {
		domain: String,
		name: String,
		role: String,
		stats: String,
	},
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
				face: Face::Article {
					title,
					subtitle,
					category: article.category.clone(),
					date: date.clone(),
				},
			}
		})
		.collect()
}

/// The home page's card, which is a page rather than an article and has its own slug.
pub const HOME_SLUG: &str = "homepage";

/// What the site amounts to, counted once and shown on every view of the home card.
///
/// Characters rather than words, and the source text rather than each translation. A word is
/// not a unit CJK has, so counting them would produce a number that means something different
/// depending on which article it came from; a character is the same thing everywhere. The
/// source is counted for all nine views because the number describes the site, not the
/// translation somebody happens to be reading.
pub struct Census {
	pub articles: usize,
	pub characters: usize,
	pub languages: usize,
}

pub fn census(articles: &Path) -> Result<Census, String> {
	let mut counted = 0;
	let mut characters = 0;

	for path in crate::refs::markdown_under(articles).map_err(|e| e.to_string())? {
		// The bio page is content, not an article, and the pages do not list it as one either.
		if path.file_stem().and_then(|name| name.to_str()) == Some(HOME_SLUG) {
			continue;
		}
		let Ok(text) = std::fs::read_to_string(&path) else {
			continue;
		};
		let body = text
			.strip_prefix("---\n")
			.and_then(|rest| rest.split_once("\n---"))
			.map_or(text.as_str(), |(_, body)| body);
		counted += 1;
		characters += body.chars().filter(|c| !c.is_whitespace()).count();
	}

	Ok(Census {
		articles: counted,
		characters,
		languages: locale::VIEWS.len(),
	})
}

/// The home card, once per view, worded by that view's own catalog.
fn home_jobs(repo: &Path, public: &Path, config: &SiteConfig, census: &Census) -> Vec<Job> {
	locale::VIEWS
		.iter()
		.map(|view| {
			let catalog = messages::load(repo, view.code);
			let stats = catalog.get("card.stats").map_or(String::new(), |template| {
				messages::fill(
					template,
					&[
						("articles", &census.articles.to_string()),
						("characters", &messages::compact(census.characters)),
						("languages", &census.languages.to_string()),
					],
				)
			});

			Job {
				label: format!("{} {HOME_SLUG}", view.code),
				target: card_path(public, view.code, HOME_SLUG),
				site: config.name.clone(),
				face: Face::Home {
					domain: config.domain.clone(),
					name: config.author.full_name.clone(),
					role: config.author.role.clone(),
					stats,
				},
			}
		})
		.collect()
}

/// The author's portrait, decoded once and shared by every thread that draws it.
///
/// Absent rather than fatal: a clone without the file still gets every card, with the home one
/// missing a portrait instead of the whole command refusing to run.
fn load_avatar(repo: &Path) -> Option<Avatar> {
	let bytes = std::fs::read(repo.join(AVATAR)).ok()?;
	let decoded = image::load_from_memory(&bytes).ok()?.to_rgba8();
	let size = decoded.width().min(decoded.height());
	// Square, from the top-left, because the portrait is already square and a rectangle here
	// would mean choosing a crop nobody asked for.
	Some(Avatar {
		rgba: image::DynamicImage::ImageRgba8(decoded)
			.crop_imm(0, 0, size, size)
			.to_rgba8()
			.into_raw(),
		size,
	})
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

	// Decoded once and shared: it is read-only pixels, and decoding it per thread would repeat
	// the only part of this that is not text shaping.
	let avatar = load_avatar(repo);

	let results: Vec<Result<(), (String, String)>> = todo
		.par_iter()
		.map_init(
			|| load_fonts(repo).expect("font already parsed once above"),
			|fonts, job| {
				let pixels = match &job.face {
					Face::Article {
						title,
						subtitle,
						category,
						date,
					} => layout::render(
						fonts,
						FAMILY,
						&Card {
							site: &job.site,
							title,
							subtitle: subtitle.as_deref(),
							category: category.as_deref(),
							date: date.as_deref(),
						},
					),
					Face::Home {
						domain,
						name,
						role,
						stats,
					} => layout::render_home(
						fonts,
						FAMILY,
						&Home {
							site: &job.site,
							domain,
							name,
							role,
							stats,
							avatar: avatar.as_ref(),
						},
					),
				};
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

/// Render every card the site needs, in every view.
pub fn run(repo: &Path, public: &Path, articles: &Path, force: bool) -> Result<Outcome, String> {
	let text = std::fs::read_to_string(config_path(repo))
		.map_err(|error| format!("could not read the site config: {error}"))?;
	let config: SiteConfig =
		serde_yaml_ng::from_str(&text).map_err(|error| format!("site config: {error}"))?;

	let mut jobs = Vec::new();
	for path in crate::refs::markdown_under(articles).map_err(|e| e.to_string())? {
		// The bio page gets the home card rather than an article one, so it is not an article
		// here either.
		if path.file_stem().and_then(|name| name.to_str()) == Some(HOME_SLUG) {
			continue;
		}
		let Some(article) = article_of(articles, &path) else {
			continue;
		};
		jobs.extend(article_jobs(public, &config.name, &path, &article));
	}

	jobs.extend(home_jobs(repo, public, &config, &census(articles)?));
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
