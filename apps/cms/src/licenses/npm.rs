//! What the Worker apps install from npm.
//!
//! Read through pnpm rather than by walking `node_modules`: the store holds every version any
//! workspace member ever asked for, including the build toolchain, and the question here is
//! narrower -- what does each deployable actually ship. `--prod` answers exactly that.

use super::{Found, Person, author, prefer_origin, purl, web_url};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace packages that carry a Worker.
///
/// Named rather than globbed. `apps/cms` is this program and has no npm side, and a directory
/// appearing under `apps/` is not by itself a thing that gets deployed.
pub const APPS: [&str; 3] = ["api", "cdn", "site"];

#[derive(Debug, Deserialize)]
struct Project {
	#[serde(default)]
	dependencies: BTreeMap<String, Node>,
}

#[derive(Debug, Deserialize)]
struct Node {
	#[serde(default)]
	version: String,
	#[serde(default)]
	path: Option<PathBuf>,
	#[serde(default)]
	dependencies: BTreeMap<String, Node>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
	#[serde(default)]
	license: Option<serde_json::Value>,
	#[serde(default)]
	author: Option<serde_json::Value>,
	#[serde(default)]
	description: Option<String>,
	#[serde(default)]
	homepage: Option<String>,
	#[serde(default)]
	repository: Option<serde_json::Value>,
}

struct Declared {
	spdx: Option<String>,
	authors: Vec<Person>,
	description: Option<String>,
	homepage: Option<String>,
	repository: Option<String>,
}

/// Every third-party package in the production closure of the Worker apps, deduplicated.
///
/// Workspace libraries are excluded. pnpm links them, so their version reads `link:../..`,
/// which is both the reliable marker and the accurate statement: they are this project, not
/// something it credits.
///
/// Packages resolved for another platform are excluded too, and this is the reason the result
/// is a list rather than a count of what pnpm printed. A tree contains every optional binary
/// for every operating system -- `lightningcss-win32-x64-msvc`, `@sentry/cli-linux-arm` -- and
/// only the one matching this machine is ever installed. The rest have no directory to read,
/// have never been built with, and are not in any Worker bundle. Reporting them as declaring
/// no licence would be false, and the count of genuinely undeclared packages is a number that
/// has to stay small enough to act on.
pub fn collect(repo: &Path) -> Result<Vec<Found>, String> {
	let mut packages: BTreeMap<String, Found> = BTreeMap::new();

	for app in APPS {
		let output = Command::new("pnpm")
			.current_dir(repo)
			.args([
				"--filter",
				&format!("@canmi/{app}"),
				"list",
				"--prod",
				"--depth",
				"Infinity",
				"--json",
			])
			.output()
			.map_err(|error| format!("could not run pnpm: {error}"))?;
		if !output.status.success() {
			return Err(format!(
				"pnpm list failed for {app}: {}",
				String::from_utf8_lossy(&output.stderr).trim()
			));
		}
		let projects: Vec<Project> = serde_json::from_slice(&output.stdout)
			.map_err(|error| format!("could not read pnpm output for {app}: {error}"))?;
		for project in &projects {
			walk(&project.dependencies, &mut packages, app, &[]);
		}
	}

	Ok(packages.into_values().collect())
}

fn walk(
	nodes: &BTreeMap<String, Node>,
	into: &mut BTreeMap<String, Found>,
	root: &str,
	path: &[String],
) {
	for (name, node) in nodes {
		// A linked workspace package. Its own dependencies are still worth walking, because a
		// library the Workers use pulls third-party code in with it.
		if node.version.starts_with("link:") {
			let mut next = path.to_vec();
			next.push(format!("workspace:{name}"));
			walk(&node.dependencies, into, root, &next);
			continue;
		}
		let Some(directory) = node.path.clone() else {
			continue;
		};
		let (namespace, bare) = split_scope(name);
		let id = purl("npm", namespace, bare, &node.version);
		// The same package reached twice through different parents is one entry. Reading its
		// manifest only on the first arrival keeps a wide tree from re-reading the same file
		// dozens of times.
		if let std::collections::btree_map::Entry::Vacant(slot) = into.entry(id.clone()) {
			// No manifest means the package was resolved but never installed here, which is
			// what an optional binary for another platform looks like. It is left out rather
			// than recorded as declaring nothing.
			if let Some(declared) = declared(&directory) {
				slot.insert(Found {
					purl: id.clone(),
					spdx: declared.spdx,
					authors: declared.authors,
					description: declared.description,
					homepage: declared.homepage,
					documentation: None,
					repository: declared.repository,
					origins: BTreeMap::new(),
					dependents: BTreeSet::new(),
					directory,
				});
			}
		}
		let Some(found) = into.get_mut(&id) else {
			continue;
		};
		// Whatever stands one step back on the path is what pulled this package in. An empty
		// path means the app itself asked for it, which is a dependent worth naming rather
		// than a gap: a package the deployable depends on directly is a different fact from
		// one that only arrived through a library.
		let parent = path
			.last()
			.cloned()
			.unwrap_or_else(|| format!("workspace:{root}"));
		found.dependents.insert(parent);

		let mut next = path.to_vec();
		next.push(id);
		prefer_origin(&mut found.origins, root, next.clone());
		walk(&node.dependencies, into, root, &next);
	}
}

/// `@scope/name` becomes a namespace and a name; anything else is a bare name.
fn split_scope(name: &str) -> (Option<&str>, &str) {
	match name.split_once('/') {
		Some((scope, bare)) if scope.starts_with('@') => (Some(scope), bare),
		_ => (None, name),
	}
}

/// The licence expression and author a package declares in its manifest.
///
/// `None` means the manifest could not be read at all, which is a different fact from a
/// manifest that declares no licence -- the first is a package that is not installed, the
/// second is a package somebody has to make a decision about.
fn declared(directory: &Path) -> Option<Declared> {
	let bytes = std::fs::read(directory.join("package.json")).ok()?;
	let manifest = serde_json::from_slice::<Manifest>(&bytes).ok()?;
	Some(Declared {
		spdx: manifest.license.as_ref().and_then(license_expression),
		authors: manifest
			.author
			.as_ref()
			.and_then(person)
			.into_iter()
			.collect(),
		description: nonempty_text(manifest.description),
		homepage: web_url(manifest.homepage),
		repository: manifest.repository.as_ref().and_then(repository_url),
	})
}

/// npm's `license` field, which has outlived two of its own formats.
///
/// The modern form is an SPDX string. The 2013-era object form is still in the wild and still
/// says which licence it is, so it is read rather than discarded -- dropping it would report a
/// package as undeclared when it plainly declared something.
fn license_expression(value: &serde_json::Value) -> Option<String> {
	match value {
		serde_json::Value::String(text) => Some(text.clone()),
		serde_json::Value::Object(map) => map.get("type")?.as_str().map(str::to_owned),
		_ => None,
	}
}

/// npm's `author`, which is either a string or a `{ name, email, url }` object.
fn person(value: &serde_json::Value) -> Option<Person> {
	match value {
		serde_json::Value::String(text) => author(text),
		serde_json::Value::Object(map) => {
			let name = map.get("name")?.as_str()?;
			let email = map
				.get("email")
				.and_then(serde_json::Value::as_str)
				.unwrap_or_default();
			let url = map
				.get("url")
				.and_then(serde_json::Value::as_str)
				.unwrap_or_default();
			author(&format!("{name} <{email}> ({url})"))
		}
		_ => None,
	}
}

fn nonempty_text(value: Option<String>) -> Option<String> {
	value
		.map(|text| text.trim().to_owned())
		.filter(|text| !text.is_empty())
}

/// The repository field's legacy shorthands, reduced to a URL a browser can open.
fn repository_url(value: &serde_json::Value) -> Option<String> {
	let raw = match value {
		serde_json::Value::String(text) => text.as_str(),
		serde_json::Value::Object(map) => map.get("url")?.as_str()?,
		_ => return None,
	}
	.trim();
	if raw.is_empty() {
		return None;
	}

	let mut url = raw.strip_prefix("git+").unwrap_or(raw).to_owned();
	if let Some(path) = url.strip_prefix("github:") {
		url = format!("https://github.com/{path}");
	} else if let Some(path) = url.strip_prefix("git@github.com:") {
		url = format!("https://github.com/{path}");
	} else if let Some(path) = url.strip_prefix("ssh://git@github.com/") {
		url = format!("https://github.com/{path}");
	} else if let Some(path) = url.strip_prefix("git://") {
		url = format!("https://{path}");
	}
	if !url.starts_with("https://") && !url.starts_with("http://") {
		return None;
	}
	Some(url.strip_suffix(".git").unwrap_or(&url).to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn splits_a_scope_off_a_package_name() {
		assert_eq!(split_scope("@sveltejs/kit"), (Some("@sveltejs"), "kit"));
		assert_eq!(split_scope("hono"), (None, "hono"));
		// Not a scope: no leading `@`, so the slash is part of the name rather than a boundary.
		assert_eq!(split_scope("a/b"), (None, "a/b"));
	}

	#[test]
	fn reads_both_shapes_of_the_license_field() {
		assert_eq!(
			license_expression(&serde_json::json!("MIT OR Apache-2.0")).as_deref(),
			Some("MIT OR Apache-2.0")
		);
		assert_eq!(
			license_expression(&serde_json::json!({ "type": "MIT", "url": "https://example.com" }))
				.as_deref(),
			Some("MIT")
		);
		assert_eq!(license_expression(&serde_json::json!(null)), None);
	}

	#[test]
	fn reads_both_shapes_of_the_author_field() {
		assert_eq!(
			person(&serde_json::json!(
				"Ada <ada@example.com> (https://example.com)"
			))
			.map(|person| person.name),
			Some("Ada".to_owned())
		);
		assert_eq!(
			person(&serde_json::json!({
				"name": "Ada",
				"email": "123+octocat@users.noreply.github.com"
			})),
			Some(Person {
				name: "Ada".to_owned(),
				github: Some("octocat".to_owned()),
			})
		);
	}

	#[test]
	fn turns_repository_shorthands_into_browser_urls() {
		for value in [
			serde_json::json!("git+https://github.com/owner/project.git"),
			serde_json::json!({ "type": "git", "url": "github:owner/project" }),
			serde_json::json!("git@github.com:owner/project.git"),
		] {
			assert_eq!(
				repository_url(&value).as_deref(),
				Some("https://github.com/owner/project")
			);
		}
	}

	/// A package directory with a manifest, the way an installed package looks.
	fn installed(root: &Path, name: &str, manifest: serde_json::Value) -> PathBuf {
		let directory = root.join(name);
		std::fs::create_dir_all(&directory).unwrap();
		std::fs::write(
			directory.join("package.json"),
			serde_json::to_vec(&manifest).unwrap(),
		)
		.unwrap();
		directory
	}

	#[test]
	fn walks_past_a_linked_workspace_package_into_its_dependencies() {
		let root = std::env::temp_dir().join(format!("cms-npm-walk-{}", std::process::id()));
		let hono = installed(&root, "hono", serde_json::json!({ "license": "MIT" }));

		let nodes = BTreeMap::from([(
			"@canmi/urls".to_owned(),
			Node {
				version: "link:../../libs/urls".to_owned(),
				path: Some(root.join("urls")),
				dependencies: BTreeMap::from([(
					"hono".to_owned(),
					Node {
						version: "4.12.34".to_owned(),
						path: Some(hono),
						dependencies: BTreeMap::new(),
					},
				)]),
			},
		)]);
		let mut found = BTreeMap::new();
		walk(&nodes, &mut found, "site", &[]);
		assert_eq!(found.keys().collect::<Vec<_>>(), ["pkg:npm/hono@4.12.34"]);
		assert_eq!(
			found["pkg:npm/hono@4.12.34"].origins["site"],
			["workspace:@canmi/urls", "pkg:npm/hono@4.12.34"]
		);
		assert_eq!(
			found["pkg:npm/hono@4.12.34"]
				.dependents
				.iter()
				.collect::<Vec<_>>(),
			["workspace:@canmi/urls"]
		);

		std::fs::remove_dir_all(&root).unwrap();
	}

	/// The origin path keeps one route in; the dependents keep every package that asked.
	#[test]
	fn records_every_parent_of_a_package_two_of_them_share() {
		let root = std::env::temp_dir().join(format!("cms-npm-dependents-{}", std::process::id()));
		let shared = installed(&root, "shared", serde_json::json!({ "license": "MIT" }));
		let middle = installed(&root, "middle", serde_json::json!({ "license": "MIT" }));

		let leaf = |path: &PathBuf| Node {
			version: "1.0.0".to_owned(),
			path: Some(path.clone()),
			dependencies: BTreeMap::new(),
		};
		let nodes = BTreeMap::from([
			("shared".to_owned(), leaf(&shared)),
			(
				"middle".to_owned(),
				Node {
					version: "2.0.0".to_owned(),
					path: Some(middle),
					dependencies: BTreeMap::from([("shared".to_owned(), leaf(&shared))]),
				},
			),
		]);
		let mut found = BTreeMap::new();
		walk(&nodes, &mut found, "site", &[]);

		// The app depends on it directly, and so does the package beside it.
		assert_eq!(
			found["pkg:npm/shared@1.0.0"]
				.dependents
				.iter()
				.collect::<Vec<_>>(),
			["pkg:npm/middle@2.0.0", "workspace:site"]
		);
		// The shortest path is still the direct one, which is why the second parent needs
		// somewhere else to be said.
		assert_eq!(
			found["pkg:npm/shared@1.0.0"].origins["site"],
			["pkg:npm/shared@1.0.0"]
		);

		std::fs::remove_dir_all(&root).unwrap();
	}

	#[test]
	fn leaves_out_a_package_resolved_for_another_platform() {
		let root = std::env::temp_dir().join(format!("cms-npm-optional-{}", std::process::id()));
		let here = installed(&root, "here", serde_json::json!({ "license": "MIT" }));

		let nodes = BTreeMap::from([
			(
				"here".to_owned(),
				Node {
					version: "1.0.0".to_owned(),
					path: Some(here),
					dependencies: BTreeMap::new(),
				},
			),
			(
				// Resolved by pnpm, never installed on this machine: no directory at all.
				"lightningcss-win32-x64-msvc".to_owned(),
				Node {
					version: "1.33.0".to_owned(),
					path: Some(root.join("lightningcss-win32-x64-msvc")),
					dependencies: BTreeMap::new(),
				},
			),
		]);
		let mut found = BTreeMap::new();
		walk(&nodes, &mut found, "site", &[]);
		assert_eq!(found.keys().collect::<Vec<_>>(), ["pkg:npm/here@1.0.0"]);

		std::fs::remove_dir_all(&root).unwrap();
	}
}
