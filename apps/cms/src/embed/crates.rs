//! Resolving a crate's dependency tree from the sparse index, once, at build time.
//!
//! The card an article shows is a whole transitive tree with sizes, which is a resolver rather
//! than a lookup: the index gives version requirements, not versions, so every edge has to be
//! matched against what is published. Doing that in the browser meant a proxy route and a
//! request per reader; doing it here means a record in git and a page that renders from a
//! checkout.
//!
//! Rebuildable from what git already holds -- an article names the crate, and crates.io answers
//! for free -- which is why the result belongs under `data/build/`. See spec/architecture/data.md.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet, VecDeque};

/// How deep the tree is walked.
///
/// Not a correctness limit but a courtesy one: a pathological graph would otherwise fetch for
/// minutes against a public index that is not ours. Ten levels covers every real crate here.
const MAX_DEPTH: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dep {
	pub name: String,
	pub version: String,
	pub kind: String,
	pub optional: bool,
	pub target: Option<String>,
	pub features: Vec<String>,
	/// Bytes of the published `.crate` archive, when crates.io reported one.
	pub size: Option<u64>,
	/// 0 for a direct dependency; deeper for one pulled in by another.
	pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Crate {
	pub name: String,
	pub version: String,
	pub rust_version: Option<String>,
	pub features: BTreeMap<String, Vec<String>>,
	pub deps: Vec<Dep>,
	pub total_dep_size: u64,
}

/// One line of the sparse index: a published version and what it needs.
#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
	pub name: String,
	pub vers: String,
	#[serde(default)]
	pub deps: Vec<IndexDep>,
	#[serde(default)]
	pub features: BTreeMap<String, Vec<String>>,
	#[serde(default)]
	pub yanked: bool,
	#[serde(default)]
	pub rust_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexDep {
	pub name: String,
	#[serde(default)]
	pub package: Option<String>,
	pub req: String,
	#[serde(default)]
	pub features: Vec<String>,
	#[serde(default)]
	pub optional: bool,
	#[serde(default)]
	pub target: Option<String>,
	#[serde(default)]
	pub kind: Option<String>,
}

/// Where a crate's index file lives.
///
/// The index shards by name length and then by leading characters, which is why this is a rule
/// rather than a path join. Getting it wrong returns a 404 that looks exactly like an
/// unpublished crate, so it is worth pinning in a test.
pub fn index_path(name: &str) -> String {
	let lower = name.to_lowercase();
	match lower.len() {
		1 => format!("1/{lower}"),
		2 => format!("2/{lower}"),
		3 => format!("3/{}/{lower}", &lower[0..1]),
		_ => format!("{}/{}/{lower}", &lower[0..2], &lower[2..4]),
	}
}

/// The newest published version satisfying a requirement, ignoring yanked and pre-release ones.
///
/// A pre-release is skipped unless the requirement asks for one, which is cargo's own rule: a
/// range like `^1` must not silently resolve to `2.0.0-alpha`.
pub fn best_match<'a>(entries: &'a [IndexEntry], req: &str) -> Option<&'a IndexEntry> {
	let range = semver::VersionReq::parse(req).ok()?;
	entries
		.iter()
		.filter(|entry| !entry.yanked)
		.filter_map(|entry| {
			let version = semver::Version::parse(&entry.vers).ok()?;
			(range.matches(&version) && (!version.pre.is_empty() == req.contains('-')))
				.then_some((version, entry))
		})
		.max_by(|(a, _), (b, _)| a.cmp(b))
		.map(|(_, entry)| entry)
}

/// The newest publishable version of a crate, for the root of a tree.
pub fn newest(entries: &[IndexEntry]) -> Option<&IndexEntry> {
	entries
		.iter()
		.filter(|entry| !entry.yanked)
		.filter_map(|entry| Some((semver::Version::parse(&entry.vers).ok()?, entry)))
		.filter(|(version, _)| version.pre.is_empty())
		.max_by(|(a, _), (b, _)| a.cmp(b))
		.map(|(_, entry)| entry)
}

/// Whether the graph below an edge is traversed.
///
/// Root optional and dev edges are still displayed: the distinction is part of what the chart
/// explains. They stop at that first tile because their transitive weight is not carried by a
/// default consumer.
fn is_traversed(dep: &IndexDep) -> bool {
	!dep.optional && dep.kind.as_deref().unwrap_or("normal") != "dev"
}

fn dependency_name(dep: &IndexDep) -> &str {
	dep.package.as_deref().unwrap_or(&dep.name)
}

/// Walk the tree breadth first, resolving each requirement against the index.
///
/// Breadth first so `depth` is the shortest path to a crate rather than whichever route the
/// walk happened to take first -- "how far from the root is this" is the question the card
/// asks, and a depth-first number would answer a different one each run.
pub fn resolve(
	root: &IndexEntry,
	mut fetch: impl FnMut(&str) -> Option<Vec<IndexEntry>>,
	mut size: impl FnMut(&str, &str) -> Option<u64>,
) -> Crate {
	let mut seen: HashSet<(String, String)> = HashSet::new();
	let mut deps: Vec<Dep> = Vec::new();
	let mut queue: VecDeque<(IndexDep, usize)> =
		root.deps.iter().map(|dep| (dep.clone(), 0usize)).collect();

	while let Some((wanted, depth)) = queue.pop_front() {
		if depth > MAX_DEPTH {
			continue;
		}
		let name = dependency_name(&wanted);
		let Some(entries) = fetch(name) else {
			continue;
		};
		let Some(chosen) = best_match(&entries, &wanted.req) else {
			continue;
		};
		// Cargo may compile two versions of one crate, but never compiles one name/version pair
		// twice. Remember the pair rather than the last version seen for a name: alternating
		// `syn 2`, `syn 3`, `syn 2` otherwise counts the first version twice.
		if !seen.insert((chosen.name.clone(), chosen.vers.clone())) {
			continue;
		}
		deps.push(Dep {
			name: chosen.name.clone(),
			version: chosen.vers.clone(),
			kind: wanted.kind.clone().unwrap_or_else(|| "normal".to_owned()),
			optional: wanted.optional,
			target: wanted.target.clone(),
			features: wanted.features.clone(),
			size: size(&chosen.name, &chosen.vers),
			depth,
		});
		if is_traversed(&wanted) {
			for next in chosen.deps.iter().filter(|dep| dep.kind.as_deref().unwrap_or("normal") != "dev")
			{
				queue.push_back((next.clone(), depth + 1));
			}
		}
	}

	deps.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.name.cmp(&b.name)));
	let total_dep_size = deps.iter().filter_map(|dep| dep.size).sum();
	Crate {
		name: root.name.clone(),
		version: root.vers.clone(),
		rust_version: root.rust_version.clone(),
		features: root.features.clone(),
		deps,
		total_dep_size,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn entry(name: &str, vers: &str, deps: Vec<IndexDep>) -> IndexEntry {
		IndexEntry {
			name: name.to_owned(),
			vers: vers.to_owned(),
			deps,
			features: BTreeMap::new(),
			yanked: false,
			rust_version: None,
		}
	}

	fn needs(name: &str, req: &str) -> IndexDep {
		IndexDep {
			name: name.to_owned(),
			package: None,
			req: req.to_owned(),
			features: Vec::new(),
			optional: false,
			target: None,
			kind: Some("normal".to_owned()),
		}
	}

	#[test]
	fn the_index_shards_by_name_length() {
		// A wrong path returns a 404 that reads exactly like an unpublished crate, so the rule is
		// pinned rather than trusted.
		assert_eq!(index_path("a"), "1/a");
		assert_eq!(index_path("io"), "2/io");
		assert_eq!(index_path("log"), "3/l/log");
		assert_eq!(index_path("serde"), "se/rd/serde");
		assert_eq!(index_path("Seam-CLI"), "se/am/seam-cli");
	}

	#[test]
	fn a_requirement_takes_the_newest_match_and_skips_yanked() {
		let mut yanked = entry("x", "1.5.0", vec![]);
		yanked.yanked = true;
		let entries = vec![
			entry("x", "1.0.0", vec![]),
			entry("x", "1.4.0", vec![]),
			yanked,
			entry("x", "2.0.0", vec![]),
		];
		assert_eq!(best_match(&entries, "^1").map(|e| e.vers.as_str()), Some("1.4.0"));
		assert_eq!(best_match(&entries, "^2").map(|e| e.vers.as_str()), Some("2.0.0"));
		assert!(best_match(&entries, "^3").is_none());
	}

	#[test]
	fn a_prerelease_is_not_picked_up_by_a_plain_range() {
		// cargo's own rule: `^1` must not resolve to 2.0.0-alpha, and must not resolve to
		// 1.1.0-beta either.
		let entries = vec![entry("x", "1.0.0", vec![]), entry("x", "1.1.0-beta", vec![])];
		assert_eq!(best_match(&entries, "^1").map(|e| e.vers.as_str()), Some("1.0.0"));
	}

	#[test]
	fn dev_and_optional_edges_are_shown_but_not_traversed() {
		// Their status is useful at the root, but their children are not default consumer weight.
		let mut dev = needs("only-for-tests", "^1");
		dev.kind = Some("dev".to_owned());
		let mut off = needs("behind-a-feature", "^1");
		off.optional = true;
		let root = entry("root", "1.0.0", vec![dev, off, needs("real", "^1")]);

		let resolved = resolve(
			&root,
			|name| {
				["only-for-tests", "behind-a-feature", "real"]
					.contains(&name)
					.then(|| vec![entry(name, "1.2.0", vec![needs("must-not-appear", "^1")])])
			},
			|_, _| Some(10),
		);
		assert_eq!(resolved.deps.len(), 3);
		assert!(resolved.deps.iter().any(|dep| dep.optional));
		assert!(resolved.deps.iter().any(|dep| dep.kind == "dev"));
		assert!(!resolved.deps.iter().any(|dep| dep.name == "must-not-appear"));
	}

	#[test]
	fn a_renamed_dependency_resolves_its_package_name() {
		let mut renamed = needs("local-name", "^1");
		renamed.package = Some("published-name".to_owned());
		let root = entry("root", "1.0.0", vec![renamed]);
		let resolved = resolve(
			&root,
			|name| (name == "published-name").then(|| vec![entry(name, "1.0.0", vec![])]),
			|_, _| Some(10),
		);
		assert_eq!(resolved.deps[0].name, "published-name");
	}

	#[test]
	fn a_crate_reached_twice_is_counted_once_at_its_shortest_depth() {
		// A diamond is one dependency a consumer compiles once. Listing it twice would double
		// its bytes in the total, and a depth-first walk would report whichever route it took
		// first rather than how far the crate actually is.
		let root = entry("root", "1.0.0", vec![needs("shared", "^1"), needs("mid", "^1")]);
		let resolved = resolve(
			&root,
			|name| match name {
				"shared" => Some(vec![entry("shared", "1.0.0", vec![])]),
				"mid" => Some(vec![entry("mid", "1.0.0", vec![needs("shared", "^1")])]),
				_ => None,
			},
			|_, _| Some(100),
		);
		assert_eq!(resolved.deps.iter().filter(|d| d.name == "shared").count(), 1);
		assert_eq!(resolved.deps.iter().find(|d| d.name == "shared").map(|d| d.depth), Some(0));
		assert_eq!(resolved.total_dep_size, 200);
	}

	#[test]
	fn returning_to_an_older_version_does_not_count_it_twice() {
		// Real dependency graphs can alternate versions while walking breadth first. Remembering
		// only the last version seen for a name made `syn 2, syn 3, syn 2` three compiled crates.
		let root = entry(
			"root",
			"1.0.0",
			vec![needs("left", "^1"), needs("middle", "^1"), needs("right", "^1")],
		);
		let resolved = resolve(
			&root,
			|name| match name {
				"left" => Some(vec![entry("left", "1.0.0", vec![needs("syn", "^2")])]),
				"middle" => Some(vec![entry("middle", "1.0.0", vec![needs("syn", "^3")])]),
				"right" => Some(vec![entry("right", "1.0.0", vec![needs("syn", "^2")])]),
				"syn" => Some(vec![entry("syn", "2.0.0", vec![]), entry("syn", "3.0.0", vec![])]),
				_ => None,
			},
			|_, _| Some(100),
		);
		let syn = resolved.deps.iter().filter(|dep| dep.name == "syn").collect::<Vec<_>>();
		assert_eq!(syn.len(), 2);
		assert_eq!(resolved.total_dep_size, 500);
	}

	#[test]
	fn a_crate_the_index_does_not_have_is_skipped_rather_than_fatal() {
		// One unreachable dependency should cost that dependency, not the card.
		let root = entry("root", "1.0.0", vec![needs("gone", "^1"), needs("here", "^1")]);
		let resolved = resolve(
			&root,
			|name| (name == "here").then(|| vec![entry("here", "1.0.0", vec![])]),
			|_, _| None,
		);
		assert_eq!(resolved.deps.len(), 1);
		assert_eq!(resolved.total_dep_size, 0);
	}
}
