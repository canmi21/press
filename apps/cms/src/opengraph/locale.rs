//! The nine views a card is rendered for.
//!
//! The codes here are the site's internal ones, because they are what `?lang=` carries and
//! therefore what a stored card has to be keyed by. They are deliberately not BCP-47 tags: two
//! of them (`mw`, `tw`) are not, and conflating the two vocabularies is the mistake
//! spec/locale.md exists to prevent.
//!
//! `mw` is the source view -- the article in its own mixed language -- and has no translation
//! to look up. It is also what a request with no `?lang=` resolves to, so it is rendered first
//! and is the fallback when a locale has no card of its own.

/// The source view, and the answer when `?lang=` is absent or unknown.
pub const SOURCE: &str = "mw";

/// One view, as the site names it and as the sidecar stores it.
pub struct View {
	/// The site's internal code. This is the `?lang=` value and the storage prefix.
	pub code: &'static str,
	/// The tag translations are filed under, or `None` for the source view.
	pub tag: Option<&'static str>,
}

/// Every view, source first.
///
/// The eight targets match `i18n::prompt::LOCALES`; a ninth entry exists here because the
/// source article is a view somebody can be served even though nothing translated it.
pub const VIEWS: [View; 9] = [
	View { code: SOURCE, tag: None },
	View { code: "en", tag: Some("en-US") },
	View { code: "zh", tag: Some("zh-CN") },
	View { code: "tw", tag: Some("zh-TW") },
	View { code: "ja", tag: Some("ja-JP") },
	View { code: "ko", tag: Some("ko-KR") },
	View { code: "de", tag: Some("de-DE") },
	View { code: "fr", tag: Some("fr-FR") },
	View { code: "es", tag: Some("es-ES") },
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::i18n::prompt;

	#[test]
	fn every_translated_locale_has_a_view() {
		// The two lists answer different questions -- what gets translated, what gets rendered --
		// and a locale present in one and missing from the other is a card nobody notices is
		// stale, in a language nobody here reads.
		for locale in prompt::LOCALES {
			assert!(
				VIEWS.iter().any(|view| view.tag == Some(locale)),
				"{locale} is translated but has no card"
			);
		}
		assert_eq!(VIEWS.len(), prompt::LOCALES.len() + 1);
	}

	#[test]
	fn the_source_view_is_first_and_carries_no_tag() {
		// It is the fallback, so nothing may make it depend on a translation existing.
		assert_eq!(VIEWS[0].code, SOURCE);
		assert!(VIEWS[0].tag.is_none());
		assert!(VIEWS.iter().skip(1).all(|view| view.tag.is_some()));
	}
}
