//! The read-only workspace snapshot shared by the CLI and desktop Overview page.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::{alt, check, image, media, paths, refs, summary};

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
	pub articles: Articles,
	pub media: Media,
	pub health: Health,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Articles {
	pub total: usize,
	pub sections: Vec<ArticleSection>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArticleSection {
	pub name: String,
	pub articles: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
	pub referenced: usize,
	pub published: usize,
	pub described: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Health {
	pub warnings: usize,
	pub notices: usize,
	pub gaps: Vec<Gap>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
	pub level: Level,
	pub subject: String,
	pub detail: String,
	pub action: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
	Warn,
	Info,
}

#[derive(Debug)]
pub enum Error {
	Repository(paths::NotFound),
	Read(std::io::Error),
}

impl std::fmt::Display for Error {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Repository(error) => error.fmt(formatter),
			Self::Read(error) => error.fmt(formatter),
		}
	}
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Self::Read(error)
	}
}

pub fn snapshot() -> Result<Snapshot, Error> {
	let repository = paths::repo_root().map_err(Error::Repository)?;
	Ok(snapshot_at(&repository)?)
}

fn snapshot_at(repository: &Path) -> std::io::Result<Snapshot> {
	let contents = repository.join("contents");
	let public = repository.join("data").join("public");
	let article_paths = refs::markdown_under(&contents)?;
	let mut article_count = 0;
	let mut article_sections = BTreeMap::<String, usize>::new();
	for path in article_paths {
		let source = std::fs::read_to_string(&path)?;
		if summary::lang_of(&source).is_some() {
			article_count += 1;
			let section = path
				.strip_prefix(&contents)
				.ok()
				.and_then(Path::parent)
				.filter(|parent| !parent.as_os_str().is_empty())
				.and_then(|parent| parent.components().next())
				.and_then(|component| component.as_os_str().to_str())
				.unwrap_or("other")
				.to_owned();
			*article_sections.entry(section).or_default() += 1;
		}
	}

	let scan = refs::scan(&contents)?;
	let content_ids = scan.cids();
	let described = media::load(&media::path_for(repository));
	let referenced = content_ids.len() + scan.unresolved().len();
	let published = content_ids
		.iter()
		.filter(|content_id| image::store::meta_path(&public, content_id).is_file())
		.count();
	let descriptions = content_ids
		.iter()
		.filter(|content_id| !alt::wants_description(&described, content_id))
		.count();

	let gaps: Vec<Gap> = check::report(repository, &public, &contents)?
		.into_iter()
		.map(|gap| Gap {
			level: match gap.level {
				check::Level::Warn => Level::Warn,
				check::Level::Info => Level::Info,
			},
			subject: gap.what,
			detail: gap.detail,
			action: gap.action.map(check::Action::command),
		})
		.collect();
	let warnings = gaps.iter().filter(|gap| gap.level == Level::Warn).count();
	let notices = gaps.len() - warnings;

	Ok(Snapshot {
		articles: Articles {
			total: article_count,
			sections: article_sections
				.into_iter()
				.map(|(name, articles)| ArticleSection { name, articles })
				.collect(),
		},
		media: Media {
			referenced,
			published,
			described: descriptions,
		},
		health: Health {
			warnings,
			notices,
			gaps,
		},
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reports_authored_content_and_resource_health() {
		let root = std::env::temp_dir().join(format!("cms-overview-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&root);
		std::fs::create_dir_all(root.join("contents")).expect("contents");
		std::fs::create_dir_all(root.join("data").join("public")).expect("public");
		std::fs::create_dir_all(root.join("contents/notes")).expect("section");
		std::fs::write(
			root.join("contents/notes/article.md"),
			"---\ntitle: Article\nlang: en\n---\n\n![](44b6081deaf0242ca3bf83d62a3b6c95.avif)\n![](draft.png)",
		)
		.expect("article");
		std::fs::write(root.join("contents/homepage.md"), "---\ntitle: Home\n---\n").expect("homepage");
		let content_id = "44b6081deaf0242ca3bf83d62a3b6c95";
		let record = image::store::meta_path(&root.join("data/public"), content_id);
		std::fs::create_dir_all(record.parent().expect("record parent")).expect("record directory");
		std::fs::write(record, "{}").expect("record");

		let found = snapshot_at(&root).expect("snapshot");
		assert_eq!(found.articles.total, 1);
		assert_eq!(
			found.articles.sections,
			vec![ArticleSection {
				name: "notes".to_owned(),
				articles: 1,
			}]
		);
		assert_eq!(found.media.referenced, 2);
		assert_eq!(found.media.published, 1);
		assert_eq!(found.media.described, 0);
		assert_eq!(found.health.warnings, 1);
		assert_eq!(found.health.notices, 1);

		std::fs::remove_dir_all(root).ok();
	}
}
