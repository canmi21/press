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
use claude_codes::{AsyncClient, ClaudeModel, ClaudeOutput, cli::ClaudeCliBuilder};

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
}

impl std::fmt::Display for Refusal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Failed(reason) | Self::Exhausted(reason) => write!(f, "{reason}"),
		}
	}
}

pub async fn ask(runner: Runner, prompt: &str, model: &str) -> Result<Answer, Refusal> {
	if runner.uses_agy() {
		agy(runner, prompt, model).await
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

/// Whether a refusal means the allowance is gone rather than the request being bad.
///
/// Matched on the message because that is the only place the distinction appears: the exit
/// code is 1 either way, and the envelope's status is `ERROR` for both a spent quota and a
/// malformed request.
fn is_exhausted(message: &str) -> bool {
	let lower = message.to_ascii_lowercase();
	lower.contains("quota") || lower.contains("rate limit") || lower.contains("resets in")
}

async fn agy(runner: Runner, prompt: &str, model: &str) -> Result<Answer, Refusal> {
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
		return Err(if is_exhausted(&reason) {
			Refusal::Exhausted(reason)
		} else {
			Refusal::Failed(reason)
		});
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

/// The Claude enum, for callers that still want it typed.
pub fn claude_model(name: &str) -> ClaudeModel {
	match name {
		"haiku" => ClaudeModel::Haiku,
		"opus" => ClaudeModel::Opus,
		_ => ClaudeModel::Sonnet,
	}
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
	fn each_runner_names_its_own_provider() {
		assert_eq!(Runner::Claude.provider(), "anthropic");
		assert_eq!(Runner::Gemini.provider(), "google");
	}
}
