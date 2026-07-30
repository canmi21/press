//! Reading icon candidates out of a page and choosing between them.
//!
//! Pure: no network, no filesystem. Everything that decides *which* icon a site gets lives
//! here so it can be tested against the shapes real pages actually use.

use regex::Regex;
use scraper::{Html, Selector};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
	Dark,
	Light,
}

impl Tone {
	pub fn suffix(self) -> &'static str {
		match self {
			Self::Dark => "dark",
			Self::Light => "light",
		}
	}
}

/// What a `media` attribute asks for. Most icons declare nothing and apply to both themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaTone {
	Dark,
	Light,
	Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconLink {
	pub href: String,
	pub sizes: Option<String>,
	pub kind: Option<String>,
	pub media: Option<String>,
}

/// Icon candidates plus the `<base href>` they resolve against, if the page declares one.
#[derive(Debug, Default)]
pub struct Head {
	pub links: Vec<IconLink>,
	pub base: Option<String>,
}

pub fn parse_head(html: &str) -> Head {
	let document = Html::parse_document(html);
	let mut head = Head::default();

	for element in document.select(selector("link")) {
		let attrs = element.value();
		let rel = attrs.attr("rel").unwrap_or_default().to_lowercase();
		if !rel.contains("icon") || is_not_a_favicon(&rel) {
			continue;
		}
		let Some(href) = attrs.attr("href").filter(|h| !h.is_empty()) else {
			continue;
		};
		head.links.push(IconLink {
			href: href.to_owned(),
			sizes: attrs.attr("sizes").map(str::to_owned),
			kind: attrs.attr("type").map(str::to_owned),
			media: attrs.attr("media").map(str::to_owned),
		});
	}

	head.base = document
		.select(selector("base"))
		.find_map(|element| element.value().attr("href"))
		.filter(|href| !href.is_empty())
		.map(str::to_owned);

	head
}

pub fn media_tone(media: Option<&str>) -> MediaTone {
	let Some(media) = media else {
		return MediaTone::Any;
	};
	let scheme = scheme_re();
	match scheme.captures(media).map(|c| c[1].to_lowercase()) {
		Some(value) if value == "dark" => MediaTone::Dark,
		Some(value) if value == "light" => MediaTone::Light,
		_ => MediaTone::Any,
	}
}

/// The largest width a `sizes` attribute declares. `sizes="any"` and junk both yield None.
pub fn max_declared_size(sizes: Option<&str>) -> Option<u32> {
	let sizes = sizes?;
	size_re()
		.captures_iter(sizes)
		.filter_map(|c| c[1].parse::<u32>().ok())
		.max()
}

/// Rels that contain the word "icon" but are not the site's favicon.
///
/// `mask-icon` is Safari's pinned-tab silhouette: monochrome by definition, and picked over
/// the real icon by any rule that prefers vectors. `fluid-icon` is a desktop-app icon. Both
/// are the wrong picture, and both are common enough that GitHub ships them side by side with
/// its actual favicon.
fn is_not_a_favicon(rel: &str) -> bool {
	rel
		.split_whitespace()
		.any(|part| part == "mask-icon" || part == "fluid-icon")
}

fn is_svg(link: &IconLink) -> bool {
	let by_type = link
		.kind
		.as_deref()
		.is_some_and(|k| k.to_lowercase().contains("svg"));
	let path = link.href.split(['?', '#']).next().unwrap_or_default();
	by_type || path.to_lowercase().ends_with(".svg")
}

/// Best candidate ignoring theme: vector beats raster, then the smallest icon that is still
/// at least 32px, then the largest available, then whatever came first.
///
/// The "smallest at least 32" rule is the interesting one. Favicons render at 16-32px, so a
/// 512px PNG is bytes spent on detail that is thrown away by the renderer; the goal is the
/// smallest file that is not visibly soft.
fn pick_by_quality(links: &[&IconLink]) -> Option<usize> {
	if links.is_empty() {
		return None;
	}
	if let Some(index) = links.iter().position(|link| is_svg(link)) {
		return Some(index);
	}

	let mut sized: Vec<(usize, u32)> = links
		.iter()
		.enumerate()
		.filter_map(|(index, link)| max_declared_size(link.sizes.as_deref()).map(|size| (index, size)))
		.collect();
	sized.sort_by_key(|(_, size)| *size);

	if let Some((index, _)) = sized.iter().find(|(_, size)| *size >= 32) {
		return Some(*index);
	}
	if let Some((index, _)) = sized.last() {
		return Some(*index);
	}
	Some(0)
}

/// Pick an icon, preferring one that declares the requested theme.
///
/// Falls through deliberately: a themed request that finds no themed icon should still get
/// the site's normal icon rather than nothing.
pub fn pick_icon(links: &[IconLink], tone: Option<Tone>) -> Option<&IconLink> {
	if links.is_empty() {
		return None;
	}

	if let Some(tone) = tone {
		let wanted = match tone {
			Tone::Dark => MediaTone::Dark,
			Tone::Light => MediaTone::Light,
		};
		let matching: Vec<&IconLink> = links
			.iter()
			.filter(|link| media_tone(link.media.as_deref()) == wanted)
			.collect();
		if let Some(index) = pick_by_quality(&matching) {
			return Some(matching[index]);
		}
	}

	let untargeted: Vec<&IconLink> = links
		.iter()
		.filter(|link| media_tone(link.media.as_deref()) == MediaTone::Any)
		.collect();
	if let Some(index) = pick_by_quality(&untargeted) {
		return Some(untargeted[index]);
	}

	let all: Vec<&IconLink> = links.iter().collect();
	pick_by_quality(&all).map(|index| all[index])
}

fn selector(css: &'static str) -> &'static Selector {
	static CACHE: OnceLock<std::sync::Mutex<Vec<(&'static str, &'static Selector)>>> =
		OnceLock::new();
	let cache = CACHE.get_or_init(|| std::sync::Mutex::new(Vec::new()));
	let mut entries = cache.lock().expect("selector cache poisoned");
	if let Some((_, selector)) = entries.iter().find(|(key, _)| *key == css) {
		return selector;
	}
	let leaked: &'static Selector = Box::leak(Box::new(
		Selector::parse(css).expect("static selector is valid"),
	));
	entries.push((css, leaked));
	leaked
}

fn size_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	RE.get_or_init(|| Regex::new(r"(?i)(\d+)x(\d+)").expect("static pattern is valid"))
}

fn scheme_re() -> &'static Regex {
	static RE: OnceLock<Regex> = OnceLock::new();
	RE.get_or_init(|| {
		Regex::new(r"(?i)prefers-color-scheme\s*:\s*(dark|light)").expect("static pattern is valid")
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn link(href: &str, sizes: Option<&str>, kind: Option<&str>, media: Option<&str>) -> IconLink {
		IconLink {
			href: href.into(),
			sizes: sizes.map(Into::into),
			kind: kind.map(Into::into),
			media: media.map(Into::into),
		}
	}

	// The four cases that decided against regex matching. Each one silently produced a wrong
	// icon before; if this file ever goes back to pattern matching, these fail. See
	// spec/code.md on dependency budgets.
	#[test]
	fn ignores_a_commented_out_link() {
		let head = parse_head(r#"<head><!-- <link rel="icon" href="/old.png"> --></head>"#);
		assert!(head.links.is_empty());
	}

	#[test]
	fn ignores_a_link_written_inside_a_script() {
		let html = r#"<head><script>var s = '<link rel="icon" href="/nope.png">';</script></head>"#;
		assert!(parse_head(html).links.is_empty());
	}

	#[test]
	fn decodes_entities_in_href() {
		let head = parse_head(r#"<head><link rel="icon" href="/i.png?a=1&amp;b=2"></head>"#);
		assert_eq!(head.links[0].href, "/i.png?a=1&b=2");
	}

	#[test]
	fn reads_the_base_href() {
		let html =
			r#"<head><base href="https://cdn.example.com/"><link rel="icon" href="/i.png"></head>"#;
		assert_eq!(
			parse_head(html).base.as_deref(),
			Some("https://cdn.example.com/")
		);
	}

	#[test]
	fn keeps_only_links_whose_rel_mentions_icon() {
		let html = r#"<head>
			<link rel="stylesheet" href="/a.css">
			<link rel="shortcut icon" href="/a.ico">
			<link rel="apple-touch-icon" href="/b.png">
		</head>"#;
		let hrefs: Vec<_> = parse_head(html).links.into_iter().map(|l| l.href).collect();
		assert_eq!(hrefs, vec!["/a.ico", "/b.png"]);
	}

	#[test]
	fn rejects_rels_that_merely_contain_the_word_icon() {
		// GitHub ships all three side by side. mask-icon is a monochrome silhouette and
		// happens to be SVG, so a rule that prefers vectors picks it over the real favicon.
		let html = r##"<head>
			<link rel="fluid-icon" href="/fluidicon.png">
			<link rel="mask-icon" href="/pinned.svg" color="#000">
			<link rel="icon" type="image/svg+xml" href="/favicon.svg">
		</head>"##;
		let hrefs: Vec<_> = parse_head(html).links.into_iter().map(|l| l.href).collect();
		assert_eq!(hrefs, vec!["/favicon.svg"]);
	}

	#[test]
	fn keeps_apple_touch_icon() {
		// Unlike mask-icon this is a real, full-colour icon and worth having as a fallback.
		let html = r#"<head><link rel="apple-touch-icon" href="/a.png"></head>"#;
		assert_eq!(parse_head(html).links.len(), 1);
	}

	#[test]
	fn skips_a_link_with_no_href() {
		assert!(
			parse_head(r#"<head><link rel="icon"></head>"#)
				.links
				.is_empty()
		);
	}

	#[test]
	fn reads_the_largest_declared_size() {
		assert_eq!(max_declared_size(Some("16x16 32x32 48x48")), Some(48));
		assert_eq!(max_declared_size(Some("any")), None);
		assert_eq!(max_declared_size(None), None);
	}

	#[test]
	fn recognises_a_color_scheme_query() {
		assert_eq!(
			media_tone(Some("(prefers-color-scheme: dark)")),
			MediaTone::Dark
		);
		assert_eq!(
			media_tone(Some("(prefers-color-scheme:light)")),
			MediaTone::Light
		);
		assert_eq!(media_tone(Some("print")), MediaTone::Any);
		assert_eq!(media_tone(None), MediaTone::Any);
	}

	#[test]
	fn prefers_vector_over_any_raster() {
		let links = vec![
			link("/big.png", Some("512x512"), None, None),
			link("/icon.svg", None, None, None),
		];
		assert_eq!(pick_icon(&links, None).unwrap().href, "/icon.svg");
	}

	#[test]
	fn detects_svg_by_type_when_the_url_does_not_say_so() {
		let links = vec![
			link("/a.png", Some("64x64"), None, None),
			link("/icon", None, Some("image/svg+xml"), None),
		];
		assert_eq!(pick_icon(&links, None).unwrap().href, "/icon");
	}

	#[test]
	fn takes_the_smallest_icon_that_is_still_big_enough() {
		let links = vec![
			link("/16.png", Some("16x16"), None, None),
			link("/32.png", Some("32x32"), None, None),
			link("/512.png", Some("512x512"), None, None),
		];
		assert_eq!(pick_icon(&links, None).unwrap().href, "/32.png");
	}

	#[test]
	fn falls_back_to_the_largest_when_everything_is_small() {
		let links = vec![
			link("/16.png", Some("16x16"), None, None),
			link("/24.png", Some("24x24"), None, None),
		];
		assert_eq!(pick_icon(&links, None).unwrap().href, "/24.png");
	}

	#[test]
	fn prefers_an_icon_declaring_the_requested_theme() {
		let links = vec![
			link(
				"/light.png",
				Some("32x32"),
				None,
				Some("(prefers-color-scheme: light)"),
			),
			link(
				"/dark.png",
				Some("32x32"),
				None,
				Some("(prefers-color-scheme: dark)"),
			),
			link("/plain.png", Some("32x32"), None, None),
		];
		assert_eq!(
			pick_icon(&links, Some(Tone::Dark)).unwrap().href,
			"/dark.png"
		);
	}

	#[test]
	fn falls_back_to_the_untargeted_icon_when_the_theme_is_missing() {
		// A themed request must still return the site's ordinary icon rather than nothing.
		let links = vec![
			link("/plain.png", Some("32x32"), None, None),
			link(
				"/light.png",
				Some("32x32"),
				None,
				Some("(prefers-color-scheme: light)"),
			),
		];
		assert_eq!(
			pick_icon(&links, Some(Tone::Dark)).unwrap().href,
			"/plain.png"
		);
	}

	#[test]
	fn returns_nothing_when_there_are_no_candidates() {
		assert!(pick_icon(&[], None).is_none());
	}
}
