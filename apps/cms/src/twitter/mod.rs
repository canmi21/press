//! Lookups that exist only on Grok.
//!
//! `Runner` answers which assistant does a job every assistant can do. These tools exist
//! only for Grok, so the provider is a fact of the operation rather than a parameter of
//! the run. See spec/twitter.md.

mod prompt;

use crate::i18n::runner::{self, Answer, Refusal};
use prompt::ParseError;
use serde::Serialize;
use std::future::Future;

/// Structural work with messy post text to transcribe. Vision used 4.6 for the same
/// reason: one-shot quality, no heading-vs-prose signal to route on.
const MODEL: &str = "grok-4.6";

/// So the model cannot substitute web search, spawn helpers, or wander past a lookup.
/// Translation does not pass these; they belong to this job. See spec/twitter.md.
const GROK_FLAGS: &[&str] = &["--disable-web-search", "--no-subagents", "--max-turns", "8"];

/// Cheap default. The tools cap at 10; asking for more would be rejected after a paid call.
pub const DEFAULT_COUNT: u32 = 3;
pub const DEFAULT_LIMIT: u32 = 3;
pub const MAX_RESULTS: u32 = 10;

/// Measured: the tool's own default returned nothing on a reasonable query; 0.1 produced
/// results. Silence from this tool more often means the threshold was too high than that
/// nothing exists. See spec/twitter.md.
pub const DEFAULT_MIN_SCORE: f64 = 0.1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
	Top,
	/// `from:username` is the usual query, and that wants recent posts, not an
	/// engagement ranking that would hide a quiet one.
	#[default]
	Latest,
}

impl Mode {
	pub fn parse(name: &str) -> Option<Self> {
		match name.trim().to_ascii_lowercase().as_str() {
			"top" => Some(Self::Top),
			"latest" => Some(Self::Latest),
			_ => None,
		}
	}

	pub fn as_str(self) -> &'static str {
		match self {
			Self::Top => "Top",
			Self::Latest => "Latest",
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Semantic {
	pub query: String,
	pub limit: u32,
	pub from_date: Option<String>,
	pub to_date: Option<String>,
	pub usernames: Vec<String>,
	pub exclude_usernames: Vec<String>,
	pub min_score: f64,
}

impl Semantic {
	pub fn new(query: impl Into<String>) -> Self {
		Self {
			query: query.into(),
			limit: DEFAULT_LIMIT,
			from_date: None,
			to_date: None,
			usernames: Vec::new(),
			exclude_usernames: Vec::new(),
			min_score: DEFAULT_MIN_SCORE,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
	pub id: String,
	pub username: String,
	pub name: String,
	pub bio: String,
	pub followers: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
	pub id: String,
	pub author: String,
	pub text: String,
	pub created: String,
	pub likes: u64,
	pub reposts: u64,
	pub replies: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub score: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Users {
	pub query: String,
	pub users: Vec<User>,
	pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Posts {
	pub query: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mode: Option<Mode>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub from_date: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub to_date: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub usernames: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub exclude_usernames: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_score: Option<f64>,
	pub posts: Vec<Post>,
	pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
	pub post_id: String,
	pub posts: Vec<Post>,
	pub rejected: usize,
}

#[derive(Debug)]
pub enum Error {
	Invalid(&'static str),
	Refused(Refusal),
	Parse(ParseError),
}

impl std::fmt::Display for Error {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Invalid(reason) => write!(formatter, "{reason}"),
			Self::Refused(reason) => write!(formatter, "{reason}"),
			Self::Parse(reason) => write!(formatter, "{reason}"),
		}
	}
}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
	fn from(error: ParseError) -> Self {
		Self::Parse(error)
	}
}

pub async fn users(query: &str, count: u32) -> Result<Users, Error> {
	users_with(query, count, ask).await
}

pub async fn keyword(query: &str, limit: u32, mode: Mode) -> Result<Posts, Error> {
	keyword_with(query, limit, mode, ask).await
}

pub async fn thread(post_id: &str) -> Result<Thread, Error> {
	thread_with(post_id, ask).await
}

pub async fn semantic(options: Semantic) -> Result<Posts, Error> {
	semantic_with(options, ask).await
}

async fn ask(prompt: String, model: String) -> Result<Answer, Refusal> {
	runner::ask_grok(&prompt, &model, GROK_FLAGS).await
}

async fn users_with<F, Fut>(query: &str, count: u32, ask: F) -> Result<Users, Error>
where
	F: Fn(String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	if query.trim().is_empty() {
		return Err(Error::Invalid("x user takes a query"));
	}
	check_limit(count, "count")?;
	let request = prompt::users_request(query, count);
	let answer = ask(request.text, MODEL.to_owned()).await.map_err(Error::Refused)?;
	let (users, rejected) = prompt::parse_users(&answer.text, Some(&request.boundary))?;
	Ok(Users { query: query.to_owned(), users, rejected })
}

async fn keyword_with<F, Fut>(query: &str, limit: u32, mode: Mode, ask: F) -> Result<Posts, Error>
where
	F: Fn(String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	if query.trim().is_empty() {
		return Err(Error::Invalid("x keyword takes a query"));
	}
	check_limit(limit, "limit")?;
	let request = prompt::keyword_request(query, limit, mode);
	let answer = ask(request.text, MODEL.to_owned()).await.map_err(Error::Refused)?;
	let (posts, rejected) = prompt::parse_posts(&answer.text, false, false, Some(&request.boundary))?;
	Ok(Posts {
		query: query.to_owned(),
		mode: Some(mode),
		from_date: None,
		to_date: None,
		usernames: Vec::new(),
		exclude_usernames: Vec::new(),
		min_score: None,
		posts,
		rejected,
	})
}

async fn thread_with<F, Fut>(post_id: &str, ask: F) -> Result<Thread, Error>
where
	F: Fn(String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	if !prompt::snowflake(post_id) {
		return Err(Error::Invalid("post id is a 19-digit snowflake"));
	}
	let request = prompt::thread_request(post_id);
	let answer = ask(request.text, MODEL.to_owned()).await.map_err(Error::Refused)?;
	let (posts, rejected) = prompt::parse_posts(&answer.text, false, true, Some(&request.boundary))?;
	Ok(Thread { post_id: post_id.to_owned(), posts, rejected })
}

async fn semantic_with<F, Fut>(options: Semantic, ask: F) -> Result<Posts, Error>
where
	F: Fn(String, String) -> Fut,
	Fut: Future<Output = Result<Answer, Refusal>>,
{
	if options.query.trim().is_empty() {
		return Err(Error::Invalid("x semantic takes a query"));
	}
	check_limit(options.limit, "limit")?;
	if let Some(date) = options.from_date.as_deref() {
		check_date(date)?;
	}
	if let Some(date) = options.to_date.as_deref() {
		check_date(date)?;
	}
	if !options.min_score.is_finite() || options.min_score < 0.0 {
		return Err(Error::Invalid("min-score must be a non-negative number"));
	}
	let request = prompt::semantic_request(&options);
	let answer = ask(request.text, MODEL.to_owned()).await.map_err(Error::Refused)?;
	let (posts, rejected) = prompt::parse_posts(&answer.text, true, false, Some(&request.boundary))?;
	Ok(Posts {
		query: options.query,
		mode: None,
		from_date: options.from_date,
		to_date: options.to_date,
		usernames: options.usernames,
		exclude_usernames: options.exclude_usernames,
		min_score: Some(options.min_score),
		posts,
		rejected,
	})
}

fn check_limit(value: u32, name: &'static str) -> Result<(), Error> {
	if (1..=MAX_RESULTS).contains(&value) {
		Ok(())
	} else {
		Err(Error::Invalid(match name {
			"count" => "count must be between 1 and 10",
			_ => "limit must be between 1 and 10",
		}))
	}
}

fn check_date(value: &str) -> Result<(), Error> {
	if value.parse::<jiff::civil::Date>().is_ok() {
		Ok(())
	} else {
		Err(Error::Invalid("date must be YYYY-MM-DD"))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::i18n::segment::{CLOSE, OPEN};

	fn mark(name: &str) -> String {
		format!("{OPEN}{name}{CLOSE}")
	}

	fn user_reply() -> String {
		format!(
			"{}\n1\n{}\n{}\n3091974750\n{}\ncanmi\n{}\nCanmi\n{}\nhello\n{}\n4\n",
			mark("count"),
			mark("user"),
			mark("id"),
			mark("username"),
			mark("name"),
			mark("bio"),
			mark("followers"),
		)
	}

	fn answer(text: &str) -> Answer {
		Answer { text: text.to_owned(), model: "grok-4-6".into(), tokens: 0, usd: 0.0 }
	}

	#[tokio::test]
	async fn a_bad_post_id_is_rejected_before_any_call() {
		let result = thread_with("abc", |_, _| async { panic!("must not call") }).await;
		assert!(matches!(result, Err(Error::Invalid(_))));
	}

	#[tokio::test]
	async fn a_zero_count_is_rejected_before_any_call() {
		let result = users_with("canmi", 0, |_, _| async { panic!("must not call") }).await;
		assert!(matches!(result, Err(Error::Invalid(_))));
	}

	#[tokio::test]
	async fn a_user_search_returns_parsed_accounts() {
		let result = users_with("canmi", 3, |prompt, model| {
			assert!(prompt.contains("x_user_search"));
			assert!(prompt.contains("canmi"));
			assert_eq!(model, MODEL);
			std::future::ready(Ok(answer(&user_reply())))
		})
		.await
		.expect("users");
		assert_eq!(result.query, "canmi");
		assert_eq!(result.users.len(), 1);
		assert_eq!(result.users[0].username, "canmi");
		assert_eq!(result.rejected, 0);
	}

	#[tokio::test]
	async fn a_bad_date_is_rejected_before_any_call() {
		let mut options = Semantic::new("rust");
		options.from_date = Some("13-08-2026".into());
		let result = semantic_with(options, |_, _| async { panic!("must not call") }).await;
		assert!(matches!(result, Err(Error::Invalid(_))));
	}

	#[test]
	fn latest_is_the_keyword_default() {
		assert_eq!(Mode::default(), Mode::Latest);
		assert_eq!(Mode::parse("TOP"), Some(Mode::Top));
		assert_eq!(Mode::parse("latest"), Some(Mode::Latest));
		assert_eq!(Mode::parse("hot"), None);
	}

	#[test]
	fn the_semantic_default_is_the_measured_floor() {
		assert_eq!(Semantic::new("q").min_score, 0.1);
	}
}
