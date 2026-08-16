//! The network half. Kept apart from `parse` so the decisions stay testable without a socket.

use std::sync::OnceLock;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);
/// How much of a page is parsed. Only the head is ever read, and a page that buries its icon
/// links past this is misconfigured.
const HTML_PARSE_LIMIT: usize = 100_000;
/// How much is read off the wire before giving up. Separate from the parse limit because
/// ureq's `limit` *rejects* an oversized body rather than truncating it, so using the parse
/// limit here discards every page larger than it -- which is how GitHub's homepage silently
/// fell through to /favicon.ico and lost its dark variant.
const HTML_READ_LIMIT: usize = 5_000_000;
/// Generous for an icon; the point is to refuse a mislabelled video, not to be strict.
const ICON_LIMIT: usize = 2_000_000;

pub struct Fetched {
	pub bytes: Vec<u8>,
	pub content_type: String,
}

fn agent() -> &'static ureq::Agent {
	static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
	AGENT.get_or_init(|| {
		ureq::Agent::config_builder()
			.timeout_global(Some(TIMEOUT))
			.user_agent(user_agent())
			.build()
			.into()
	})
}

// Identifies the crawler and points at the site, so anyone reading their logs can tell what
// this is. The URL is a citation for a human, not something this program resolves -- but it
// is still the site's address, so it comes from the map rather than being a second copy.
fn user_agent() -> String {
	format!(
		"Mozilla/5.0 (compatible; favicon/1.0; +{})",
		crate::urls::APPS_PRODUCTION_SITE
	)
}

/// The site's home page, or None for anything that is not reachable HTML.
pub fn html(domain: &str) -> Option<String> {
	let mut response = agent().get(format!("https://{domain}/")).call().ok()?;
	if !response.status().is_success() {
		return None;
	}
	let content_type = header(&response, "content-type").unwrap_or_default();
	if !content_type.to_lowercase().contains("text/html") {
		return None;
	}
	let body = response
		.body_mut()
		.with_config()
		.limit(HTML_READ_LIMIT as u64)
		.read_to_string()
		.ok()?;
	Some(truncate_on_char_boundary(body, HTML_PARSE_LIMIT))
}

fn truncate_on_char_boundary(mut body: String, limit: usize) -> String {
	if body.len() <= limit {
		return body;
	}
	let mut end = limit;
	while end > 0 && !body.is_char_boundary(end) {
		end -= 1;
	}
	body.truncate(end);
	body
}

pub fn bytes(url: &str) -> Option<Fetched> {
	let mut response = agent().get(url).call().ok()?;
	if !response.status().is_success() {
		return None;
	}
	let header_type = header(&response, "content-type");
	let bytes = response
		.body_mut()
		.with_config()
		.limit(ICON_LIMIT as u64)
		.read_to_vec()
		.ok()?;
	if bytes.is_empty() {
		return None;
	}
	Some(Fetched {
		content_type: infer_content_type(url, header_type.as_deref()),
		bytes,
	})
}

fn header<T>(response: &ureq::http::Response<T>, name: &str) -> Option<String> {
	response
		.headers()
		.get(name)
		.and_then(|value| value.to_str().ok())
		.map(str::to_owned)
}

/// Trust the server's `Content-Type` only when it claims an image. Plenty of hosts serve
/// icons as `application/octet-stream` or `text/plain`, so the extension is the better
/// signal in those cases.
pub fn infer_content_type(url: &str, header: Option<&str>) -> String {
	if let Some(header) = header {
		let cleaned = header
			.split(';')
			.next()
			.unwrap_or_default()
			.trim()
			.to_lowercase();
		if cleaned.starts_with("image/") {
			return cleaned;
		}
	}
	let path = url
		.split(['?', '#'])
		.next()
		.unwrap_or_default()
		.to_lowercase();
	match () {
		() if path.ends_with(".svg") => "image/svg+xml",
		() if path.ends_with(".png") => "image/png",
		() if path.ends_with(".jpg") || path.ends_with(".jpeg") => "image/jpeg",
		() if path.ends_with(".ico") => "image/x-icon",
		() => "application/octet-stream",
	}
	.to_owned()
}

/// The file extension to store an icon under, or None for something we should not keep.
pub fn extension_for(content_type: &str) -> Option<&'static str> {
	let lower = content_type.to_lowercase();
	if lower.contains("svg") {
		Some("svg")
	} else if lower.contains("png") {
		Some("png")
	} else if lower.contains("jpeg") || lower.contains("jpg") {
		Some("jpg")
	} else if lower.contains("icon") {
		Some("ico")
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn believes_a_server_that_claims_an_image() {
		assert_eq!(infer_content_type("/x", Some("image/png")), "image/png");
	}

	#[test]
	fn strips_charset_from_the_header() {
		assert_eq!(
			infer_content_type("/x", Some("image/svg+xml; charset=utf-8")),
			"image/svg+xml"
		);
	}

	#[test]
	fn falls_back_to_the_extension_when_the_header_is_useless() {
		// Serving an icon as octet-stream is common enough that trusting the header blindly
		// would store a lot of files under the wrong name.
		assert_eq!(
			infer_content_type(
				"https://a.example/icon.svg",
				Some("application/octet-stream")
			),
			"image/svg+xml"
		);
		assert_eq!(
			infer_content_type("https://a.example/icon.ico", None),
			"image/x-icon"
		);
	}

	#[test]
	fn ignores_query_strings_when_reading_the_extension() {
		assert_eq!(
			infer_content_type("https://a.example/i.png?v=2", None),
			"image/png"
		);
	}

	#[test]
	fn gives_up_on_an_unrecognised_type() {
		assert_eq!(
			infer_content_type("https://a.example/i", None),
			"application/octet-stream"
		);
		assert_eq!(extension_for("application/octet-stream"), None);
	}

	#[test]
	fn truncates_a_long_body_instead_of_discarding_it() {
		// ureq's own limit rejects rather than truncates. Relying on it dropped every page
		// over the limit, which is how GitHub's homepage lost its dark icon.
		let body = "a".repeat(200);
		assert_eq!(truncate_on_char_boundary(body, 100).len(), 100);
	}

	#[test]
	fn never_splits_a_multibyte_character() {
		// Cutting at a fixed byte offset in the middle of a character would panic on
		// truncate; a page of CJK text hits this immediately.
		let body = "中".repeat(10); // 3 bytes each
		let out = truncate_on_char_boundary(body, 10);
		assert_eq!(out.chars().count(), 3);
		assert_eq!(out.len(), 9);
	}

	#[test]
	fn leaves_a_short_body_alone() {
		assert_eq!(truncate_on_char_boundary("short".into(), 100), "short");
	}

	#[test]
	fn maps_types_to_extensions() {
		assert_eq!(extension_for("image/svg+xml"), Some("svg"));
		assert_eq!(extension_for("image/png"), Some("png"));
		assert_eq!(extension_for("image/jpeg"), Some("jpg"));
		assert_eq!(extension_for("image/x-icon"), Some("ico"));
	}
}
