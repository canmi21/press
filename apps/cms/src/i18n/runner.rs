//! Who does the translating.
//!
//! Two agents with the same shape: a local binary taking one prompt non-interactively and
//! printing a JSON envelope. Neither is an API client, so there is no key here and no request
//! to assemble -- the difference between them is which envelope to read.
//!
//! Claude reports the model that actually ran; `agy` does not, so for Gemini what was asked
//! for is what gets recorded. That is a weaker fact and it is worth knowing which one you
//! have. See spec/i18n.md.

use super::model;
use super::segment::Kind;
use claude_codes::{AsyncClient, ClaudeOutput, cli::ClaudeCliBuilder};

/// Which agent a run uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runner {
	Claude,
	Gemini,
	/// Open-weight GPT, served through the same `agy` binary as Gemini.
	GptOss,
}

impl Runner {
	pub fn parse(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().as_str() {
			"claude" => Some(Self::Claude),
			"gemini" | "agy" => Some(Self::Gemini),
			"gpt-oss" | "oss" => Some(Self::GptOss),
			_ => None,
		}
	}

	pub fn provider(self) -> &'static str {
		match self {
			Self::Claude => "anthropic",
			Self::Gemini => "google",
			// The weights are OpenAI's; agy is only the road it arrives by.
			Self::GptOss => "openai",
		}
	}

	/// The model for a block of this kind on this attempt.
	///
	/// Three tiers either way, chosen by what the block is and escalated only after a failure
	/// has actually happened. Gemini spells effort into the model id rather than taking it as
	/// a separate flag, so a tier is one string here as it is there.
	pub fn model_for(self, kind: Kind, attempt: usize) -> &'static str {
		match self {
			Self::Claude => match (kind.is_light(), attempt) {
				(true, 0) => "haiku",
				(_, 0 | 1) => "sonnet",
				_ => "opus",
			},
			Self::Gemini => match (kind.is_light(), attempt) {
				(true, 0) => "gemini-3.6-flash-medium",
				(_, 0 | 1) => "gemini-3.6-flash-high",
				_ => "gemini-3.1-pro-high",
			},
			// One size offered, so every tier is the same string. Named per tier anyway, so
			// that adding a second is a table edit rather than a restructure.
			Self::GptOss => "gpt-oss-120b-medium",
		}
	}

	/// Whether this runner is driven by the `agy` binary.
	pub fn uses_agy(self) -> bool {
		matches!(self, Self::Gemini | Self::GptOss)
	}
}

/// One completed turn, however it was produced.
pub struct Answer {
	pub text: String,
	pub model: String,
	pub tokens: u64,
	pub usd: f64,
}

/// Why a turn did not produce an answer.
#[derive(Debug)]
pub enum Refusal {
	/// This one went wrong. Another may still work.
	Failed(String),
	/// There is no point asking again: the allowance is spent until it resets.
	Exhausted(String),
	/// Too fast, not too much. The capacity comes back on its own in seconds.
	Throttled(String),
}

impl std::fmt::Display for Refusal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Failed(reason) | Self::Exhausted(reason) | Self::Throttled(reason) => {
				write!(f, "{reason}")
			}
		}
	}
}

pub async fn ask(runner: Runner, prompt: &str, model: &str) -> Result<Answer, Refusal> {
	if runner.uses_agy() {
		agy(prompt, model).await
	} else {
		claude(prompt, model).await
	}
}

async fn claude(prompt: &str, model: &str) -> Result<Answer, Refusal> {
	let builder = ClaudeCliBuilder::new()
		.model(model)
		// Nothing is read and nothing is written; the whole task is in the prompt.
		.allowed_tools(Vec::<String>::new());

	let mut client = AsyncClient::from_builder(builder)
		.await
		.map_err(|error| Refusal::Failed(error.to_string()))?;
	let outputs = client
		.query(prompt)
		.await
		.map_err(|error| Refusal::Failed(error.to_string()))?;
	let _ = client.shutdown().await;

	let result = outputs
		.iter()
		.find_map(|output| match output {
			ClaudeOutput::Result(message) => Some(message),
			_ => None,
		})
		.ok_or_else(|| Refusal::Failed("the CLI ended without a result".to_owned()))?;
	if result.is_error {
		return Err(Refusal::Failed(format!("{:?}", result.subtype)));
	}

	let usage = result.usage.as_ref();
	Ok(Answer {
		text: result.result.clone().unwrap_or_default(),
		// Read from the envelope rather than assumed: what ran is a runtime fact.
		model: result
			.model_usage
			.as_ref()
			.and_then(|usage| usage.keys().next())
			.map(|name| model::normalise(name))
			.unwrap_or_else(|| model::normalise(model)),
		tokens: usage.map_or(0, |u| {
			u64::from(u.input_tokens)
				+ u64::from(u.output_tokens)
				+ u64::from(u.cache_read_input_tokens)
				+ u64::from(u.cache_creation_input_tokens)
		}),
		usd: result.total_cost_usd,
	})
}

/// What `agy --output-format json` prints.
///
/// Parsing this is safe in a way that asking a model for JSON is not: the CLI emits it, so a
/// defect here is a bug in a program rather than a sentence that came out wrong.
#[derive(serde::Deserialize)]
struct AgyEnvelope {
	status: String,
	#[serde(default)]
	response: String,
	#[serde(default)]
	error: String,
	#[serde(default)]
	usage: AgyUsage,
}

#[derive(serde::Deserialize, Default)]
struct AgyUsage {
	#[serde(default)]
	total_tokens: u64,
}

/// How long until the runner says it will work again.
///
/// The reset is the only thing that separates the two refusals worth telling apart, and both
/// spell it the same way: `Resets in 0s`, `Resets in 167h29m42s`.
fn resets_in(message: &str) -> Option<std::time::Duration> {
	let at = message.to_ascii_lowercase().find("resets in ")? + "resets in ".len();
	let tail: String = message[at..]
		.chars()
		.take_while(|c| c.is_ascii_alphanumeric())
		.collect();

	let mut seconds = 0u64;
	let mut value = 0u64;
	for c in tail.chars() {
		match c {
			'0'..='9' => value = value * 10 + u64::from(c as u8 - b'0'),
			'h' => {
				seconds += value * 3600;
				value = 0;
			}
			'm' => {
				seconds += value * 60;
				value = 0;
			}
			's' => {
				seconds += value;
				value = 0;
			}
			_ => return None,
		}
	}
	Some(std::time::Duration::from_secs(seconds))
}

/// Anything coming back sooner than this is congestion rather than a spent allowance.
///
/// The two look alike -- exit code 1, status ERROR, a sentence about capacity -- and only the
/// reset separates them. Minutes mean the runner is asking to be asked again; hours mean the
/// account is out until it is not.
const BRIEF: std::time::Duration = std::time::Duration::from_secs(15 * 60);

fn classify(message: &str) -> Refusal {
	let lower = message.to_ascii_lowercase();
	let capacity = lower.contains("quota")
		|| lower.contains("rate limit")
		|| lower.contains("capacity")
		|| lower.contains("resets in");
	if !capacity {
		return Refusal::Failed(message.to_owned());
	}
	// "retryable" is the provider saying so itself, and a short reset says the same thing.
	if lower.contains("retryable") || resets_in(message).is_some_and(|d| d < BRIEF) {
		Refusal::Throttled(message.to_owned())
	} else {
		Refusal::Exhausted(message.to_owned())
	}
}

async fn agy(prompt: &str, model: &str) -> Result<Answer, Refusal> {
	let output = tokio::process::Command::new("agy")
		.arg("--print")
		.arg(prompt)
		.arg("--model")
		.arg(model)
		.arg("--output-format")
		.arg("json")
		// Nothing here needs a tool, and this runs unattended over a whole library.
		.arg("--disable-slash-commands")
		// Set inside an agent session, and inherited by anything spawned from one. Left in
		// place, the child refuses to start.
		.env_remove("CLAUDECODE")
		.env_remove("CLAUDE_CODE_ENTRYPOINT")
		.output()
		.await
		.map_err(|error| Refusal::Failed(format!("could not run agy: {error}")))?;

	// Read before the exit code is consulted. agy exits 1 on a spent quota and still prints a
	// full envelope saying so, with the reset time in it; trusting the status first threw that
	// away and reported an empty "exit status: 1" instead.
	let envelope: Option<AgyEnvelope> = serde_json::from_slice(&output.stdout).ok();

	let Some(envelope) = envelope else {
		let stderr = String::from_utf8_lossy(&output.stderr);
		return Err(Refusal::Failed(format!(
			"agy exited {} with no envelope: {}",
			output.status,
			stderr.trim().chars().take(160).collect::<String>()
		)));
	};

	if envelope.status != "SUCCESS" {
		let reason = if envelope.error.is_empty() {
			format!("agy reported {}", envelope.status)
		} else {
			envelope.error.clone()
		};
		return Err(classify(&reason));
	}

	Ok(Answer {
		text: envelope.response,
		// The envelope names no model, so what was asked for is what is recorded. Weaker than
		// Claude's answer, where the runtime says what actually ran.
		model: model::normalise(model),
		tokens: envelope.usage.total_tokens,
		// Not reported. Left at zero rather than estimated, so a run's cost is either measured
		// or visibly absent.
		usd: 0.0,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_runner_is_named_the_way_a_person_would_type_it() {
		assert_eq!(Runner::parse("claude"), Some(Runner::Claude));
		assert_eq!(Runner::parse("Gemini"), Some(Runner::Gemini));
		// The binary is `agy`, and somebody will reach for that name.
		assert_eq!(Runner::parse("agy"), Some(Runner::Gemini));
		assert_eq!(Runner::parse("gpt"), None);
	}

	#[test]
	fn both_runners_have_the_same_three_tiers() {
		for runner in [Runner::Claude, Runner::Gemini] {
			let light = runner.model_for(Kind::Heading, 0);
			let standard = runner.model_for(Kind::Prose, 0);
			let strong = runner.model_for(Kind::Prose, 2);
			assert_ne!(light, standard);
			assert_ne!(standard, strong);
		}
	}

	#[test]
	fn gemini_spells_effort_into_the_model_id() {
		// `agy` takes --effort separately, but its model list already carries the tier, so one
		// string says both and there is no second flag to keep in step.
		assert_eq!(
			Runner::Gemini.model_for(Kind::Prose, 0),
			"gemini-3.6-flash-high"
		);
		assert_eq!(
			Runner::Gemini.model_for(Kind::Prose, 2),
			"gemini-3.1-pro-high"
		);
	}

	#[test]
	fn a_gemini_model_normalises_to_the_recorded_spelling() {
		// Dots to hyphens, like every other id this project stores.
		assert_eq!(
			model::normalise(Runner::Gemini.model_for(Kind::Heading, 0)),
			"gemini-3-6-flash-medium"
		);
	}

	#[test]
	fn a_reset_time_is_read_from_the_message() {
		use std::time::Duration;
		assert_eq!(resets_in("Resets in 0s."), Some(Duration::from_secs(0)));
		assert_eq!(resets_in("Resets in 90s."), Some(Duration::from_secs(90)));
		assert_eq!(
			resets_in("Resets in 167h29m42s."),
			Some(Duration::from_secs(167 * 3600 + 29 * 60 + 42))
		);
		assert_eq!(resets_in("no reset mentioned"), None);
	}

	#[test]
	fn congestion_and_a_spent_allowance_are_told_apart() {
		// They look alike: exit 1, status ERROR, a sentence about capacity. Only the reset
		// separates them, and getting it wrong means either giving up an hour early or
		// hammering a dead account for the rest of the week.
		let busy = "Encountered retryable error from model provider: You have exhausted your \
		            capacity on this model. Resets in 0s.";
		let spent = "Individual quota reached. Please upgrade your subscription to increase \
		             your limits. Resets in 167h29m42s.";
		assert!(matches!(classify(busy), Refusal::Throttled(_)));
		assert!(matches!(classify(spent), Refusal::Exhausted(_)));
		// Anything not about capacity stays an ordinary failure.
		assert!(matches!(classify("malformed request"), Refusal::Failed(_)));
	}

	#[test]
	fn each_runner_names_its_own_provider() {
		assert_eq!(Runner::Claude.provider(), "anthropic");
		assert_eq!(Runner::Gemini.provider(), "google");
	}
}
