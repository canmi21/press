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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
	Anthropic,
	Openai,
	Alibaba,
	Deepseek,
}

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

/// Bring a raw model string to the recorded form.
///
/// Lowercased, dots to hyphens, and anything a runner appends that is not part of the identity
/// -- a dated snapshot suffix, most often -- left alone rather than invented away. Whatever
/// arrives is what is stored; this only settles spelling.
pub fn normalise(raw: &str) -> String {
	raw.trim().to_ascii_lowercase().replace(['.', '_'], "-")
}

/// The rule as the prompt states it, for a provider that has to be asked.
///
/// Unused today: every runner here reports its own model. Kept whole so that adding one that
/// does not is a matter of calling `anonymous` rather than working the convention out again.
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
