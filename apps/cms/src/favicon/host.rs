//! Turning whatever the caller has into hostnames worth fetching.
//!
//! The caller is a person typing domains today and the article pipeline handing over every
//! link it found tomorrow. Both give a messy mix -- bare domains, full URLs, duplicates,
//! things that are not sites at all -- so normalising and filtering belongs here rather than
//! in each caller.

use std::collections::BTreeSet;
use url::Url;

/// Hostnames from a mixed list of domains and URLs: deduplicated, lowercased, sorted.
///
/// Sorted rather than input-ordered so that running twice over the same article produces the
/// same sequence of requests, which makes a failure reproducible.
pub fn normalise<I, S>(inputs: I) -> Vec<String>
where
	I: IntoIterator<Item = S>,
	S: AsRef<str>,
{
	inputs
		.into_iter()
		.filter_map(|raw| hostname(raw.as_ref()))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}

/// The hostname a single input refers to, or None if it does not name a fetchable site.
pub fn hostname(raw: &str) -> Option<String> {
	let trimmed = raw.trim();
	if trimmed.is_empty() {
		return None;
	}

	let candidate = if trimmed.contains("://") {
		Url::parse(trimmed).ok()?.host_str()?.to_lowercase()
	} else if let Ok(url) = Url::parse(&format!("https://{trimmed}")) {
		url.host_str()?.to_lowercase()
	} else {
		return None;
	};

	is_fetchable(&candidate).then_some(candidate)
}

/// Whether a hostname is worth a request at all.
///
/// Rejects bare names, localhost and bare IPv4. None of those identify a public site, and a
/// build that quietly tried to fetch `localhost` would behave differently on every machine.
pub fn is_fetchable(host: &str) -> bool {
	if host.len() > 253 || host == "localhost" {
		return false;
	}
	let labels: Vec<&str> = host.split('.').collect();
	if labels.len() < 2 {
		return false;
	}
	if labels.iter().all(|label| label.parse::<u8>().is_ok()) && labels.len() == 4 {
		return false;
	}
	labels.iter().all(|label| {
		!label.is_empty()
			&& label.len() <= 63
			&& !label.starts_with('-')
			&& !label.ends_with('-')
			&& label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
	})
}

/// Resolve an icon href against the page it was found on, honouring `<base href>`.
///
/// A page that declares a base and serves its icons from a CDN is otherwise resolved against
/// the wrong origin, which fetches a 404 from the site itself.
pub fn absolute(domain: &str, base: Option<&str>, href: &str) -> Option<String> {
	let page = Url::parse(&format!("https://{domain}/")).ok()?;
	let root = match base {
		Some(base) => page.join(base).unwrap_or(page),
		None => page,
	};
	root.join(href).ok().map(Into::into)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accepts_a_bare_domain() {
		assert_eq!(hostname("example.com").as_deref(), Some("example.com"));
	}

	#[test]
	fn pulls_the_host_out_of_a_url() {
		assert_eq!(
			hostname("https://blog.example.com/a/b?c=1#d").as_deref(),
			Some("blog.example.com")
		);
	}

	#[test]
	fn lowercases_and_trims() {
		assert_eq!(hostname("  Example.COM  ").as_deref(), Some("example.com"));
	}

	#[test]
	fn rejects_things_that_are_not_public_sites() {
		assert_eq!(hostname("localhost"), None);
		assert_eq!(hostname("127.0.0.1"), None);
		assert_eq!(hostname("nodots"), None);
		assert_eq!(hostname(""), None);
		assert_eq!(hostname("   "), None);
	}

	#[test]
	fn rejects_malformed_labels() {
		assert!(!is_fetchable("-lead.example.com"));
		assert!(!is_fetchable("trail-.example.com"));
		assert!(!is_fetchable("a..example.com"));
	}

	#[test]
	fn keeps_a_hostname_that_merely_starts_with_digits() {
		// 4-label all-numeric is an IP; this is not.
		assert!(is_fetchable("1.example.com"));
	}

	#[test]
	fn deduplicates_across_forms() {
		let out = normalise(["https://example.com/a", "example.com", "EXAMPLE.com/b"]);
		assert_eq!(out, vec!["example.com"]);
	}

	#[test]
	fn sorts_so_repeat_runs_match() {
		let out = normalise(["https://b.example.com", "a.example.com"]);
		assert_eq!(out, vec!["a.example.com", "b.example.com"]);
	}

	#[test]
	fn drops_unusable_entries_without_failing_the_batch() {
		// One bad link in an article must not stop the other icons being fetched.
		let out = normalise(["example.com", "not a url", "localhost", "other.example.com"]);
		assert_eq!(out, vec!["example.com", "other.example.com"]);
	}

	#[test]
	fn resolves_a_relative_href_against_the_page() {
		assert_eq!(
			absolute("example.com", None, "/icon.png").as_deref(),
			Some("https://example.com/icon.png")
		);
	}

	#[test]
	fn honours_a_base_href() {
		assert_eq!(
			absolute(
				"example.com",
				Some("https://cdn.example.net/assets/"),
				"icon.png"
			)
			.as_deref(),
			Some("https://cdn.example.net/assets/icon.png")
		);
	}

	#[test]
	fn leaves_an_absolute_href_alone() {
		assert_eq!(
			absolute("example.com", None, "https://cdn.other.net/i.svg").as_deref(),
			Some("https://cdn.other.net/i.svg")
		);
	}
}
