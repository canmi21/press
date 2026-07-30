//! The network half. Kept apart from `parse` so the decisions stay testable without a socket.

use std::sync::OnceLock;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);
/// Only the head is ever read, and a page that buries its icon links past this is
/// misconfigured. The cap matters because some sites stream megabytes of inlined markup.
const HTML_LIMIT: usize = 100_000;
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
			.user_agent(USER_AGENT)
			.build()
			.into()
	})
}

// Identifies the crawler and points at the site, so anyone reading their logs can tell what
// this is. The URL is a citation for a human, not something this program resolves.
const USER_AGENT: &str = "Mozilla/5.0 (compatible; favicon/1.0; +https://canmi.net)";

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
	response
		.body_mut()
		.with_config()
		.limit(HTML_LIMIT as u64)
		.read_to_string()
		.ok()
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
	fn maps_types_to_extensions() {
		assert_eq!(extension_for("image/svg+xml"), Some("svg"));
		assert_eq!(extension_for("image/png"), Some("png"));
		assert_eq!(extension_for("image/jpeg"), Some("jpg"));
		assert_eq!(extension_for("image/x-icon"), Some("ico"));
	}
}
