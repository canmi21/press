//! The UI copy a card borrows from the pages.
//!
//! A card is a page in picture form, so its words are the page's words: read straight out of
//! `apps/site/messages/{view}.json`, the catalogs paraglide compiles for the site. Writing them
//! again here would mean two sets of nine translations that agree only until one is edited.
//!
//! Only `{name}` interpolation is supported, which is all these messages use -- there is no
//! plural machinery in the catalogs, so there is none to reimplement.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn catalog_path(repo: &Path, view: &str) -> PathBuf {
	repo
		.join("apps")
		.join("site")
		.join("messages")
		.join(format!("{view}.json"))
}

/// Every message for one view, or an empty map when the catalog cannot be read.
pub fn load(repo: &Path, view: &str) -> BTreeMap<String, String> {
	std::fs::read_to_string(catalog_path(repo, view))
		.ok()
		.and_then(|text| serde_json::from_str(&text).ok())
		.unwrap_or_default()
}

/// Every view's catalogue, read once for a whole run.
///
/// The nine files do not change while `cms og` is running -- they are build input, and this
/// command does not write them -- so reading one per card meant reading nine files several
/// thousand times to get nine answers. Loading them up front turns that into nine reads.
pub fn load_all(repo: &Path) -> BTreeMap<&'static str, BTreeMap<String, String>> {
	super::locale::VIEWS
		.iter()
		.map(|view| (view.code, load(repo, view.code)))
		.collect()
}

/// The catalogue for one view, or an empty one when that view has none.
///
/// Borrowed from the set above rather than looked up by path, so a caller cannot reach past it
/// and read the file again.
pub fn for_view<'a>(
	catalogs: &'a BTreeMap<&'static str, BTreeMap<String, String>>,
	view: &str,
) -> &'a BTreeMap<String, String> {
	static EMPTY: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
	catalogs
		.get(view)
		.unwrap_or_else(|| EMPTY.get_or_init(BTreeMap::new))
}

/// Fill `{slot}` from the pairs given, leaving anything unnamed as it stands.
///
/// An unknown slot survives rather than being blanked: a message with a typo in it should be
/// visible on the card that carries it, not silently rendered as a hole.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
	let mut out = template.to_owned();
	for (name, value) in values {
		out = out.replace(&format!("{{{name}}}"), value);
	}
	out
}

/// A count as a card should show it: thousands as `128k`, below that as itself.
///
/// Compact rather than grouped, which sidesteps the separator question entirely -- a card is
/// read at a glance and `128k` is the same three characters in every locale, while `128,000`
/// and `128.000` mean opposite things depending on who is reading.
pub fn compact(count: usize) -> String {
	match count {
		0..=999 => count.to_string(),
		1_000..=999_999 => format!("{}k", count / 1_000),
		_ => format!("{}M", count / 1_000_000),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fills_the_slots_a_message_names() {
		assert_eq!(
			fill("{a} articles · {b} words", &[("a", "4"), ("b", "128k")]),
			"4 articles · 128k words"
		);
	}

	#[test]
	fn leaves_an_unknown_slot_visible() {
		assert_eq!(fill("{a} and {oops}", &[("a", "1")]), "1 and {oops}");
	}

	#[test]
	fn shortens_a_count_without_choosing_a_separator() {
		assert_eq!(compact(4), "4");
		assert_eq!(compact(999), "999");
		assert_eq!(compact(128_400), "128k");
		assert_eq!(compact(2_500_000), "2M");
	}

	#[test]
	fn reads_the_catalog_the_site_reads() {
		// The repository's own catalogs, so this fails if the path moves or a view loses its
		// file -- either of which would silently drop a card back to its slots.
		let repo = crate::paths::repo_root().expect("repo");
		for view in super::super::locale::VIEWS {
			let catalog = load(&repo, view.code);
			// Every key the renderer looks up, not a sample of them. A missing one resolves to
			// an empty string and draws a card with a blank where a fact should be -- which is
			// exactly what happened to `card.packages`, unnoticed until somebody looked at the
			// picture. The slots are the other half of that contract: a message that loses one
			// renders without the number it was supposed to carry.
			for (key, slots) in [
				(
					"card.stats",
					["{articles}", "{characters}", "{languages}"].as_slice(),
				),
				("card.languages", ["{count}"].as_slice()),
				("card.packages", ["{count}"].as_slice()),
				("card.more_licenses", ["{count}"].as_slice()),
				("card.licenses", [].as_slice()),
				("card.by_registry", [].as_slice()),
				("card.from_registry", ["{count}"].as_slice()),
				("card.under_license", ["{count}"].as_slice()),
			] {
				let message = catalog.get(key);
				assert!(message.is_some(), "{} has no {key}", view.code);
				let text = message.expect("checked above");
				for slot in slots {
					assert!(text.contains(slot), "{}: {key} has no {slot}", view.code);
				}
			}
		}
	}
}
