//! What every derived record owes the writing, counted one class at a time.
//!
//! `cms check` answers this per item, which is the right shape for a person fixing one thing and
//! the wrong shape for deciding what to run: a list of two hundred lines does not say whether the
//! images are behind or the translations are. This counts each class against what it should hold
//! and names the command that closes the difference, so the interface can offer the run rather
//! than making somebody translate a list into a decision.
//!
//! Nothing here derives anything itself. It reads the same records `check` and `articles` read,
//! because a second opinion about what "complete" means is how two pages start disagreeing.

use serde::Serialize;
use std::path::Path;

use crate::{alt, articles, favicon, image, media, paths, refs};

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
	pub classes: Vec<Class>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Class {
	/// Stable key the interface addresses this class by, and never shown to a reader.
	pub id: String,
	pub name: String,
	/// One line on what the class is, for a reader who has not met the command.
	pub detail: String,
	pub have: usize,
	pub want: usize,
	/// The `cms` subcommand that closes the difference, where one exists.
	pub action: Option<String>,
	/// Whether that command spends money on a model. The interface warns before offering it.
	pub paid: bool,
}

impl Class {
	fn new(
		id: &str,
		name: &str,
		detail: &str,
		have: usize,
		want: usize,
		action: Option<&str>,
		paid: bool,
	) -> Self {
		Self {
			id: id.to_owned(),
			name: name.to_owned(),
			detail: detail.to_owned(),
			have,
			want,
			action: action.map(str::to_owned),
			paid,
		}
	}
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

pub fn report() -> Result<Report, Error> {
	let repository = paths::repo_root().map_err(Error::Repository)?;
	Ok(report_at(&repository)?)
}

pub fn report_at(repository: &Path) -> std::io::Result<Report> {
	let contents = repository.join("contents");
	let public = repository.join("data").join("public");

	let scan = refs::scan(&contents)?;
	let cids = scan.cids();
	let described = media::load(&media::path_for(repository))?;

	let published = cids
		.iter()
		.filter(|cid| image::store::meta_path(&public, cid).is_file())
		.count();
	let descriptions = cids
		.iter()
		.filter(|cid| !alt::wants_description(&described, cid))
		.count();

	let icons = scan.icons();
	let collected = icons
		.iter()
		.filter(|(domain, tone)| favicon::stored(&public, domain, tone.as_deref()).is_some())
		.count();

	let listing = articles::listing_at(repository)?;
	let translated: usize = listing
		.articles
		.iter()
		.map(|article| article.translated)
		.sum();
	let translatable: usize = listing.articles.iter().map(|article| article.wanted).sum();
	let summaries_wanted = listing.articles.len() * listing.locales.len();
	let summaries_missing: usize = listing
		.articles
		.iter()
		.map(|article| article.summary_gaps.len())
		.sum();

	Ok(Report {
		classes: vec![
			Class::new(
				"images",
				"Images",
				"Referenced pictures with a processed record in data/public.",
				published,
				// An unresolved reference names a picture that was never imported, so it counts
				// against the total rather than being absent from it -- otherwise importing one
				// would make the figure go down.
				cids.len() + scan.unresolved().len(),
				Some("image"),
				false,
			),
			Class::new(
				"descriptions",
				"Descriptions",
				"Accessible descriptions, written once per picture and inherited by every use.",
				descriptions,
				cids.len(),
				Some("alt"),
				true,
			),
			Class::new(
				"favicons",
				"Favicons",
				"Icons the linkcards draw, one per site an article links to.",
				collected,
				icons.len(),
				Some("favicon"),
				false,
			),
			Class::new(
				"translations",
				"Translations",
				"Article segments carried into every locale.",
				translated,
				translatable,
				Some("i18n"),
				true,
			),
			Class::new(
				"summaries",
				"Summaries",
				"A reader-facing summary per article per locale.",
				summaries_wanted.saturating_sub(summaries_missing),
				summaries_wanted,
				Some("summary"),
				true,
			),
		],
	})
}
