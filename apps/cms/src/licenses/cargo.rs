//! What this program itself is built out of.
//!
//! `cargo metadata` already resolves the whole tree and reports where each crate was
//! unpacked, so the registry cache is read through it rather than reconstructed from
//! `~/.cargo/registry/src`. That path is a cache layout, not an interface, and a crate whose
//! directory name does not match `{name}-{version}` -- a git or path dependency -- would be
//! silently missed by a reconstruction.

use super::{Found, author_name, purl};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Metadata {
	packages: Vec<CratePackage>,
}

#[derive(Debug, Deserialize)]
struct CratePackage {
	name: String,
	version: String,
	#[serde(default)]
	license: Option<String>,
	#[serde(default)]
	authors: Vec<String>,
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

	Ok(
		metadata
			.packages
			.into_iter()
			.filter(|package| package.source.is_some())
			.map(|package| Found {
				purl: purl("cargo", None, &package.name, &package.version),
				spdx: package.license.clone(),
				authors: package
					.authors
					.iter()
					.filter_map(|entry| author_name(entry))
					.collect(),
				// The manifest's directory is the crate root, which is where a licence sits.
				directory: package
					.manifest_path
					.parent()
					.map(Path::to_path_buf)
					.unwrap_or_default(),
			})
			.collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn keeps_registry_crates_and_drops_the_workspace_own() {
		let metadata: Metadata = serde_json::from_value(serde_json::json!({
			"packages": [
				{
					"name": "cms",
					"version": "0.1.0",
					"license": "MIT",
					"authors": [],
					"manifest_path": "/repo/apps/cms/Cargo.toml",
					"source": null
				},
				{
					"name": "serde",
					"version": "1.0.219",
					"license": "MIT OR Apache-2.0",
					"authors": ["Ada <ada@example.com>"],
					"manifest_path": "/cache/serde-1.0.219/Cargo.toml",
					"source": "registry+https://github.com/rust-lang/crates.io-index"
				}
			]
		}))
		.unwrap();

		let found: Vec<Found> = metadata
			.packages
			.into_iter()
			.filter(|package| package.source.is_some())
			.map(|package| Found {
				purl: purl("cargo", None, &package.name, &package.version),
				spdx: package.license.clone(),
				authors: package
					.authors
					.iter()
					.filter_map(|entry| author_name(entry))
					.collect(),
				directory: package
					.manifest_path
					.parent()
					.map(Path::to_path_buf)
					.unwrap_or_default(),
			})
			.collect();

		assert_eq!(found.len(), 1);
		assert_eq!(found[0].purl, "pkg:cargo/serde@1.0.219");
		assert_eq!(found[0].authors, ["Ada"]);
		assert_eq!(found[0].directory, PathBuf::from("/cache/serde-1.0.219"));
	}
}
