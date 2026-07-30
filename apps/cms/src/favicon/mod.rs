//! Resolving a site's favicon and storing it under `data/public/favicon`.
//!
//! Everything here writes locally and nothing reaches R2; publishing is `mise run sync`'s
//! job. There is no allowlist any more -- the worker no longer writes, so there is nothing to
//! gate. See spec/architecture.md.

pub mod fetch;
pub mod host;
pub mod parse;

use parse::Tone;
use std::path::{Path, PathBuf};

pub struct Stored {
	pub path: PathBuf,
	pub skipped: bool,
}

#[derive(Debug)]
pub enum Error {
	NotResolved,
	UnsupportedType(String),
	Write(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NotResolved => write!(f, "no icon found"),
			Self::UnsupportedType(kind) => write!(f, "unusable content type: {kind}"),
			Self::Write(error) => write!(f, "could not write: {error}"),
		}
	}
}

/// Fetch `domain`'s icon and store it, unless it is already on disk.
///
/// Skipping what exists is what makes this safe to call on every build: the local tree is the
/// source of truth, so a file that is already there is the answer, and re-fetching would only
/// add load on somebody else's server.
pub fn store(root: &Path, domain: &str, tone: Option<Tone>, force: bool) -> Result<Stored, Error> {
	if !force && let Some(existing) = existing(root, domain, tone) {
		return Ok(Stored {
			path: existing,
			skipped: true,
		});
	}

	let icon = resolve(domain, tone).ok_or(Error::NotResolved)?;
	let extension = fetch::extension_for(&icon.content_type)
		.ok_or_else(|| Error::UnsupportedType(icon.content_type.clone()))?;

	// A themed request that fell back to the site's ordinary icon is stored unthemed. Writing
	// it under the themed name would claim the site ships a dark variant when it does not,
	// and every later run would trust that claim.
	let name = match tone.filter(|_| icon.matched_tone) {
		Some(tone) => format!("{domain}-{}.{extension}", tone.suffix()),
		None => format!("{domain}.{extension}"),
	};

	let directory = root.join("favicon");
	std::fs::create_dir_all(&directory).map_err(Error::Write)?;
	let path = directory.join(name);
	std::fs::write(&path, &icon.bytes).map_err(Error::Write)?;
	Ok(Stored {
		path,
		skipped: false,
	})
}

struct Icon {
	bytes: Vec<u8>,
	content_type: String,
	matched_tone: bool,
}

fn resolve(domain: &str, tone: Option<Tone>) -> Option<Icon> {
	if let Some(html) = fetch::html(domain)
		&& let head = parse::parse_head(&html)
		&& let Some(link) = parse::pick_icon(&head.links, tone)
		&& let Some(absolute) = host::absolute(domain, head.base.as_deref(), &link.href)
		&& let Some(fetched) = fetch::bytes(&absolute)
	{
		let wanted = tone.map(|tone| match tone {
			Tone::Dark => parse::MediaTone::Dark,
			Tone::Light => parse::MediaTone::Light,
		});
		let matched_tone =
			wanted.is_some_and(|wanted| parse::media_tone(link.media.as_deref()) == wanted);
		return Some(Icon {
			bytes: fetched.bytes,
			content_type: fetched.content_type,
			matched_tone,
		});
	}

	// Every site is entitled to serve /favicon.ico without declaring it.
	let fallback = fetch::bytes(&format!("https://{domain}/favicon.ico"))?;
	Some(Icon {
		bytes: fallback.bytes,
		content_type: fallback.content_type,
		matched_tone: false,
	})
}

fn existing(root: &Path, domain: &str, tone: Option<Tone>) -> Option<PathBuf> {
	let stem = match tone {
		Some(tone) => format!("{domain}-{}", tone.suffix()),
		None => domain.to_owned(),
	};
	["svg", "png", "jpg", "ico"]
		.iter()
		.map(|extension| root.join("favicon").join(format!("{stem}.{extension}")))
		.find(|path| path.exists())
}
