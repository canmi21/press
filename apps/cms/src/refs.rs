//! Reading the assets an article asks for.
//!
//! Every command that touches `data/public` starts here. What to derive, what to fetch, what
//! is missing and what is no longer wanted are all answers to one question: which assets do
//! the articles reference. Articles are the only authority -- something nothing links to is
//! not an asset, it is a leftover. See spec/architecture.md.

use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// `![alt](value)`, stopping at whitespace so a markdown title is not swallowed.
static MARKDOWN_IMAGE: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"!\[[^\]]*\]\(\s*([^)\s]+)").expect("static pattern"));

/// A linkcard directive and its attribute block.
static LINKCARD: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"::linkcard\{([^}]*)\}").expect("static pattern"));

/// An image directive, which names an asset the same way but asks for it cropped.
static IMAGE_DIRECTIVE: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"::image\{([^}]*)\}").expect("static pattern"));

/// One `name="value"` pair inside a directive.
static ATTRIBUTE: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r#"(\w+)="([^"]*)""#).expect("static pattern"));

/// A reference that has been through the pipeline: content id plus the format it resolved to.
static RESOLVED: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"^([0-9a-f]{32})\.([a-z0-9]+)$").expect("static pattern"));

/// An image an article names, exactly as it was written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageRef {
	pub file: PathBuf,
	pub value: String,
}

impl ImageRef {
	/// The content id and format, when this reference has already been processed.
	pub fn resolved(&self) -> Option<(&str, &str)> {
		resolved(&self.value)
	}
}

/// The icon a linkcard needs, which is always identified by the domain it links to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaviconRef {
	pub file: PathBuf,
	pub domain: String,
	/// The shade this card renders against, when it names one.
	pub tone: Option<String>,
	/// Where to take the icon from, when the site's own is not the one wanted.
	///
	/// This is an instruction to the collector and never a rendering instruction: the page
	/// always draws `/favicon/{domain}`, so once collected there is nothing in the article to
	/// rewrite. Leaving it in place also keeps the choice re-runnable -- rewriting it away
	/// would destroy the only record of where the icon came from.
	pub source: Option<String>,
}

/// Split a processed reference into its content id and format.
pub fn resolved(value: &str) -> Option<(&str, &str)> {
	let captures = RESOLVED.captures(value)?;
	let (_, [cid, extension]) = captures.extract();
	Some((cid, extension))
}

/// Whether a reference points somewhere this repository does not own.
fn is_external(value: &str) -> bool {
	let lowered = value.to_ascii_lowercase();
	lowered.starts_with("http://") || lowered.starts_with("https://") || lowered.starts_with("data:")
}

/// A domain an article links to, and what the article said about its icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
	pub domain: String,
	/// Where the icon should come from, when an article named a source.
	pub source: Option<String>,
	/// Which shade that source is the icon for.
	pub tone: Option<String>,
}

#[derive(Debug, Default)]
pub struct Scan {
	pub images: Vec<ImageRef>,
	pub favicons: Vec<FaviconRef>,
}

impl Scan {
	/// Content ids the articles resolve to -- exactly the set worth keeping in `data/public`.
	pub fn cids(&self) -> BTreeSet<String> {
		self
			.images
			.iter()
			.filter_map(|image| image.resolved().map(|(cid, _)| cid.to_owned()))
			.collect()
	}

	/// References that still name a file rather than a content id, deduplicated.
	///
	/// The same picture used in three articles is one thing to derive, not three.
	pub fn unresolved(&self) -> Vec<&ImageRef> {
		let mut seen = BTreeSet::new();
		self
			.images
			.iter()
			.filter(|image| image.resolved().is_none())
			.filter(|image| seen.insert(image.value.as_str()))
			.collect()
	}

	/// Every icon the articles need, as domain and the tone that must exist for it.
	pub fn icons(&self) -> BTreeSet<(String, Option<String>)> {
		self
			.favicons
			.iter()
			.map(|icon| (icon.domain.clone(), icon.tone.clone()))
			.collect()
	}

	/// The domains to collect from, each with whatever an article said about its icon.
	pub fn wanted(&self) -> Vec<Wanted> {
		let mut found: Vec<Wanted> = Vec::new();
		for icon in &self.favicons {
			let named = Wanted {
				domain: icon.domain.clone(),
				source: icon.source.clone(),
				// Only meaningful alongside a source: it says which shade that icon *is*, not
				// which shade the card happens to render against.
				tone: icon.source.as_ref().and(icon.tone.clone()),
			};
			match found.iter_mut().find(|found| found.domain == icon.domain) {
				// An override wins over the default wherever it appears, so one article naming
				// the icon for a site settles it for every other article linking there.
				Some(existing) if existing.source.is_none() => *existing = named,
				Some(_) => {}
				None => found.push(named),
			}
		}
		found.sort_by(|a, b| a.domain.cmp(&b.domain));
		found
	}
}

/// Read every article under `articles` and collect what it references.
pub fn scan(articles: &Path) -> std::io::Result<Scan> {
	let mut found = Scan::default();
	for path in markdown_under(articles)? {
		let text = std::fs::read_to_string(&path)?;
		collect(&path, &text, &mut found);
	}
	found.images.sort();
	found.favicons.sort();
	Ok(found)
}

fn collect(file: &Path, text: &str, into: &mut Scan) {
	for capture in MARKDOWN_IMAGE.captures_iter(text) {
		push_image(file, &capture[1], into);
	}

	// Cropping is presentation and changes nothing about which asset is wanted, so this reads
	// only the src. Missing it would be worse than cosmetic: an asset referenced solely by a
	// cropped directive would look unreferenced, and `cms gc` would delete it.
	for directive in IMAGE_DIRECTIVE.captures_iter(text) {
		if let Some((_, src)) = attributes(&directive[1])
			.iter()
			.find(|(key, _)| key == "src")
		{
			push_image(file, src, into);
		}
	}

	for card in LINKCARD.captures_iter(text) {
		let attributes = attributes(&card[1]);
		let get = |name: &str| attributes.iter().find(|(k, _)| k == name).map(|(_, v)| v);

		if let Some(src) = get("src") {
			push_image(file, src, into);
		}

		// Without a url there is no domain, and the icon has no identity to be stored under.
		let Some(domain) = get("url").and_then(|url| domain_of(url)) else {
			continue;
		};
		into.favicons.push(FaviconRef {
			file: file.to_path_buf(),
			domain,
			tone: get("tone").cloned(),
			source: get("favicon").cloned(),
		});
	}
}

fn push_image(file: &Path, value: &str, into: &mut Scan) {
	if is_external(value) {
		return;
	}
	into.images.push(ImageRef {
		file: file.to_path_buf(),
		value: value.to_owned(),
	});
}

fn attributes(block: &str) -> Vec<(String, String)> {
	ATTRIBUTE
		.captures_iter(block)
		.map(|capture| (capture[1].to_owned(), capture[2].to_owned()))
		.collect()
}

fn domain_of(link: &str) -> Option<String> {
	url::Url::parse(link)
		.ok()?
		.host_str()
		.map(str::to_ascii_lowercase)
}

pub fn markdown_under(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
	let mut found = Vec::new();
	if !directory.is_dir() {
		return Ok(found);
	}
	for entry in std::fs::read_dir(directory)?.filter_map(Result::ok) {
		let path = entry.path();
		if path.is_dir() {
			found.extend(markdown_under(&path)?);
		} else if path.extension().and_then(|e| e.to_str()) == Some("md") {
			found.push(path);
		}
	}
	found.sort();
	Ok(found)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn scan_text(text: &str) -> Scan {
		let mut found = Scan::default();
		collect(Path::new("a.md"), text, &mut found);
		found
	}

	#[test]
	fn splits_a_processed_reference() {
		assert_eq!(
			resolved("44b6081deaf0242ca3bf83d62a3b6c95.avif"),
			Some(("44b6081deaf0242ca3bf83d62a3b6c95", "avif"))
		);
	}

	#[test]
	fn a_bare_id_without_a_format_is_not_finished() {
		// The extension is what says which format was actually produced, so a reference
		// without one has not been through the pipeline.
		assert!(resolved("44b6081deaf0242ca3bf83d62a3b6c95").is_none());
	}

	#[test]
	fn a_filename_that_merely_looks_hex_is_not_an_id() {
		assert!(resolved("deadbeef.png").is_none());
		assert!(resolved("zzb6081deaf0242ca3bf83d62a3b6c95.avif").is_none());
	}

	#[test]
	fn finds_markdown_images_without_their_titles() {
		let found = scan_text("![alt](shot.png \"a title\") and ![](other.jpg)");
		let values: Vec<&str> = found.images.iter().map(|i| i.value.as_str()).collect();
		assert_eq!(values, vec!["shot.png", "other.jpg"]);
	}

	#[test]
	fn ignores_images_this_repository_does_not_own() {
		// Rewriting one of these would point an article at a copy that was never made.
		let found = scan_text("![](https://example.com/a.png) ![](data:image/png;base64,AA)");
		assert!(found.images.is_empty());
	}

	#[test]
	fn reads_both_halves_of_a_linkcard() {
		let found =
			scan_text(r#"::linkcard{src="shot.png" url="https://GitHub.com/x" title="t" tone="dark"}"#);
		assert_eq!(found.images.len(), 1);
		assert_eq!(found.images[0].value, "shot.png");

		let icon = &found.favicons[0];
		// Lowercased, because a domain is a name for a directory here and two spellings would
		// become two directories holding the same icon.
		assert_eq!(icon.domain, "github.com");
		assert_eq!(icon.tone.as_deref(), Some("dark"));
		assert_eq!(icon.source, None);
	}

	#[test]
	fn keeps_a_named_icon_source() {
		let found = scan_text(
			r#"::linkcard{url="https://sakura-ushio.icu" favicon="https://avatars.githubusercontent.com/u/1?v=4"}"#,
		);
		assert_eq!(
			found.favicons[0].source.as_deref(),
			Some("https://avatars.githubusercontent.com/u/1?v=4")
		);
	}

	#[test]
	fn an_image_directive_names_an_asset_like_any_other() {
		// Only the src matters here. If this were missed, an image used solely in cropped form
		// would read as unreferenced and be swept.
		let found = scan_text(r#"::image{src="shot.png" ratio="16:9" align="top" alt="a thing"}"#);
		assert_eq!(found.images.len(), 1);
		assert_eq!(found.images[0].value, "shot.png");
	}

	#[test]
	fn a_cropped_reference_counts_as_the_same_asset() {
		// The same picture shown cropped in one article and whole in another is one asset, and
		// its content id has nothing to do with how any page frames it.
		let found = scan_text(
			"![](44b6081deaf0242ca3bf83d62a3b6c95.avif)\n\
			 ::image{src=\"44b6081deaf0242ca3bf83d62a3b6c95.avif\" ratio=\"1:1\"}",
		);
		assert_eq!(found.cids().len(), 1);
	}

	#[test]
	fn a_linkcard_without_a_url_asks_for_no_icon() {
		let found = scan_text(r#"::linkcard{src="shot.png" title="t"}"#);
		assert!(found.favicons.is_empty());
		assert_eq!(found.images.len(), 1);
	}

	#[test]
	fn one_picture_used_twice_is_one_thing_to_derive() {
		let found = scan_text("![](shot.png)\n\n![](shot.png)\n\n![](other.png)");
		assert_eq!(found.unresolved().len(), 2);
	}

	#[test]
	fn separates_finished_references_from_pending_ones() {
		let found = scan_text("![](44b6081deaf0242ca3bf83d62a3b6c95.avif)\n![](shot.png)");
		assert_eq!(found.cids().len(), 1);
		assert!(found.cids().contains("44b6081deaf0242ca3bf83d62a3b6c95"));
		assert_eq!(found.unresolved().len(), 1);
	}

	#[test]
	fn an_override_anywhere_settles_the_domain_everywhere() {
		// Two articles link to one site and only one of them says where the icon comes from.
		// Collecting the default for the other would overwrite the chosen icon on every run.
		let found = scan_text(
			r#"::linkcard{url="https://a.com"}
			::linkcard{url="https://a.com" favicon="https://cdn.example/a.svg"}"#,
		);
		assert_eq!(
			found.wanted(),
			vec![Wanted {
				domain: "a.com".to_owned(),
				source: Some("https://cdn.example/a.svg".to_owned()),
				tone: None,
			}]
		);
	}

	#[test]
	fn a_tone_qualifies_the_icon_a_card_names_and_nothing_else() {
		// Alongside a source the tone says which shade that file *is*. Alone it only says what
		// the card renders against, which is no instruction to the collector.
		let named =
			scan_text(r#"::linkcard{url="https://a.com" tone="dark" favicon="https://x/i.png"}"#);
		assert_eq!(named.wanted()[0].tone.as_deref(), Some("dark"));

		let plain = scan_text(r#"::linkcard{url="https://a.com" tone="dark"}"#);
		assert_eq!(plain.wanted()[0].tone, None);
	}

	#[test]
	fn an_icon_is_wanted_once_per_tone() {
		let found = scan_text(
			r#"::linkcard{url="https://a.com" tone="dark"}
			::linkcard{url="https://a.com" tone="dark"}
			::linkcard{url="https://a.com"}"#,
		);
		assert_eq!(found.icons().len(), 2);
	}
}
