//! What this program itself is built out of.
//!
//! `cargo metadata` already resolves the whole tree and reports where each crate was
//! unpacked, so the registry cache is read through it rather than reconstructed from
//! `~/.cargo/registry/src`. That path is a cache layout, not an interface, and a crate whose
//! directory name does not match `{name}-{version}` -- a git or path dependency -- would be
//! silently missed by a reconstruction.

use super::{Found, author, prefer_origin, purl, web_url};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Metadata {
	packages: Vec<CratePackage>,
	#[serde(default)]
	workspace_members: Vec<String>,
	#[serde(default)]
	resolve: Option<Resolve>,
}

#[derive(Debug, Deserialize)]
struct Resolve {
	#[serde(default)]
	nodes: Vec<ResolveNode>,
}

#[derive(Debug, Deserialize)]
struct ResolveNode {
	id: String,
	#[serde(default)]
	dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CratePackage {
	id: String,
	name: String,
	version: String,
	#[serde(default)]
	license: Option<String>,
	#[serde(default)]
	authors: Vec<String>,
	#[serde(default)]
	description: Option<String>,
	#[serde(default)]
	homepage: Option<String>,
	#[serde(default)]
	documentation: Option<String>,
	#[serde(default)]
	repository: Option<String>,
	manifest_path: PathBuf,
	#[serde(default)]
	source: Option<String>,
}

/// Every crate the workspace resolves from a registry.
///
/// Crates with no `source` are the workspace's own and are excluded: the credit list is for
/// other people's work. Nothing filters on dev-dependencies, because a binary nobody
/// distributes has no meaningful line between what it ships and what it tests with -- the
/// whole tree is what it was built out of.
pub fn collect(repo: &Path) -> Result<Vec<Found>, String> {
	let output = Command::new("cargo")
		.current_dir(repo)
		.args(["metadata", "--format-version", "1"])
		.output()
		.map_err(|error| format!("could not run cargo: {error}"))?;
	if !output.status.success() {
		return Err(format!(
			"cargo metadata failed: {}",
			String::from_utf8_lossy(&output.stderr).trim()
		));
	}
	let metadata: Metadata = serde_json::from_slice(&output.stdout)
		.map_err(|error| format!("could not read cargo metadata: {error}"))?;

	Ok(from_metadata(metadata))
}

fn found(package: CratePackage) -> Found {
	Found {
		purl: purl("cargo", None, &package.name, &package.version),
		spdx: package.license,
		authors: package.authors.iter().filter_map(|entry| author(entry)).collect(),
		description: package.description,
		homepage: web_url(package.homepage),
		documentation: web_url(package.documentation),
		repository: web_url(package.repository),
		origins: BTreeMap::new(),
		dependents: BTreeSet::new(),
		// The manifest's directory is the crate root, which is where a licence sits.
		directory: package.manifest_path.parent().map(Path::to_path_buf).unwrap_or_default(),
	}
}

fn from_metadata(metadata: Metadata) -> Vec<Found> {
	let labels: BTreeMap<String, String> = metadata
		.packages
		.iter()
		.map(|package| {
			let label = match package.source.as_ref() {
				Some(_) => purl("cargo", None, &package.name, &package.version),
				None => format!("workspace:{}", package.name),
			};
			(package.id.clone(), label)
		})
		.collect();
	let package_purls: BTreeMap<String, String> = metadata
		.packages
		.iter()
		.filter(|package| package.source.is_some())
		.map(|package| (package.id.clone(), purl("cargo", None, &package.name, &package.version)))
		.collect();
	let root_names: BTreeMap<String, String> = metadata
		.packages
		.iter()
		.filter(|package| metadata.workspace_members.contains(&package.id))
		.map(|package| (package.id.clone(), package.name.clone()))
		.collect();
	let mut found: BTreeMap<String, Found> = metadata
		.packages
		.into_iter()
		.filter(|package| package.source.is_some())
		.map(found)
		.map(|package| (package.purl.clone(), package))
		.collect();

	let graph: BTreeMap<String, Vec<String>> = metadata
		.resolve
		.into_iter()
		.flat_map(|resolve| resolve.nodes)
		.map(|mut node| {
			node.dependencies.sort();
			(node.id, node.dependencies)
		})
		.collect();
	collect_dependents(&graph, &labels, &package_purls, &mut found);

	for root in metadata.workspace_members {
		let Some(root_name) = root_names.get(&root) else {
			continue;
		};
		trace_origins(&root, root_name, &graph, &labels, &package_purls, &mut found);
	}

	found.into_values().collect()
}

/// Every reverse edge in the resolved graph, read straight off it.
///
/// Deliberately not folded into `trace_origins`: that walk keeps only the shortest path to
/// each crate and prunes every edge that does not improve one, so the edges it discards are
/// exactly the second and third crate that also depend on something. Those are the answer
/// here, not noise.
fn collect_dependents(
	graph: &BTreeMap<String, Vec<String>>,
	labels: &BTreeMap<String, String>,
	package_purls: &BTreeMap<String, String>,
	found: &mut BTreeMap<String, Found>,
) {
	for (parent, dependencies) in graph {
		let Some(label) = labels.get(parent) else {
			continue;
		};
		for dependency in dependencies {
			let Some(purl) = package_purls.get(dependency) else {
				continue;
			};
			// A crate reached from itself is a feature-resolution artefact, not a dependency
			// anybody can act on.
			if purl == label {
				continue;
			}
			if let Some(package) = found.get_mut(purl) {
				package.dependents.insert(label.clone());
			}
		}
	}
}

fn trace_origins(
	root: &str,
	root_name: &str,
	graph: &BTreeMap<String, Vec<String>>,
	labels: &BTreeMap<String, String>,
	package_purls: &BTreeMap<String, String>,
	found: &mut BTreeMap<String, Found>,
) {
	let mut best = BTreeMap::from([(root.to_owned(), Vec::<String>::new())]);
	let mut queue = VecDeque::from([root.to_owned()]);
	while let Some(parent) = queue.pop_front() {
		let path = best.get(&parent).cloned().unwrap_or_default();
		for dependency in graph.get(&parent).into_iter().flatten() {
			let Some(label) = labels.get(dependency) else {
				continue;
			};
			let mut candidate = path.clone();
			candidate.push(label.clone());
			let improves = match best.get(dependency) {
				None => true,
				Some(current) => {
					candidate.len() < current.len()
						|| (candidate.len() == current.len() && candidate < *current)
				}
			};
			if !improves {
				continue;
			}
			best.insert(dependency.clone(), candidate.clone());
			if let Some(purl) = package_purls.get(dependency)
				&& let Some(package) = found.get_mut(purl)
			{
				prefer_origin(&mut package.origins, root_name, candidate);
			}
			queue.push_back(dependency.clone());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn keeps_registry_crates_and_drops_the_workspace_own() {
		let metadata: Metadata = serde_json::from_value(serde_json::json!({
			"workspace_members": ["cms 0.1.0 (path+file:///repo/apps/cms)"],
			"packages": [
				{
					"id": "cms 0.1.0 (path+file:///repo/apps/cms)",
					"name": "cms",
					"version": "0.1.0",
					"license": "MIT",
					"authors": [],
					"manifest_path": "/repo/apps/cms/Cargo.toml",
					"source": null
				},
				{
					"id": "serde 1.0.219 (registry+https://github.com/rust-lang/crates.io-index)",
					"name": "serde",
					"version": "1.0.219",
					"license": "MIT OR Apache-2.0",
					"authors": ["Ada <ada@example.com>"],
					"manifest_path": "/cache/serde-1.0.219/Cargo.toml",
					"source": "registry+https://github.com/rust-lang/crates.io-index"
				}
			],
			"resolve": {
				"nodes": [
					{
						"id": "cms 0.1.0 (path+file:///repo/apps/cms)",
						"dependencies": ["serde 1.0.219 (registry+https://github.com/rust-lang/crates.io-index)"]
					},
					{
						"id": "serde 1.0.219 (registry+https://github.com/rust-lang/crates.io-index)",
						"dependencies": []
					}
				]
			}
		}))
		.unwrap();

		let found = from_metadata(metadata);

		assert_eq!(found.len(), 1);
		assert_eq!(found[0].purl, "pkg:cargo/serde@1.0.219");
		assert_eq!(found[0].authors[0].name, "Ada");
		assert_eq!(found[0].origins["cms"], ["pkg:cargo/serde@1.0.219"]);
		assert_eq!(found[0].dependents.iter().collect::<Vec<_>>(), ["workspace:cms"]);
		assert_eq!(found[0].directory, PathBuf::from("/cache/serde-1.0.219"));
	}
}
