//! Naming a model the same way twice.
//!
//! Two families order their parts differently and neither is going to change, so the shape is
//! recorded rather than guessed: Anthropic names a variant then a version, OpenAI names a
//! version then a variant. Dots become hyphens throughout, so `4.5` is `4-5`.
//!
//! The identity is read from the response envelope wherever the runner reports one, because a
//! model's account of itself is unreliable in a way that a runtime fact is not. `anonymous`
//! exists for providers that report nothing; it is wired up to nothing yet and is here so the
//! next provider is a function to fill rather than a shape to invent.

/// Who ran it.
///
/// Unused while every runner reports its provider directly. Kept, with the rules below, so
/// that adding a provider which reports nothing is a function to call rather than a
/// convention to reconstruct -- see `anonymous`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
	Anthropic,
	Openai,
	Alibaba,
	Deepseek,
}

#[allow(dead_code)]
impl Provider {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Anthropic => "anthropic",
			Self::Openai => "openai",
			Self::Alibaba => "alibaba",
			Self::Deepseek => "deepseek",
		}
	}

	/// The family whose names this provider issues.
	pub fn family(self) -> &'static str {
		match self {
			Self::Anthropic => "claude",
			Self::Openai => "gpt",
			Self::Alibaba => "qwen",
			Self::Deepseek => "deepseek",
		}
	}

	pub fn of_model(model: &str) -> Option<Self> {
		match model.split('-').next()? {
			"claude" => Some(Self::Anthropic),
			"gpt" => Some(Self::Openai),
			"qwen" => Some(Self::Alibaba),
			"deepseek" => Some(Self::Deepseek),
			_ => None,
		}
	}
}

/// Every model id this repository knows how to name.
///
/// Enumerated rather than derived, because the two families disagree about part order and no
/// pattern covers both without also matching nonsense. A runner reporting something absent
/// from this list still gets recorded -- see `normalise` -- but only what is listed here is a
/// name this project claims to understand.
#[allow(dead_code)]
pub const KNOWN: [&str; 22] = [
	// anthropic: family, variant, version
	"claude-opus-5",
	"claude-opus-4-8",
	"claude-opus-4-7",
	"claude-sonnet-5",
	"claude-sonnet-4-5",
	"claude-haiku-4-5",
	"claude-fable-5",
	// openai: family, version, variant
	"gpt-5",
	"gpt-5-2",
	"gpt-5-sol",
	"gpt-5-6-sol",
	"gpt-5-6-terra",
	"gpt-5-6-luna",
	// alibaba
	"qwen-3",
	"qwen-2-5",
	"qwen-2-5-max",
	"qwen-3-235b-a22b",
	// deepseek
	"deepseek-v3",
	"deepseek-r1",
	"deepseek-coder-v2",
	"deepseek-prover-v2",
	// the light Claude, kept last so the list reads by provider
	"claude-haiku-4-6",
];

/// A trailing dated snapshot, which runners append and which is not part of the identity.
///
/// `claude-haiku-4-5-20251001` is `claude-haiku-4-5` pinned to a build. Recording the date
/// would make two runs of the same model look like two models, which is exactly the question
/// this field exists to answer.
fn strip_snapshot(id: &str) -> &str {
	match id.rsplit_once('-') {
		Some((head, tail)) if tail.len() >= 6 && tail.chars().all(|c| c.is_ascii_digit()) => head,
		_ => id,
	}
}

/// Bring a raw model string to the recorded form.
///
/// Spelling is settled first -- lower case, dots and underscores to hyphens -- then a dated
/// snapshot suffix is removed. What survives is checked against `KNOWN` only to answer whether
/// this project recognises it; an unrecognised id is still stored as it arrived, because
/// inventing a name for it would be worse than recording an unfamiliar one.
pub fn normalise(raw: &str) -> String {
	let spelled = raw.trim().to_ascii_lowercase().replace(['.', '_'], "-");
	strip_snapshot(&spelled).to_owned()
}

/// Whether this is an id the naming rules above account for.
#[allow(dead_code)]
pub fn known(id: &str) -> bool {
	KNOWN.contains(&id)
}

/// The rule as the prompt states it, for a provider that has to be asked.
///
/// Unused today: every runner here reports its own model. Kept whole so that adding one that
/// does not is a matter of calling `anonymous` rather than working the convention out again.
#[allow(dead_code)]
pub const NAMING_RULE: &str = "\
Report the model that produced this answer on a single line, as `provider/model`.

  anthropic/claude-{variant}-{version}   claude-sonnet-5, claude-haiku-4-5, claude-opus-4-8
  openai/gpt-{version}[-{variant}]       gpt-5, gpt-5-2, gpt-5-sol, gpt-5-6-terra
  alibaba/qwen-{version}[-{variant}]     qwen-3, qwen-2-5, qwen-2-5-max, qwen-3-235b-a22b
  deepseek/deepseek-{variant}            deepseek-v3, deepseek-r1, deepseek-coder-v2

Anthropic names the variant before the version; OpenAI names the version before the variant.
Write every dot as a hyphen, and use lower case throughout.";

/// Read a `provider/model` line from a reply.
///
/// The entry point for a provider that cannot be asked any other way. Nothing calls it yet.
#[allow(dead_code)]
pub fn anonymous(reply: &str) -> Option<(String, String)> {
	let line = reply
		.lines()
		.map(str::trim)
		.find(|line| line.contains('/') && !line.contains(' '))?;
	let (provider, model) = line.split_once('/')?;
	let model = normalise(model);
	// Believed only as far as it agrees with itself: a model claiming to be a Claude while
	// naming OpenAI is reporting nothing worth storing.
	let claimed = Provider::of_model(&model)?;
	if claimed.as_str() != provider.trim().to_ascii_lowercase() {
		return None;
	}
	Some((claimed.as_str().to_owned(), model))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dots_become_hyphens() {
		assert_eq!(normalise("Claude-Sonnet-4.5"), "claude-sonnet-4-5");
		assert_eq!(normalise("qwen-2.5-max"), "qwen-2-5-max");
	}

	#[test]
	fn a_dated_snapshot_is_not_a_different_model() {
		// Runners pin a build and report it. Storing the date would make two runs of one model
		// look like two models, which is the question this field exists to answer.
		assert_eq!(normalise("claude-haiku-4-5-20251001"), "claude-haiku-4-5");
		assert_eq!(normalise("gpt-5-6-terra-20260214"), "gpt-5-6-terra");
	}

	#[test]
	fn a_version_is_not_mistaken_for_a_date() {
		// Short trailing digits are part of the name; only a long run of them is a snapshot.
		assert_eq!(normalise("claude-sonnet-5"), "claude-sonnet-5");
		assert_eq!(normalise("gpt-5-2"), "gpt-5-2");
		assert_eq!(normalise("qwen-3-235b-a22b"), "qwen-3-235b-a22b");
	}

	#[test]
	fn the_known_list_holds_what_the_rules_describe() {
		assert!(known("claude-haiku-4-5"));
		assert!(known("gpt-5-6-luna"));
		assert!(known("deepseek-prover-v2"));
		// Recognition is not a gate: an unfamiliar id is still recorded as it arrived.
		assert!(!known("claude-something-9"));
		assert_eq!(normalise("Claude-Something-9"), "claude-something-9");
	}

	#[test]
	fn the_two_families_order_their_parts_differently() {
		// Worth a test because it is the thing most likely to be silently normalised into one
		// shape by somebody tidying up later.
		assert!(NAMING_RULE.contains("claude-{variant}-{version}"));
		assert!(NAMING_RULE.contains("gpt-{version}[-{variant}]"));
	}

	#[test]
	fn a_provider_is_recognised_from_its_family() {
		assert_eq!(
			Provider::of_model("claude-opus-4-8"),
			Some(Provider::Anthropic)
		);
		assert_eq!(Provider::of_model("gpt-5-6-luna"), Some(Provider::Openai));
		assert_eq!(
			Provider::of_model("qwen-3-235b-a22b"),
			Some(Provider::Alibaba)
		);
		assert_eq!(
			Provider::of_model("deepseek-prover-v2"),
			Some(Provider::Deepseek)
		);
		assert_eq!(Provider::of_model("something-else"), None);
	}

	#[test]
	fn a_self_report_is_read_when_it_is_consistent() {
		assert_eq!(
			anonymous("here you go\nanthropic/claude-sonnet-5\n"),
			Some(("anthropic".to_owned(), "claude-sonnet-5".to_owned()))
		);
	}

	#[test]
	fn a_self_report_that_contradicts_itself_is_discarded() {
		// Recording "openai" beside a Claude name would be worse than recording nothing: it
		// reads as a fact and is not one.
		assert_eq!(anonymous("openai/claude-sonnet-5"), None);
		assert_eq!(anonymous("no slash here"), None);
	}
}
