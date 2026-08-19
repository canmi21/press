//! Cards for the licence surface: the pages that are not articles.
//!
//! These are generated rather than authored, so what a card says has to come from the same two
//! places the page says it from -- the licence record for the facts, the message catalogs for
//! the words. Writing the copy here would give the site a second voice that agrees with the
//! first only until one of them is edited. See spec/architecture/media.md.

use super::messages;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Only what a card needs. The record carries far more per package, and reading it all here
/// would tie this to a shape it has no opinion about.
#[derive(Debug, Deserialize)]
pub struct Record {
	#[serde(default)]
	pub packages: BTreeMap<String, Package>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
	#[serde(default)]
	pub spdx: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
}

pub fn record_path(repo: &Path) -> std::path::PathBuf {
	repo.join("data").join("build").join("licenses.json")
}

pub fn load(path: &Path) -> Option<Record> {
	serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// What one route's card says, before a view decides which language it says it in.
pub struct Route {
	/// The page's own path, which is also the card's slug.
	pub slug: String,
	/// A message key, or a literal when the text is an identifier rather than prose.
	pub title: Title,
	pub subtitle: Subtitle,
	pub section: Section,
	pub badge: Badge,
	pub qualifier: Option<String>,
}

pub enum Title {
	/// Looked up per view.
	Message(&'static str),
	/// The same in every language: a licence identifier, a registry, a package name.
	Literal(String),
}

pub enum Subtitle {
	/// A message with slots, filled per view.
	Message(&'static str, Vec<(&'static str, String)>),
	Literal(String),
	None,
}

pub enum Section {
	Message(&'static str),
	Literal(String),
	None,
}

pub enum Badge {
	/// `card.packages`, filled with this count.
	Packages(usize),
	/// `card.more_licenses`: the other licences, in the `+N` form the article cards use.
	MoreLicenses(usize),
	/// The same in every language: an SPDX expression.
	Literal(String),
	None,
}

struct Coordinates {
	registry: String,
	name: String,
	version: String,
}

/// A purl as the readable coordinates used by the package page route.
fn coordinates_of(purl: &str) -> Option<Coordinates> {
	let (registry, package) = purl.strip_prefix("pkg:")?.split_once('/')?;
	let at = package.rfind('@')?;
	let name = percent_decode_str(&package[..at])
		.decode_utf8()
		.ok()?
		.into_owned();
	let version = percent_decode_str(&package[at + 1..])
		.decode_utf8()
		.ok()?
		.into_owned();
	Some(Coordinates {
		registry: registry.to_owned(),
		name,
		version,
	})
}

/// The registry a purl names, and the name the pages show for it.
fn registry_of(purl: &str) -> Option<String> {
	purl
		.strip_prefix("pkg:")?
		.split('/')
		.next()
		.map(str::to_owned)
}

fn registry_name(id: &str) -> &str {
	match id {
		"cargo" => "crates.io",
		other => other,
	}
}

/// Split an SPDX expression into the licence terms it names, mirroring the site's own splitter.
///
/// Whole tokens and case-sensitively, because SPDX writes its operators in capitals and the
/// tree contains every way a looser rule goes wrong: `LGPL-2.1-or-later` has a lowercase `or`
/// inside one identifier, and `Apache-2.0 WITH LLVM-exception` is one licence rather than two.
pub fn terms(expression: &str) -> Vec<String> {
	let flattened = expression.replace(['(', ')'], " ").replace('/', " OR ");
	let mut out: Vec<String> = Vec::new();
	for token in flattened.split_whitespace() {
		if token == "OR" || token == "AND" {
			continue;
		}
		if token == "WITH" {
			if let Some(previous) = out.pop() {
				out.push(format!("{previous} WITH"));
			}
			continue;
		}
		match out.last() {
			Some(last) if last.ends_with(" WITH") => {
				let joined = format!("{last} {token}");
				out.pop();
				out.push(joined);
			}
			_ => out.push(token.to_owned()),
		}
	}
	out.dedup();
	out
}

/// An SPDX term as the route segment the site files it under.
pub fn slug_of(license: &str) -> String {
	let lowered: String = license
		.to_lowercase()
		.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
		.collect();
	let mut out = String::new();
	let mut dash = false;
	for c in lowered.chars() {
		if c == '-' {
			dash = true;
			continue;
		}
		if dash && !out.is_empty() {
			out.push('-');
		}
		dash = false;
		out.push(c);
	}
	out
}

/// Every licence route that gets a card, down to the directories and no further.
pub fn directories(record: &Record) -> Vec<Route> {
	let total = record.packages.len();

	let mut by_license: BTreeMap<String, usize> = BTreeMap::new();
	let mut by_registry: BTreeMap<String, usize> = BTreeMap::new();
	for (purl, package) in &record.packages {
		if let Some(registry) = registry_of(purl) {
			*by_registry.entry(registry).or_default() += 1;
		}
		let mut seen = BTreeSet::new();
		for term in terms(package.spdx.as_deref().unwrap_or_default()) {
			if seen.insert(term.clone()) {
				*by_license.entry(term).or_default() += 1;
			}
		}
	}

	let mut routes = vec![
		Route {
			slug: "licenses".to_owned(),
			title: Title::Message("licenses.title"),
			// Its own line rather than the page's description, which is written for a search
			// result and runs to three clauses. Same reason an article card takes the subtitle
			// and leaves the description alone.
			subtitle: Subtitle::Message("card.licenses", vec![]),
			section: Section::None,
			badge: Badge::Packages(total),
			qualifier: None,
		},
		Route {
			slug: "licenses/pkgs".to_owned(),
			title: Title::Message("licenses.packages"),
			// The title is already the word "Packages"; this says how they are arranged.
			subtitle: Subtitle::Message("card.by_registry", vec![]),
			section: Section::Message("licenses.directory"),
			badge: Badge::Packages(total),
			qualifier: None,
		},
	];

	for (registry, count) in &by_registry {
		routes.push(Route {
			slug: format!("licenses/pkgs/{registry}"),
			title: Title::Literal(registry_name(registry).to_owned()),
			// The title names the registry, so this does not name it again.
			subtitle: Subtitle::Message("card.from_registry", vec![("count", count.to_string())]),
			section: Section::Message("licenses.packages"),
			badge: Badge::None,
			qualifier: None,
		});
	}

	for (license, count) in &by_license {
		routes.push(Route {
			slug: format!("licenses/{}", slug_of(license)),
			title: Title::Literal(license.clone()),
			// The title is the licence. Repeating it here would spend the one line under it
			// saying what the reader has already read.
			subtitle: Subtitle::Message("card.under_license", vec![("count", count.to_string())]),
			section: Section::Message("licenses.directory"),
			// The other licences, not this one. The subtitle already says how many packages
			// carry it, so repeating that here would use the corner to say nothing new.
			badge: Badge::MoreLicenses(by_license.len().saturating_sub(1)),
			qualifier: None,
		});
	}

	routes
}

/// Every individual package page, with the facts the package itself declares.
pub fn packages(record: &Record) -> Vec<Route> {
	record
		.packages
		.iter()
		.filter_map(|(purl, package)| {
			let coordinates = coordinates_of(purl)?;
			Some(Route {
				slug: format!(
					"licenses/pkgs/{}/{}@{}",
					coordinates.registry, coordinates.name, coordinates.version
				),
				title: Title::Literal(coordinates.name),
				subtitle: package
					.description
					.clone()
					.filter(|description| !description.trim().is_empty())
					.map_or(Subtitle::None, Subtitle::Literal),
				section: Section::Literal(registry_name(&coordinates.registry).to_owned()),
				badge: Badge::Literal(package.spdx.clone().unwrap_or_default()),
				qualifier: Some(coordinates.version),
			})
		})
		.collect()
}

/// Resolve one route's text for one view.
pub fn worded(
	route: &Route,
	catalog: &BTreeMap<String, String>,
) -> (String, Option<String>, Option<String>, String) {
	let title = match &route.title {
		Title::Message(key) => catalog.get(*key).cloned().unwrap_or_default(),
		Title::Literal(text) => text.clone(),
	};
	let subtitle = match &route.subtitle {
		Subtitle::Message(key, values) => catalog.get(*key).map(|template| {
			let pairs: Vec<(&str, &str)> = values
				.iter()
				.map(|(name, value)| (*name, value.as_str()))
				.collect();
			messages::fill(template, &pairs)
		}),
		Subtitle::Literal(text) => Some(text.clone()),
		Subtitle::None => None,
	};
	let section = match &route.section {
		Section::Message(key) => catalog.get(*key).cloned(),
		Section::Literal(text) => Some(text.clone()),
		Section::None => None,
	};
	let counted = |key: &str, count: usize| {
		catalog.get(key).map_or(String::new(), |template| {
			messages::fill(template, &[("count", &messages::compact(count))])
		})
	};
	let badge = match &route.badge {
		Badge::Packages(count) => counted("card.packages", *count),
		Badge::MoreLicenses(count) => counted("card.more_licenses", *count),
		Badge::Literal(text) => text.clone(),
		Badge::None => String::new(),
	};
	(title, subtitle, section, badge)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn splits_an_expression_the_way_the_site_does() {
		assert_eq!(terms("MIT OR Apache-2.0"), ["MIT", "Apache-2.0"]);
		assert_eq!(terms("LGPL-2.1-or-later"), ["LGPL-2.1-or-later"]);
		assert_eq!(
			terms("Apache-2.0 WITH LLVM-exception OR MIT"),
			["Apache-2.0 WITH LLVM-exception", "MIT"]
		);
		assert_eq!(
			terms("(MIT OR Apache-2.0) AND NCSA"),
			["MIT", "Apache-2.0", "NCSA"]
		);
	}

	#[test]
	fn a_licence_route_matches_the_slug_the_site_serves() {
		assert_eq!(
			slug_of("Apache-2.0 WITH LLVM-exception"),
			"apache-2-0-with-llvm-exception"
		);
		assert_eq!(slug_of("MIT"), "mit");
		assert_eq!(slug_of("0BSD"), "0bsd");
	}

	#[test]
	fn a_package_route_carries_its_own_declared_facts() {
		let record = Record {
			packages: BTreeMap::from([(
				"pkg:npm/%40sveltejs/kit@2.70.2".to_owned(),
				Package {
					spdx: Some("MIT".to_owned()),
					description: Some("Web development, streamlined".to_owned()),
				},
			)]),
		};
		let routes = packages(&record);
		let route = routes.first().expect("package route");
		assert_eq!(route.slug, "licenses/pkgs/npm/@sveltejs/kit@2.70.2");
		assert_eq!(route.qualifier.as_deref(), Some("2.70.2"));
		assert_eq!(
			worded(route, &BTreeMap::new()),
			(
				"@sveltejs/kit".to_owned(),
				Some("Web development, streamlined".to_owned()),
				Some("npm".to_owned()),
				"MIT".to_owned(),
			)
		);
	}

	#[test]
	fn a_package_without_declared_copy_leaves_those_slots_empty() {
		let record = Record {
			packages: BTreeMap::from([(
				"pkg:cargo/example@1.2.3".to_owned(),
				Package {
					spdx: None,
					description: None,
				},
			)]),
		};
		let routes = packages(&record);
		let route = routes.first().expect("package route");
		assert_eq!(
			worded(route, &BTreeMap::new()),
			(
				"example".to_owned(),
				None,
				Some("crates.io".to_owned()),
				String::new(),
			)
		);
	}

	#[test]
	fn every_record_package_has_a_card_route() {
		let repo = crate::paths::repo_root().expect("repo");
		let record = load(&record_path(&repo)).expect("license record");
		assert_eq!(packages(&record).len(), record.packages.len());
	}
}
