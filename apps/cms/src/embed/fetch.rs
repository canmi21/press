//! The network half of the embeds. Kept apart from the resolvers so the decisions stay
//! testable without a socket, the same split `favicon` uses.

use super::crates::{Crate, IndexEntry};
use super::repos::{Repo, Response};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(15);

/// Identifies the build and points at the site. crates.io asks for this and answers 403 without
/// one, so it is a requirement here rather than a courtesy.
const USER_AGENT: &str = "canmi-workspace-cms (+https://canmi.net)";

fn agent() -> &'static ureq::Agent {
	static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
	AGENT.get_or_init(|| {
		ureq::Agent::config_builder()
			.timeout_global(Some(TIMEOUT))
			.user_agent(USER_AGENT)
			.build()
			.into()
	})
}

/// Every published version of a crate, as the sparse index lists them.
///
/// One JSON object per line rather than one document, so a version that fails to parse costs
/// that version instead of the crate.
fn index(name: &str) -> Option<Vec<IndexEntry>> {
	let url = format!(
		"https://index.crates.io/{}",
		super::crates::index_path(name)
	);
	let mut response = agent().get(&url).call().ok()?;
	if !response.status().is_success() {
		return None;
	}
	let body = response.body_mut().read_to_string().ok()?;
	Some(
		body
			.lines()
			.filter(|line| !line.trim().is_empty())
			.filter_map(|line| serde_json::from_str::<IndexEntry>(line).ok())
			.collect(),
	)
}

/// The published archive size, which the index does not carry.
fn crate_size(name: &str, version: &str) -> Option<u64> {
	let url = format!("https://crates.io/api/v1/crates/{name}/{version}");
	let mut response = agent().get(&url).call().ok()?;
	if !response.status().is_success() {
		return None;
	}
	let text = response.body_mut().read_to_string().ok()?;
	let body: serde_json::Value = serde_json::from_str(&text).ok()?;
	body.pointer("/version/crate_size")?.as_u64()
}

/// Resolve one crate's tree, reusing index reads across the whole walk.
///
/// A dependency graph revisits the same crates constantly -- `serde` appears under nearly
/// everything -- so without the cache a single card would fetch the same file dozens of times
/// from a public index that is not ours.
pub fn krate(name: &str) -> Option<Crate> {
	let mut cache: HashMap<String, Option<Vec<IndexEntry>>> = HashMap::new();
	let root_entries = index(name)?;
	let root = super::crates::newest(&root_entries)?.clone();
	Some(super::crates::resolve(
		&root,
		|wanted| {
			cache
				.entry(wanted.to_lowercase())
				.or_insert_with(|| index(wanted))
				.clone()
		},
		crate_size,
	))
}

/// Public repository metadata, unauthenticated.
pub fn repo(full_name: &str) -> Option<Repo> {
	let url = format!("https://api.github.com/repos/{full_name}");
	let mut response = agent()
		.get(&url)
		.header("Accept", "application/vnd.github+json")
		.call()
		.ok()?;
	if !response.status().is_success() {
		return None;
	}
	let text = response.body_mut().read_to_string().ok()?;
	let parsed: Response = serde_json::from_str(&text).ok()?;
	Some(parsed.into())
}
