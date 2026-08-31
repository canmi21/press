//! Resolving a site's favicons and storing them under `data/public/favicon/<domain>/`.
//!
//! Everything here writes locally and nothing reaches R2; publishing is `mise run sync`'s
//! job. There is no allowlist any more -- the worker no longer writes, so there is nothing to
//! gate. See spec/architecture/data.md.
//!
//! One directory per domain, holding `light.<ext>` and optionally `dark.<ext>`. The layout is
//! doing two jobs. It groups the variants, and **the directory existing is the record that
//! this domain was checked at all** -- so a site that turns out to have no dark icon is not
//! re-fetched forever looking for one. A flat `<domain>-dark.<ext>` cannot express "asked,
//! and the answer was no", because a missing file and an unasked question look identical.

pub mod collect;
pub mod fetch;
pub mod host;
pub mod parse;

use parse::{MediaTone, Tone};
use std::path::{Path, PathBuf};

pub struct Stored {
	pub written: Vec<PathBuf>,
	pub skipped: bool,
}

/// Icons fetched for one domain, before anything is written.
///
/// The split exists so the network call and the disk write can be separated by the caller: the
/// fetch is seconds of somebody else's server, the write is microseconds, and holding the record
/// across both would serialise every domain behind the slowest one. See spec/tasks.md.
#[derive(Debug)]
pub struct Icons {
	/// File name to bytes, already expanded to every tone the icon should be written under.
	pub files: Vec<(String, Vec<u8>)>,
}

/// Write what a fetch produced. No network, and no decisions -- those were made during the fetch.
pub fn write_fetched(root: &Path, domain: &str, icons: &Icons) -> Result<Stored, Error> {
	let directory = root.join("favicon").join(domain);
	std::fs::create_dir_all(&directory).map_err(Error::Write)?;
	let mut written = Vec::new();
	for (name, bytes) in &icons.files {
		let path = directory.join(name);
		std::fs::write(&path, bytes).map_err(Error::Write)?;
		written.push(path);
	}
	Ok(Stored { written, skipped: false })
}

#[derive(Debug)]
pub enum Error {
	NotResolved,
	Write(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NotResolved => write!(f, "no icon found"),
			Self::Write(error) => write!(f, "could not write: {error}"),
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::NotResolved => None,
			Self::Write(error) => Some(error),
		}
	}
}

/// The stored icon for a domain and tone, if one was collected.
///
/// A named tone is answered exactly or not at all, matching how the worker resolves it: a
/// card that asked for the dark icon and got the light one would draw a light silhouette on a
/// light surface, which is worse than the icon being absent. An unnamed tone takes whichever
/// exists.
pub fn stored(root: &Path, domain: &str, tone: Option<&str>) -> Option<PathBuf> {
	let directory = root.join("favicon").join(domain);
	let tones: &[&str] = match tone {
		Some("dark") => &["dark"],
		Some("light") => &["light"],
		_ => &["light", "dark"],
	};
	tones.iter().find_map(|name| {
		crate::extension::ICON_EXTENSIONS
			.iter()
			.map(|extension| directory.join(format!("{name}.{extension}")))
			.find(|path| path.is_file())
	})
}

/// Store one named icon as this domain's, taking the article's word for where it lives.
///
/// Used when a linkcard names its own icon: the site being linked to may have no usable
/// favicon, or the author may want the avatar rather than the site mark. Resolving it into the
/// domain's own slot is what lets the page go on rendering `/favicon/{domain}` and the article
/// go on carrying the source URL, with neither needing to know about the other.
/// `tone` is the shade the article said this icon *is*. Naming one stores it there alone;
/// naming none means the icon is simply the site's, and it goes under both.
pub fn store_named(
	root: &Path,
	domain: &str,
	source: &str,
	tone: Option<&str>,
	force: bool,
) -> Result<Stored, Error> {
	let Some(icons) = fetch_named(root, domain, source, tone, force)? else {
		return Ok(Stored { written: Vec::new(), skipped: true });
	};
	write_fetched(root, domain, &icons)
}

/// The network half of `store_named`.
pub fn fetch_named(
	root: &Path,
	domain: &str,
	source: &str,
	tone: Option<&str>,
	force: bool,
) -> Result<Option<Icons>, Error> {
	if !force && root.join("favicon").join(domain).is_dir() {
		return Ok(None);
	}
	let icon = fetch::bytes(source).ok_or(Error::NotResolved)?;
	let extension = crate::extension::for_icon(&icon.content_type).ok_or(Error::NotResolved)?;
	let tones: &[Tone] = match tone {
		Some("dark") => &[Tone::Dark],
		Some("light") => &[Tone::Light],
		_ => &[Tone::Light, Tone::Dark],
	};
	Ok(Some(Icons {
		files: tones
			.iter()
			.map(|tone| (format!("{}.{extension}", tone.suffix()), icon.bytes.clone()))
			.collect(),
	}))
}

/// Fetch every variant a site offers, deciding what each file should be called.
///
/// The network half of `store`. Returns `None` when the domain was already collected and this is
/// not a forced run, which is a skip rather than a failure.
///
/// Resolving all tones in one pass rather than once per tone: the tones are decided by a
/// single reading of one page, so fetching that page three times would triple the load on
/// somebody else's server to learn the same thing.
pub fn fetch_for(root: &Path, domain: &str, force: bool) -> Result<Option<Icons>, Error> {
	let directory = root.join("favicon").join(domain);
	if !force && directory.is_dir() {
		return Ok(None);
	}

	let variants = resolve(domain);
	if variants.is_empty() {
		return Err(Error::NotResolved);
	}

	// A site that publishes one icon publishes it for every context: the browser draws that
	// same file on light and dark chrome alike, and an icon meant for only one of them is
	// something a site has to go out of its way to declare. Storing it under both tones
	// records that rather than guessing at it -- the worker never substitutes, so otherwise a
	// card asking for the dark icon of a single-icon site gets nothing at all.
	//
	// Measured: remix.run and www.typeless.com both publish exactly one, and remix.run's is an
	// SVG carrying its own opaque backdrop, which is what makes it legible either way.
	let single = variants.len() == 1;

	let mut files = Vec::new();
	for (tone, icon) in variants {
		let Some(extension) = crate::extension::for_icon(&icon.content_type) else {
			continue;
		};
		let tones: &[Tone] =
			if single { &[Tone::Light, Tone::Dark] } else { std::slice::from_ref(&tone) };
		for tone in tones {
			files.push((format!("{}.{extension}", tone.suffix()), icon.bytes.clone()));
		}
	}

	if files.is_empty() {
		return Err(Error::NotResolved);
	}
	Ok(Some(Icons { files }))
}

/// Fetch every variant a site offers and store them, unless the domain was already checked.
pub fn store(root: &Path, domain: &str, force: bool) -> Result<Stored, Error> {
	let Some(icons) = fetch_for(root, domain, force)? else {
		return Ok(Stored { written: Vec::new(), skipped: true });
	};
	write_fetched(root, domain, &icons)
}

fn resolve(domain: &str) -> Vec<(Tone, fetch::Fetched)> {
	let Some(html) = fetch::html(domain) else {
		return fetch::bytes(&format!("https://{domain}/favicon.ico"))
			.map(|icon| vec![(Tone::Light, icon)])
			.unwrap_or_default();
	};

	let head = parse::parse_head(&html);
	let base = head.base.as_deref();
	let mut out = Vec::new();

	let declared = |tone: MediaTone| -> Option<String> {
		let candidates: Vec<parse::IconLink> = head
			.links
			.iter()
			.filter(|link| parse::media_tone(link.media.as_deref()) == tone)
			.cloned()
			.collect();
		let picked = parse::pick_icon(&candidates, None)?;
		host::absolute(domain, base, &picked.href)
	};

	// The neutral icon is what a site without any theming has, and what a themed site falls
	// back to. It is stored as `light` because an untinted icon is drawn for light
	// backgrounds; treating it as a third "any" name would only push the same choice onto
	// every reader.
	let neutral = declared(MediaTone::Any).or_else(|| Some(format!("https://{domain}/favicon.ico")));

	let light = declared(MediaTone::Light).or(neutral.clone());
	if let Some(url) = light.as_deref()
		&& let Some(icon) = fetch::bytes(url)
	{
		out.push((Tone::Light, icon));
	}

	// A declared dark icon is the standard mechanism and almost nobody uses it. The sibling
	// guess below is what actually finds dark icons in practice -- see the comment there.
	let dark = declared(MediaTone::Dark).or_else(|| light.as_deref().and_then(dark_sibling));
	if let Some(url) = dark
		&& let Some(icon) = fetch::bytes(&url)
	{
		let differs = out.first().is_none_or(|(_, light)| light.bytes != icon.bytes);
		if differs {
			out.push((Tone::Dark, icon));
		}
	}

	out
}

/// The `-dark` neighbour of an icon URL, by convention.
///
/// Surveyed sites almost never declare `media="(prefers-color-scheme: dark)"` on a link, yet
/// plenty ship two icons and swap them with JavaScript -- GitHub serves `favicon.svg` and
/// `favicon-dark.svg` and picks between them in script, which no amount of parsing the markup
/// will reveal. Guessing the neighbour costs one request that usually 404s, and the result is
/// only kept when it exists *and* differs from the light icon, so a server that answers 200
/// for everything cannot produce a bogus variant.
fn dark_sibling(url: &str) -> Option<String> {
	let (head, extension) = url.rsplit_once('.')?;
	if extension.is_empty() || extension.contains('/') || extension.len() > 5 {
		return None;
	}
	Some(format!("{head}-dark.{extension}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn builds_the_dark_neighbour_of_an_icon_url() {
		assert_eq!(
			dark_sibling("https://a.example/favicons/favicon.svg").as_deref(),
			Some("https://a.example/favicons/favicon-dark.svg")
		);
	}

	#[test]
	fn refuses_urls_with_no_usable_extension() {
		// The dot is in the host, not a filename, so there is no neighbour to guess.
		assert_eq!(dark_sibling("https://a.example/icon"), None);
		assert_eq!(dark_sibling("https://a.example/"), None);
	}
}
