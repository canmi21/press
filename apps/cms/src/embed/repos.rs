//! What a GitHub repository looked like when the article was last built.
//!
//! One request per repository, unauthenticated, which is enough for public metadata and keeps
//! this free of a token that would then have to reach CI. A rate limit reached mid-run costs
//! the repositories not yet read, never the ones already recorded.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repo {
	/// `owner/name`, as the article writes it.
	pub full_name: String,
	pub description: Option<String>,
	pub language: Option<String>,
	pub stars: u64,
	pub forks: u64,
	pub open_issues: u64,
	pub license: Option<String>,
	/// ISO 8601, as GitHub reports it.
	pub pushed_at: Option<String>,
}

/// The subset of GitHub's reply this reads.
///
/// Named rather than taken whole: the response carries around a hundred fields, and deserialising
/// into a shape that says which eight are used documents the dependency better than a `Value`.
#[derive(Debug, Deserialize)]
pub struct Response {
	pub full_name: String,
	pub description: Option<String>,
	pub language: Option<String>,
	#[serde(default)]
	pub stargazers_count: u64,
	#[serde(default)]
	pub forks_count: u64,
	#[serde(default)]
	pub open_issues_count: u64,
	#[serde(default)]
	pub license: Option<License>,
	pub pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct License {
	pub spdx_id: Option<String>,
}

impl From<Response> for Repo {
	fn from(response: Response) -> Self {
		Self {
			full_name: response.full_name,
			description: response.description,
			language: response.language,
			stars: response.stargazers_count,
			forks: response.forks_count,
			open_issues: response.open_issues_count,
			// `NOASSERTION` is GitHub's way of saying it could not identify the licence, which is
			// not a licence name and should not be printed as one.
			license: response
				.license
				.and_then(|license| license.spdx_id)
				.filter(|id| id != "NOASSERTION"),
			pushed_at: response.pushed_at,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn only_the_fields_the_card_shows_are_read() {
		// GitHub's reply carries around a hundred fields. Deserialising into a named shape is
		// what makes the eight that matter visible to the next reader.
		let json = r#"{
			"full_name": "canmi21/seam",
			"description": "a protocol",
			"language": "Rust",
			"stargazers_count": 12,
			"forks_count": 3,
			"open_issues_count": 1,
			"license": { "spdx_id": "MIT" },
			"pushed_at": "2026-08-01T00:00:00Z",
			"watchers": 12,
			"network_count": 3
		}"#;
		let repo: Repo = serde_json::from_str::<Response>(json)
			.expect("parse")
			.into();
		assert_eq!(repo.full_name, "canmi21/seam");
		assert_eq!(repo.stars, 12);
		assert_eq!(repo.license.as_deref(), Some("MIT"));
	}

	#[test]
	fn an_unidentified_licence_is_absent_rather_than_named_noassertion() {
		let json = r#"{
			"full_name": "a/b",
			"description": null,
			"language": null,
			"license": { "spdx_id": "NOASSERTION" },
			"pushed_at": null
		}"#;
		let repo: Repo = serde_json::from_str::<Response>(json)
			.expect("parse")
			.into();
		assert_eq!(repo.license, None);
		assert_eq!(repo.stars, 0);
	}
}
