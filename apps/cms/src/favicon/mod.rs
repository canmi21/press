//! Resolving a site's favicons and storing them under `data/public/favicon/<domain>/`.
//!
//! Everything here writes locally and nothing reaches R2; publishing is `mise run sync`'s
//! job. There is no allowlist any more -- the worker no longer writes, so there is nothing to
//! gate. See spec/architecture.md.
//!
//! One directory per domain, holding `light.<ext>` and optionally `dark.<ext>`. The layout is
//! doing two jobs. It groups the variants, and **the directory existing is the record that
//! this domain was checked at all** -- so a site that turns out to have no dark icon is not
//! re-fetched forever looking for one. A flat `<domain>-dark.<ext>` cannot express "asked,
//! and the answer was no", because a missing file and an unasked question look identical.

pub mod fetch;
pub mod host;
pub mod parse;

use parse::{MediaTone, Tone};
use std::path::{Path, PathBuf};

pub struct Stored {
	pub written: Vec<PathBuf>,
	pub skipped: bool,
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

/// Fetch every variant a site offers and store them, unless the domain was already checked.
///
/// Resolving all tones in one pass rather than once per tone: the tones are decided by a
/// single reading of one page, so fetching that page three times would triple the load on
/// somebody else's server to learn the same thing.
pub fn store(root: &Path, domain: &str, force: bool) -> Result<Stored, Error> {
	let directory = root.join("favicon").join(domain);
	if !force && directory.is_dir() {
		return Ok(Stored {
			written: Vec::new(),
			skipped: true,
		});
	}

	let variants = resolve(domain);
	if variants.is_empty() {
		return Err(Error::NotResolved);
	}

	std::fs::create_dir_all(&directory).map_err(Error::Write)?;
	let mut written = Vec::new();
	for (tone, icon) in variants {
		let Some(extension) = fetch::extension_for(&icon.content_type) else {
			continue;
		};
		let path = directory.join(format!("{}.{extension}", tone.suffix()));
		std::fs::write(&path, &icon.bytes).map_err(Error::Write)?;
		written.push(path);
	}

	if written.is_empty() {
		return Err(Error::NotResolved);
	}
	Ok(Stored {
		written,
		skipped: false,
	})
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
		let differs = out
			.first()
			.is_none_or(|(_, light)| light.bytes != icon.bytes);
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
