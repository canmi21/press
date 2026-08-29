//! Who does the translating.
//!
//! Local agents taking one prompt non-interactively. None is an API client, so there is no key
//! here and no request to assemble -- the difference between them is which command to run and
//! which envelope to read.
//!
//! Claude reports the model that actually ran; the others do not, so what was asked for is what
//! gets recorded. That is a weaker fact and it is worth knowing which one you have. See
//! spec/i18n.md.

use super::model;
use super::segment::Kind;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use claude_codes::{AsyncClient, ClaudeOutput, cli::ClaudeCliBuilder};
use std::ffi::OsString;
use std::path::Path;

/// Which agent a run uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runner {
	Claude,
	Gemini,
	/// Open-weight GPT, served through the same `agy` binary as Gemini.
	GptOss,
	Codex,
	Cursor,
	Grok,
}

/// Pure text stays on the open-weight model unless a runner is named explicitly.
pub const DEFAULT_TEXT: Runner = Runner::GptOss;

/// Tasks that need to see an image use Codex's balanced vision model by default.
pub const DEFAULT_VISION: Runner = Runner::Codex;

pub const CHOICES: &str = "claude, gemini, gpt-oss, codex, cursor, or grok";
pub const EFFORT_CHOICES: &str = "low, medium, high, xhigh, max, or ultra";

/// Build the concrete tier name stored by the CMS and split at the Codex boundary later.
///
/// The runner remains a separate choice because it determines the binary and response envelope.
/// A concrete model is useful at the bindings whose CLIs accept one: Codex, which takes model
/// and effort as independent settings, and Claude, whose CLI takes a model name alone -- so
/// `--effort` stays a Codex flag, and pinning opus is `--model claude --model-id opus`.
pub fn model_override(
	runner: Runner,
	model: Option<&str>,
	effort: Option<&str>,
) -> Result<Option<String>, String> {
	let Some(model) = model else {
		return if effort.is_some() {
			Err("--effort requires --model-id".to_owned())
		} else {
			Ok(None)
		};
	};
	if !matches!(runner, Runner::Codex | Runner::Claude) {
		return Err("--model-id requires --model codex or --model claude".to_owned());
	}
	let model = model.trim();
	if model.is_empty() || model.starts_with('-') {
		return Err("--model-id takes a model id".to_owned());
	}
	if runner == Runner::Claude {
		return if effort.is_some() {
			Err("--effort is a Codex setting; the Claude CLI takes only a model name".to_owned())
		} else {
			Ok(Some(model.to_owned()))
		};
	}
	let Some(effort) = effort else {
		return Ok(Some(model.to_owned()));
	};
	let effort = effort.trim().to_ascii_lowercase();
	if !matches!(
		effort.as_str(),
		"low" | "medium" | "high" | "xhigh" | "max" | "ultra"
	) {
		return Err(format!("--effort takes {EFFORT_CHOICES}"));
	}
	Ok(Some(format!("{model}-{effort}")))
}

impl Runner {
	pub fn parse(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().as_str() {
			"claude" => Some(Self::Claude),
			"gemini" | "agy" => Some(Self::Gemini),
			"gpt-oss" | "oss" => Some(Self::GptOss),
			"codex" => Some(Self::Codex),
			"cursor" | "cursor-agent" => Some(Self::Cursor),
			"grok" => Some(Self::Grok),
			_ => None,
		}
	}

	pub fn provider(self) -> &'static str {
		match self {
			Self::Claude => "anthropic",
			Self::Gemini => "google",
			// The weights are OpenAI's; agy is only the road it arrives by.
			Self::GptOss => "openai",
			Self::Codex => "openai",
			Self::Cursor => "cursor",
			Self::Grok => "xai",
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
			Self::Codex => match (kind.is_light(), attempt) {
				(true, 0) => "gpt-5.6-luna-medium",
				(_, 0 | 1) => "gpt-5.6-terra-medium",
				_ => "gpt-5.6-terra-high",
			},
			Self::Cursor => "composer-2.5",
			// Two models, mapped onto the same three arms. 4.5 is structural; 4.6 is prose
			// and the retry. There is no third model to escalate to.
			Self::Grok => match (kind.is_light(), attempt) {
				(true, 0) => "grok-4.5",
				_ => "grok-4.6",
			},
		}
	}

	/// The model for reading a whole article to decide what will need a note.
	///
	/// The strongest tier each runner offers, and unapologetically so. This is one call per
	/// article against dozens per article for translation, and the judgement it makes -- whether
	/// the surrounding text already carries a meaning -- is the one a cheap model cannot make.
	/// Getting it wrong is not a bad sentence but a note nobody needed, or a missing one nobody
	/// notices.
	pub fn model_for_scan(self) -> &'static str {
		match self {
			Self::Claude => "opus",
			Self::Gemini => "gemini-3.1-pro-high",
			Self::GptOss => "gpt-oss-120b-medium",
			Self::Codex => "gpt-5.6-sol-high",
			Self::Cursor => "composer-2.5",
			Self::Grok => "grok-4.6",
		}
	}

	/// The model for a task that involves looking at an image.
	///
	/// No tiering here. Reading a picture is the whole of the work and there is no structural
	/// signal to route on -- a photograph and a screenshot are equally a look, unlike a
	/// heading and a paragraph, which differ in what they can lose.
	pub fn model_for_vision(self) -> Option<&'static str> {
		match self {
			Self::Claude => Some("sonnet"),
			Self::Gemini => Some("gemini-3.6-flash-high"),
			Self::Codex => Some("gpt-5.6-terra-medium"),
			Self::Cursor => Some("composer-2.5"),
			// Text only. Asked to look at a file it cancels the turn and reports no error at
			// all, so a caller that tried anyway would see an empty answer and no reason for
			// it. Measured: the same model answers a text prompt in the same breath.
			Self::GptOss => None,
			// 4.6 is the prose model. A look is quality work with no structural signal, and
			// there is no third model to escalate to, so the one-shot is the better of the
			// two. 4.5 also sees -- measured -- but is the structural tier.
			Self::Grok => Some("grok-4.6"),
		}
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
	dispatch(runner, prompt, model, None).await
}

/// Ask a runner about an image, attaching the bytes when its CLI supports that directly.
pub async fn ask_vision(
	runner: Runner,
	prompt: &str,
	model: &str,
	image: &Path,
) -> Result<Answer, Refusal> {
	dispatch(runner, prompt, model, Some(image)).await
}

async fn dispatch(
	runner: Runner,
	prompt: &str,
	model: &str,
	image: Option<&Path>,
) -> Result<Answer, Refusal> {
	match runner {
		Runner::Claude => claude(prompt, model).await,
		Runner::Gemini | Runner::GptOss => agy(prompt, model).await,
		Runner::Codex => codex(prompt, model, image).await,
		Runner::Cursor => cursor(prompt, model).await,
		Runner::Grok => grok(prompt, model, image).await,
	}
}

async fn claude(prompt: &str, model: &str) -> Result<Answer, Refusal> {
	// Read, and nothing else. Some tasks name an image for the agent to open; none of them
	// have any business writing, and this runs unattended over a whole library.
	let builder = ClaudeCliBuilder::new().model(model).allowed_tools(["Read"]);

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

fn failed_command(binary: &str, output: &std::process::Output) -> Refusal {
	let stderr = String::from_utf8_lossy(&output.stderr);
	let stdout = String::from_utf8_lossy(&output.stdout);
	let detail = if stderr.trim().is_empty() {
		stdout.trim()
	} else {
		stderr.trim()
	};
	classify(&format!(
		"{binary} exited {}: {}",
		output.status,
		detail.chars().take(500).collect::<String>()
	))
}

/// Read the final message and billed tokens from `codex exec --json` JSONL.
fn codex_result(stdout: &[u8]) -> Result<(String, u64), Refusal> {
	let mut answer = None;
	let mut tokens = 0;
	for line in stdout
		.split(|byte| *byte == b'\n')
		.filter(|line| !line.is_empty())
	{
		let event: serde_json::Value = serde_json::from_slice(line)
			.map_err(|error| Refusal::Failed(format!("invalid codex event: {error}")))?;
		match event.get("type").and_then(serde_json::Value::as_str) {
			Some("item.completed")
				if event
					.pointer("/item/type")
					.and_then(serde_json::Value::as_str)
					== Some("agent_message") =>
			{
				answer = event
					.pointer("/item/text")
					.and_then(serde_json::Value::as_str)
					.map(str::to_owned);
			}
			Some("turn.completed") => {
				let input = event
					.pointer("/usage/input_tokens")
					.and_then(serde_json::Value::as_u64)
					.unwrap_or(0);
				let output = event
					.pointer("/usage/output_tokens")
					.and_then(serde_json::Value::as_u64)
					.unwrap_or(0);
				tokens = input + output;
			}
			_ => {}
		}
	}
	answer
		.map(|text| (text, tokens))
		.ok_or_else(|| Refusal::Failed("codex ended without a final message".to_owned()))
}

/// The reason codex gives for a turn it could not run, from its event stream.
///
/// Worth digging for because the useful message is on stdout while stderr carries only
/// "Reading additional input from stdin...", which is printed on success too. Reporting the
/// stream in exit-code order said nothing at all six times over.
fn codex_error(stdout: &[u8]) -> Option<String> {
	stdout
		.split(|byte| *byte == b'\n')
		.filter(|line| !line.is_empty())
		.filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
		.filter(|event| event.get("type").and_then(serde_json::Value::as_str) == Some("error"))
		.find_map(|event| {
			let message = event.get("message")?.as_str()?.to_owned();
			// The message is often a JSON error envelope quoted into a string. One level of
			// unwrapping turns a wall of escapes back into the sentence it contains.
			Some(
				serde_json::from_str::<serde_json::Value>(&message)
					.ok()
					.and_then(|inner| Some(inner.pointer("/error/message")?.as_str()?.to_owned()))
					.unwrap_or(message),
			)
		})
}

/// Split a tier name into the model and the effort the CLI wants separately.
///
/// Every provider here names its tiers with the effort baked in, because for most of them it
/// is genuinely part of the model id. Codex is the exception: it takes a family and a
/// reasoning effort as two settings, and rejects `gpt-5.6-terra-medium` outright. Splitting
/// happens here, at the edge that binds to the CLI, so the vocabulary stays one shape
/// everywhere else -- including in what gets recorded, which should say what was asked for.
fn split_effort(model: &str) -> (&str, Option<&str>) {
	match model.rsplit_once('-') {
		Some((family, effort @ ("low" | "medium" | "high" | "xhigh" | "max" | "ultra"))) => {
			(family, Some(effort))
		}
		_ => (model, None),
	}
}

/// The argument list for one `codex exec` run.
///
/// Separated from the call so the shape can be asserted. The prompt's position here is load
/// bearing and the way it breaks is silent -- see the terminator below.
fn codex_args(prompt: &str, model: &str, image: Option<&Path>) -> Vec<OsString> {
	let mut args: Vec<OsString> = ["exec", "--ephemeral", "--sandbox", "read-only", "--model"]
		.iter()
		.map(OsString::from)
		.collect();
	let (family, effort) = split_effort(model);
	args.push(family.into());
	if let Some(effort) = effort {
		// There is no flag for this, only a config override.
		args.push("-c".into());
		args.push(format!("model_reasoning_effort={effort}").into());
	}
	args.push("--json".into());
	if let Some(image) = image {
		args.push("--image".into());
		args.push(image.into());
	}
	// `--image` takes `<FILE>...`, so without this the prompt is read as a second file. Codex
	// then finds no positional argument, falls back to a stdin that `output()` has already
	// closed, and fails with a message naming stdin and never the flag that caused it.
	args.push("--".into());
	args.push(prompt.into());
	args
}

async fn codex(prompt: &str, model: &str, image: Option<&Path>) -> Result<Answer, Refusal> {
	let output = tokio::process::Command::new("codex")
		.args(codex_args(prompt, model, image))
		.output()
		.await
		.map_err(|error| Refusal::Failed(format!("could not run codex: {error}")))?;
	// The stream is read before the exit code, because it is the only place that says why.
	if let Some(reason) = codex_error(&output.stdout) {
		return Err(classify(&format!("codex: {reason}")));
	}
	if !output.status.success() {
		return Err(failed_command("codex", &output));
	}

	let (text, tokens) = codex_result(&output.stdout)?;
	Ok(Answer {
		text,
		// Codex's event stream does not name the resolved model, so record the requested one.
		model: model::normalise(model),
		tokens,
		// The CLI reports tokens but not their cost.
		usd: 0.0,
	})
}

async fn cursor(prompt: &str, model: &str) -> Result<Answer, Refusal> {
	let output = tokio::process::Command::new("cursor-agent")
		.arg("--print")
		.arg("--output-format")
		.arg("text")
		// Ask mode can inspect the named image but cannot edit the workspace.
		.arg("--mode")
		.arg("ask")
		.arg("--model")
		.arg(model)
		.arg(prompt)
		.output()
		.await
		.map_err(|error| Refusal::Failed(format!("could not run cursor-agent: {error}")))?;
	if !output.status.success() {
		return Err(failed_command("cursor-agent", &output));
	}

	Ok(Answer {
		text: String::from_utf8_lossy(&output.stdout).into_owned(),
		// Text output identifies neither the resolved model nor usage.
		model: model::normalise(model),
		tokens: 0,
		usd: 0.0,
	})
}

/// The argument list for one `grok -p` run.
///
/// Separated from the call so the shape can be asserted. The model id is a flag, the same
/// as Codex and Cursor; baking it into the prompt would silently ask the default instead.
fn grok_args(prompt: &str, model: &str) -> Vec<OsString> {
	let mut args = vec!["-p".into(), prompt.into(), "-m".into(), model.into()];
	restrict_grok_transform(&mut args);
	args
}

/// Keep a text transformation out of the coding-agent environment the Grok CLI discovers.
///
/// Twitter lookup deliberately uses the agent path below. Translation, summaries and vision
/// already carry every byte they need and must not inspect the repository, run hooks or narrate
/// a plan learned from project instructions.
const GROK_TRANSFORM_SYSTEM: &str = "You are a stateless text transformation engine. Follow only +the user's prompt. Do not inspect files, repositories, project instructions, memories, skills, +version control or previous outputs. Do not plan, explain, use tools, run commands or delegate. +Return only the exact output format the prompt requests.";

fn restrict_grok_transform(args: &mut Vec<OsString>) {
	args.extend(
		[
			"--system-prompt-override",
			GROK_TRANSFORM_SYSTEM,
			"--verbatim",
			"--tools",
			"",
			"--no-subagents",
			"--disable-web-search",
			"--max-turns",
			"1",
			"--permission-mode",
			"dontAsk",
		]
		.into_iter()
		.map(OsString::from),
	);
}

fn grok_text_args(prompt: &str, model: &str, extra: &[&str]) -> Vec<OsString> {
	let mut args = vec![
		"-p".into(),
		prompt.into(),
		"-m".into(),
		model.into(),
		"--permission-mode".into(),
		"bypassPermissions".into(),
	];
	for flag in extra {
		args.push((*flag).into());
	}
	args
}

/// Ask Grok with extra CLI flags. Used by jobs that are not a runner choice.
///
/// Translation goes through `ask(Runner::Grok, ...)`. Lookups that exist only on Grok
/// pass the flags that job needs -- see spec/twitter.md.
pub async fn ask_grok(prompt: &str, model: &str, extra: &[&str]) -> Result<Answer, Refusal> {
	grok_spawn(grok_text_args(prompt, model, extra), model).await
}

/// The argument list for one visual `grok` run.
///
/// The text path uses `-p`. This one cannot: the image has to travel as an ACP content
/// block on `--prompt-json`. The help text names that flag but not the block shape. The
/// working shape has `data` and `mimeType` at the top of the block; the Anthropic-style
/// `{"source":{"type":"base64",...}}` nesting is rejected with `missing field 'data'`.
fn grok_vision_args(prompt: &str, model: &str, mime: &str, data: &str) -> Vec<OsString> {
	let blocks = serde_json::json!([
		{
			"type": "image",
			"mimeType": mime,
			"data": data,
		},
		{
			"type": "text",
			"text": prompt,
		},
	]);
	let mut args = vec![
		"--prompt-json".into(),
		blocks.to_string().into(),
		"-m".into(),
		model.into(),
	];
	restrict_grok_transform(&mut args);
	args
}

async fn grok(prompt: &str, model: &str, image: Option<&Path>) -> Result<Answer, Refusal> {
	let args = match image {
		Some(path) => {
			let bytes = std::fs::read(path)
				.map_err(|error| Refusal::Failed(format!("could not read {}: {error}", path.display())))?;
			grok_vision_args(
				prompt,
				model,
				crate::image::mime_of(path),
				&STANDARD.encode(bytes),
			)
		}
		None => grok_args(prompt, model),
	};
	grok_spawn(args, model).await
}

async fn grok_spawn(args: Vec<OsString>, model: &str) -> Result<Answer, Refusal> {
	let output = tokio::process::Command::new("grok")
		.args(args)
		.output()
		.await
		.map_err(|error| Refusal::Failed(format!("could not run grok: {error}")))?;
	if !output.status.success() {
		return Err(failed_command("grok", &output));
	}

	Ok(Answer {
		text: String::from_utf8_lossy(&output.stdout).into_owned(),
		// Plain stdout identifies neither the resolved model nor usage.
		model: model::normalise(model),
		tokens: 0,
		usd: 0.0,
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
	fn an_attached_image_cannot_swallow_the_prompt() {
		// `--image` takes `<FILE>...`. Without the terminator it took the prompt as a second
		// file, and codex went looking for the prompt on a closed stdin -- reporting that
		// nothing arrived there rather than that a flag had eaten it. Every image tagged in one
		// run failed identically before this was found.
		let args = codex_args(
			"describe this",
			"gpt-5.6-terra-medium",
			Some(Path::new("/a.png")),
		);
		let end = &args[args.len() - 2..];
		assert_eq!(end, ["--", "describe this"]);

		// The terminator holds with no image too, where a prompt beginning with a dash would
		// otherwise be read as a flag.
		let bare = codex_args("--not-a-flag", "gpt-5.6-terra-medium", None);
		assert_eq!(&bare[bare.len() - 2..], ["--", "--not-a-flag"]);
		assert!(!bare.contains(&OsString::from("--image")));
	}

	#[test]
	fn codex_is_given_the_effort_apart_from_the_model() {
		// It rejects `gpt-5.6-terra-medium` as a model name and takes the effort as a config
		// override instead. Everywhere else the tier is one string, so the split lives here.
		assert_eq!(
			split_effort("gpt-5.6-terra-medium"),
			("gpt-5.6-terra", Some("medium"))
		);
		assert_eq!(
			split_effort("gpt-5.6-luna-high"),
			("gpt-5.6-luna", Some("high"))
		);
		// A name that merely ends in a word is not an effort.
		assert_eq!(split_effort("gpt-5.6-sol"), ("gpt-5.6-sol", None));
		assert_eq!(
			split_effort("gpt-5.6-sol-xhigh"),
			("gpt-5.6-sol", Some("xhigh"))
		);

		let args = codex_args("hi", "gpt-5.6-terra-medium", None);
		assert!(args.contains(&OsString::from("gpt-5.6-terra")));
		assert!(args.contains(&OsString::from("model_reasoning_effort=medium")));
		assert!(!args.contains(&OsString::from("gpt-5.6-terra-medium")));
	}

	#[test]
	fn an_explicit_codex_model_can_carry_an_explicit_effort() {
		assert_eq!(
			model_override(Runner::Codex, Some("gpt-5.6-sol"), Some("xhigh")).unwrap(),
			Some("gpt-5.6-sol-xhigh".to_owned())
		);
		assert_eq!(model_override(Runner::Codex, None, None).unwrap(), None);
		assert!(model_override(Runner::GptOss, Some("gpt-5.6-sol"), None).is_err());
		assert!(model_override(Runner::Codex, None, Some("xhigh")).is_err());

		let args = codex_args("translate", "gpt-5.6-sol-xhigh", None);
		assert!(args.contains(&OsString::from("gpt-5.6-sol")));
		assert!(args.contains(&OsString::from("model_reasoning_effort=xhigh")));
	}

	#[test]
	fn the_reason_a_turn_failed_is_read_off_the_stream() {
		// stderr says only "Reading additional input from stdin...", which it also says on a
		// run that works. Reporting that instead of this is reporting nothing.
		let stream = concat!(
			r#"{"type":"thread.started","thread_id":"x"}"#,
			"\n",
			r#"{"type":"error","message":"{\"error\":{\"message\":\"model not supported\"}}"}"#,
			"\n"
		);
		assert_eq!(
			codex_error(stream.as_bytes()).as_deref(),
			Some("model not supported")
		);

		// A plain message survives the unwrapping attempt intact.
		let plain = "{\"type\":\"error\",\"message\":\"stream disconnected\"}\n";
		assert_eq!(
			codex_error(plain.as_bytes()).as_deref(),
			Some("stream disconnected")
		);
		assert_eq!(codex_error(b"{\"type\":\"turn.completed\"}\n"), None);
	}

	#[test]
	fn a_runner_is_named_the_way_a_person_would_type_it() {
		assert_eq!(Runner::parse("claude"), Some(Runner::Claude));
		assert_eq!(Runner::parse("Gemini"), Some(Runner::Gemini));
		// The binary is `agy`, and somebody will reach for that name.
		assert_eq!(Runner::parse("agy"), Some(Runner::Gemini));
		assert_eq!(Runner::parse("codex"), Some(Runner::Codex));
		assert_eq!(Runner::parse("cursor-agent"), Some(Runner::Cursor));
		assert_eq!(Runner::parse("grok"), Some(Runner::Grok));
		assert_eq!(Runner::parse("gpt"), None);
	}

	#[test]
	fn tiered_runners_have_the_same_three_roles() {
		for runner in [Runner::Claude, Runner::Gemini, Runner::Codex] {
			let light = runner.model_for(Kind::Heading, 0);
			let standard = runner.model_for(Kind::Prose, 0);
			let strong = runner.model_for(Kind::Prose, 2);
			assert_ne!(light, standard);
			assert_ne!(standard, strong);
		}
	}

	#[test]
	fn codex_uses_the_three_requested_tiers() {
		assert_eq!(
			Runner::Codex.model_for(Kind::Heading, 0),
			"gpt-5.6-luna-medium"
		);
		assert_eq!(
			Runner::Codex.model_for(Kind::Prose, 0),
			"gpt-5.6-terra-medium"
		);
		assert_eq!(
			Runner::Codex.model_for(Kind::Prose, 2),
			"gpt-5.6-terra-high"
		);
	}

	#[test]
	fn cursor_only_offers_composer() {
		for kind in [Kind::Heading, Kind::Prose] {
			assert_eq!(Runner::Cursor.model_for(kind, 0), "composer-2.5");
			assert_eq!(Runner::Cursor.model_for(kind, 2), "composer-2.5");
		}
		assert_eq!(Runner::Cursor.model_for_vision(), Some("composer-2.5"));
	}

	#[test]
	fn grok_uses_the_two_requested_tiers() {
		assert_eq!(Runner::Grok.model_for(Kind::Heading, 0), "grok-4.5");
		assert_eq!(Runner::Grok.model_for(Kind::Prose, 0), "grok-4.6");
		assert_eq!(Runner::Grok.model_for(Kind::Prose, 1), "grok-4.6");
		assert_eq!(Runner::Grok.model_for(Kind::Prose, 2), "grok-4.6");
		assert_eq!(Runner::Grok.model_for_scan(), "grok-4.6");
	}

	#[test]
	fn grok_is_given_the_model_on_the_command_line() {
		// Same shape as Codex and Cursor: the id is a flag. A prompt-only invocation would
		// silently run the CLI default instead of the tier this file chose.
		let args = grok_args("translate this", "grok-4.5");
		assert!(args.contains(&OsString::from("-m")));
		assert!(args.contains(&OsString::from("grok-4.5")));
		assert!(args.contains(&OsString::from("-p")));
		assert!(args.contains(&OsString::from("translate this")));
		assert!(args.contains(&OsString::from(GROK_TRANSFORM_SYSTEM)));
		assert!(args.contains(&OsString::from("--verbatim")));
		assert!(args.contains(&OsString::from("--tools")));
		assert!(args.contains(&OsString::from("dontAsk")));
		assert!(!args.contains(&OsString::from("--prompt-json")));
	}

	#[test]
	fn grok_accepts_extra_flags_after_the_common_ones() {
		// Twitter lookups pass these; translation does not. The common shape stays in one
		// function so a second binding cannot drift. See spec/twitter.md.
		let args = grok_text_args(
			"find this",
			"grok-4.6",
			&["--disable-web-search", "--max-turns", "8"],
		);
		assert!(args.contains(&OsString::from("--disable-web-search")));
		assert!(args.contains(&OsString::from("--max-turns")));
		assert!(args.contains(&OsString::from("8")));
		assert!(args.contains(&OsString::from("bypassPermissions")));
		assert!(!args.contains(&OsString::from(GROK_TRANSFORM_SYSTEM)));
	}

	#[test]
	fn grok_attaches_an_image_as_a_content_block() {
		// `--prompt-json` is how the image lands. `-p` cannot carry it, and the help text
		// does not document the block shape -- `data` and `mimeType` sit at the top; a
		// nested Anthropic `source` is rejected.
		let args = grok_vision_args("what is this", "grok-4.6", "image/png", "QUJD");
		assert!(args.contains(&OsString::from("--prompt-json")));
		assert!(!args.contains(&OsString::from("-p")));
		assert!(args.contains(&OsString::from("-m")));
		assert!(args.contains(&OsString::from("grok-4.6")));
		assert!(args.contains(&OsString::from(GROK_TRANSFORM_SYSTEM)));
		assert!(args.contains(&OsString::from("--tools")));

		let json = args
			.iter()
			.position(|arg| arg == "--prompt-json")
			.and_then(|at| args.get(at + 1))
			.expect("prompt-json value");
		let blocks: serde_json::Value =
			serde_json::from_str(&json.to_string_lossy()).expect("prompt-json");
		assert_eq!(blocks[0]["type"], "image");
		assert_eq!(blocks[0]["mimeType"], "image/png");
		assert_eq!(blocks[0]["data"], "QUJD");
		assert!(blocks[0].get("source").is_none());
		assert_eq!(blocks[1]["type"], "text");
		assert_eq!(blocks[1]["text"], "what is this");
	}

	#[test]
	fn text_and_vision_have_separate_defaults() {
		assert_eq!(DEFAULT_TEXT, Runner::GptOss);
		assert_eq!(DEFAULT_VISION, Runner::Codex);
		assert_eq!(
			DEFAULT_VISION.model_for_vision(),
			Some("gpt-5.6-terra-medium")
		);

		// The open-weight model is text only, and saying so is the whole reason this returns an
		// option. Measured: asked to look at a file it cancels the turn and fills in no error,
		// so a caller handed a model name anyway would see an empty answer and no cause.
		assert_eq!(Runner::GptOss.model_for_vision(), None);
		// Both Grok tiers can see. Vision is quality work, so it takes the prose model.
		assert_eq!(Runner::Grok.model_for_vision(), Some("grok-4.6"));
	}

	#[test]
	fn a_codex_event_stream_yields_the_answer_and_usage() {
		let events = br#"{"type":"thread.started","thread_id":"1"}
{"type":"item.completed","item":{"type":"agent_message","text":"done"}}
{"type":"turn.completed","usage":{"input_tokens":12,"output_tokens":3}}
"#;
		assert_eq!(codex_result(events).unwrap(), ("done".to_owned(), 15));
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
	fn a_grok_model_normalises_to_the_recorded_spelling() {
		assert_eq!(
			model::normalise(Runner::Grok.model_for(Kind::Heading, 0)),
			"grok-4-5"
		);
		assert_eq!(
			model::normalise(Runner::Grok.model_for(Kind::Prose, 0)),
			"grok-4-6"
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
		assert_eq!(Runner::GptOss.provider(), "openai");
		assert_eq!(Runner::Codex.provider(), "openai");
		assert_eq!(Runner::Cursor.provider(), "cursor");
		assert_eq!(Runner::Grok.provider(), "xai");
	}
}
