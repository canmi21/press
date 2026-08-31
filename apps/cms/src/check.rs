//! The `cms check` command: what the articles ask for that is not there.
//!
//! A report, never a gate. Publishing an article whose picture has not been imported yet is a
//! normal state to be in, and a build that refused it would only teach everyone to skip the
//! check. Severity carries the difference instead: a missing image leaves a hole in the page,
//! while a missing icon leaves a linkcard that still reads correctly.

use crate::refs::{self, Scan};
use crate::{favicon, image};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Level {
	/// The page will render with something visibly absent.
	Warn,
	/// A detail is missing and nothing else is affected.
	Info,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Action {
	Image,
	Alt,
	Favicon,
}

impl Action {
	pub fn command(self) -> &'static str {
		match self {
			Self::Image => "image",
			Self::Alt => "alt",
			Self::Favicon => "favicon",
		}
	}
}

impl Level {
	pub fn label(self) -> &'static str {
		match self {
			Self::Warn => "warn",
			Self::Info => "info",
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct Gap {
	pub level: Level,
	pub what: String,
	pub detail: String,
	pub action: Option<Action>,
}

/// Everything an article references that `data/public` cannot answer for.
pub fn report(repo: &Path, public: &Path, articles: &Path) -> std::io::Result<Vec<Gap>> {
	let scan = refs::scan(articles)?;
	let described = crate::media::load(&crate::media::path_for(repo))?;
	Ok(gaps(&scan, public, &described))
}

fn gaps(scan: &Scan, public: &Path, described: &crate::media::Media) -> Vec<Gap> {
	let mut found = Vec::new();

	for image in scan.unresolved() {
		found.push(Gap {
			level: Level::Warn,
			what: image.value.clone(),
			detail: "not derived yet".to_owned(),
			action: Some(Action::Image),
		});
	}

	for cid in scan.cids() {
		if !image::store::meta_path(public, &cid).is_file() {
			found.push(Gap {
				level: Level::Warn,
				what: cid,
				detail: "referenced but not published".to_owned(),
				action: Some(Action::Image),
			});
		}
	}

	// An image with no description is served correctly and read badly. That is a gap in what
	// the page says rather than in what it can show, so it sits below a missing image.
	for cid in scan.cids() {
		if crate::alt::wants_description(described, &cid) {
			found.push(Gap {
				level: Level::Info,
				what: cid,
				detail: "no description".to_owned(),
				action: Some(Action::Alt),
			});
		}
	}

	for (domain, tone) in scan.icons() {
		if favicon::stored(public, &domain, tone.as_deref()).is_none() {
			let detail = match &tone {
				Some(tone) => format!("no {tone} icon collected"),
				None => "no icon collected".to_owned(),
			};
			found.push(Gap { level: Level::Info, what: domain, detail, action: Some(Action::Favicon) });
		}
	}

	found
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	fn temp(name: &str) -> PathBuf {
		let path = std::env::temp_dir().join(format!("cms-check-{name}-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&path);
		std::fs::create_dir_all(&path).expect("temp");
		path
	}

	fn article(root: &Path, text: &str) {
		std::fs::create_dir_all(root.join("contents")).expect("dir");
		std::fs::write(root.join("contents/a.md"), text).expect("write");
	}

	#[test]
	fn a_missing_image_outranks_a_missing_icon() {
		// One leaves a hole in the page and the other does not, so they must not be reported
		// at the same level -- a report where everything is urgent is a report nobody reads.
		let root = temp("levels");
		article(
			&root,
			r#"![](shot.png)
			::linkcard{url="https://a.example"}"#,
		);

		let found = report(&root, &root.join("public"), &root.join("contents")).expect("report");
		let image = found.iter().find(|gap| gap.what == "shot.png").expect("image");
		let icon = found.iter().find(|gap| gap.what == "a.example").expect("icon");

		assert_eq!(image.level, Level::Warn);
		assert_eq!(icon.level, Level::Info);
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn a_published_reference_is_not_a_gap() {
		let root = temp("published");
		let cid = "44b6081deaf0242ca3bf83d62a3b6c95";
		article(&root, &format!("![]({cid}.avif)"));
		let meta = image::store::meta_path(&root.join("public"), cid);
		std::fs::create_dir_all(meta.parent().expect("parent")).expect("dir");
		std::fs::write(&meta, b"{}").expect("write");

		// One gap remains and should: the bytes are published, but nothing has described them.
		// That is what `cms alt` is for, and it is information rather than a hole in the page.
		let found = report(&root, &root.join("public"), &root.join("contents")).expect("report");
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].level, Level::Info);
		assert!(found[0].detail.contains("description"));
		std::fs::remove_dir_all(&root).ok();
	}

	#[test]
	fn a_reference_whose_record_is_gone_is_reported() {
		// The article says the work was done, and the bytes disagree. Sweeping too eagerly
		// looks exactly like this, which is the reason to notice it.
		let root = temp("swept");
		article(&root, "![](44b6081deaf0242ca3bf83d62a3b6c95.avif)");

		// Two now: the record is gone, and nothing has described the asset either. Only the
		// first is a warning -- a missing record leaves a hole, a missing description does not.
		let found = report(&root, &root.join("public"), &root.join("contents")).expect("report");
		assert_eq!(found.len(), 2);
		assert_eq!(found.iter().filter(|gap| gap.level == Level::Warn).count(), 1);
		std::fs::remove_dir_all(&root).ok();
	}
}
