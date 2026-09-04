//! What `cms` accepts, stated once as types.
//!
//! The whole point of declaring it rather than reading it: an argument this file does not name
//! is rejected. The version this replaced read `--limit` with `.parse().ok()` and treated the
//! failure as absent -- and absent means *no limit* on five commands that spend money per item,
//! so `--limit 2x` and `--lmit 2` both bought the whole library in silence. Neither can now.
//!
//! Domain types stay out of here. `Runner` is parsed through a function rather than by deriving
//! `ValueEnum` on it, because the CLI is one of two adapters over the application layer and a
//! domain type that derives a clap trait has learned about the command line. See
//! spec/architecture/cms.md.

use crate::i18n::runner::Runner;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Cargo's palette rather than clap's default.
///
/// This is a local Rust CLI standing next to `cargo` in the same terminal, and matching what its
/// user's eye already sorts -- green for headings, cyan for the things you type -- costs one
/// constant and saves them learning a second colour language.
const STYLES: clap::builder::Styles = clap::builder::Styles::styled()
	.header(anstyle::AnsiColor::Green.on_default().bold())
	.usage(anstyle::AnsiColor::Green.on_default().bold())
	.literal(anstyle::AnsiColor::Cyan.on_default().bold())
	.placeholder(anstyle::AnsiColor::Cyan.on_default())
	.error(anstyle::AnsiColor::Red.on_default().bold())
	.valid(anstyle::AnsiColor::Cyan.on_default().bold())
	.invalid(anstyle::AnsiColor::Yellow.on_default().bold());

#[derive(Parser, Debug)]
#[command(
	name = "cms",
	about = "Content management for this workspace",
	styles = STYLES,
	arg_required_else_help = true,
	disable_version_flag = true
)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Command,
}

/// The model a paid command asks, and how hard it is asked to think.
///
/// One struct rather than three flags repeated five times: the three are meaningless apart, and
/// `--model-id` with `--effort` is resolved against whichever runner `--model` chose.
#[derive(Args, Debug, Clone)]
pub struct ModelArgs {
	/// Which runner answers: claude, gemini, gpt-oss, codex, cursor or grok.
	#[arg(long, value_name = "RUNNER", value_parser = parse_runner)]
	pub model: Option<Runner>,
	/// A specific model id, overriding what the runner would pick.
	#[arg(long = "model-id", value_name = "ID")]
	pub model_id: Option<String>,
	/// Reasoning effort: low, medium, high, xhigh, max or ultra.
	#[arg(long, value_name = "EFFORT")]
	pub effort: Option<String>,
}

impl ModelArgs {
	/// The runner to ask, or the caller's default when `--model` was not given.
	pub fn runner(&self, default: Runner) -> Runner {
		self.model.unwrap_or(default)
	}

	/// The specific model id `--model-id` and `--effort` resolve to for that runner.
	pub fn overrides(&self, runner: Runner) -> Result<Option<String>, String> {
		crate::i18n::runner::model_override(runner, self.model_id.as_deref(), self.effort.as_deref())
	}
}

fn parse_runner(name: &str) -> Result<Runner, String> {
	Runner::parse(name).ok_or_else(|| format!("expected {}", crate::i18n::runner::CHOICES))
}

/// A positive count. Zero is rejected rather than silently meaning "all".
fn positive(value: &str) -> Result<usize, String> {
	match value.parse::<usize>() {
		Ok(value) if value > 0 => Ok(value),
		Ok(_) => Err("expected a positive integer, not zero".to_owned()),
		Err(_) => Err("expected a positive integer".to_owned()),
	}
}

fn positive_u32(value: &str) -> Result<u32, String> {
	match value.parse::<u32>() {
		Ok(value) if value > 0 => Ok(value),
		Ok(_) => Err("expected a positive integer, not zero".to_owned()),
		Err(_) => Err("expected a positive integer".to_owned()),
	}
}

#[derive(Subcommand, Debug)]
pub enum Command {
	/// Print the workspace overview as JSON
	Overview,
	/// Print the article listing and translation coverage
	Articles,
	/// Print what each derived record class still owes
	Derived,
	/// Print the catalogue of long-running operations
	Tasks,
	/// Print what is running right now, machine-wide
	Runs,
	/// Print the port the web UI will bind
	Port,
	/// Write article segment ids and source ranges
	Segments,
	/// List referenced assets that are not present
	Check,
	/// Record the licence of every dependency the apps ship
	Licenses,

	/// Collect the icons the linkcards need
	Favicon {
		/// Fetch again even where an icon is already stored
		#[arg(long)]
		force: bool,
		/// Domains to collect; defaults to whatever the articles link to
		#[arg(value_name = "DOMAIN")]
		domains: Vec<String>,
	},

	/// Derive what the articles reference, then rewrite them
	Image {
		/// Derive again even where the variants are already published
		#[arg(long)]
		force: bool,
		/// Keep a full-resolution rung for images where the detail is the point
		#[arg(long)]
		original: bool,
		/// Files to import ahead of the article that will use them
		#[arg(value_name = "FILE")]
		files: Vec<PathBuf>,
	},

	/// Render an OpenGraph card per page per language
	Og {
		/// Render again rather than skipping what is current
		#[arg(long)]
		force: bool,
	},

	/// Describe assets that have no description yet
	Alt {
		#[command(flatten)]
		model: ModelArgs,
		/// Redo what is already recorded rather than skipping it
		#[arg(long)]
		force: bool,
		/// Stop after this many, so a prompt can be tried cheaply
		#[arg(long, value_name = "N", value_parser = positive)]
		limit: Option<usize>,
	},

	/// Give each asset a category and tags
	Tag {
		#[command(flatten)]
		model: ModelArgs,
		/// Redo what is already recorded rather than skipping it
		#[arg(long)]
		force: bool,
		/// Stop after this many, so a prompt can be tried cheaply
		#[arg(long, value_name = "N", value_parser = positive)]
		limit: Option<usize>,
	},

	/// Write a reader-facing summary for each article
	Summary {
		#[command(flatten)]
		model: ModelArgs,
		/// Redo what is already recorded rather than skipping it
		#[arg(long)]
		force: bool,
		/// Stop after this many, so a prompt can be tried cheaply
		#[arg(long, value_name = "N", value_parser = positive)]
		limit: Option<usize>,
	},

	/// Translate article segments into every locale
	I18n {
		#[command(flatten)]
		model: ModelArgs,
		/// Redo what is already recorded rather than skipping it
		#[arg(long)]
		force: bool,
		/// Report what is missing without asking for any of it
		#[arg(long)]
		check: bool,
		/// Translate frontmatter only, leaving the body alone
		#[arg(long)]
		frontmatter: bool,
		#[arg(long, value_name = "N", value_parser = positive)]
		limit: Option<usize>,
		/// How many requests are in flight at once
		#[arg(long, value_name = "N", value_parser = positive)]
		parallel: Option<usize>,
		/// Restrict to these locales; repeatable
		#[arg(long, value_name = "LOCALE")]
		locale: Vec<String>,
		/// Articles to translate; defaults to all of them
		#[arg(value_name = "ARTICLE")]
		articles: Vec<PathBuf>,
	},

	/// Suggest passages a translation would have to gloss
	Tn {
		#[command(flatten)]
		model: ModelArgs,
		/// Scan again rather than reusing what is recorded
		#[arg(long)]
		force: bool,
		#[arg(value_name = "ARTICLE")]
		articles: Vec<PathBuf>,
	},

	/// Fetch the crate and repository data the articles embed
	Embed {
		/// Fetch again rather than reusing what is recorded
		#[arg(long)]
		force: bool,
	},

	/// Translate tag labels, descriptions and summaries
	Locale {
		#[command(flatten)]
		model: ModelArgs,
		/// Redo what is already recorded rather than skipping it
		#[arg(long)]
		force: bool,
		/// Stop after this many, so a prompt can be tried cheaply
		#[arg(long, value_name = "N", value_parser = positive)]
		limit: Option<usize>,
	},

	/// Drop recorded translations precisely, so the next i18n run redoes them
	Invalidate {
		/// Actually delete. Without it the selection is printed and nothing is touched.
		#[arg(long)]
		live: bool,
		/// Drop this segment id; repeatable
		#[arg(long, value_name = "ID")]
		segment: Vec<String>,
		/// Drop segments whose source contains this text; repeatable
		#[arg(long, value_name = "TEXT")]
		containing: Vec<String>,
		/// Drop entries whose stored translation contains this text; repeatable
		#[arg(long, value_name = "TEXT")]
		translation_containing: Vec<String>,
		/// Restrict to these locales; repeatable
		#[arg(long, value_name = "LOCALE")]
		locale: Vec<String>,
		/// Articles to touch; defaults to all of them
		#[arg(value_name = "ARTICLE")]
		articles: Vec<PathBuf>,
	},

	/// Drop published assets no article asks for
	Gc {
		/// Actually delete. Without it the sweep is printed and nothing is touched.
		#[arg(long)]
		live: bool,
		/// Sweep translations for paragraphs an article no longer has, instead of published bytes
		#[arg(long)]
		segments: bool,
		/// Limit to these articles, relative to `contents`; repeatable. Implies `--segments`
		#[arg(long, value_name = "PATH")]
		article: Vec<String>,
	},

	/// Search Twitter
	Twitter {
		#[command(subcommand)]
		command: TwitterCommand,
	},
}

#[derive(Subcommand, Debug)]
pub enum TwitterCommand {
	/// Search Twitter users
	User {
		/// Words to search for; joined with spaces
		#[arg(value_name = "QUERY", required = true)]
		query: Vec<String>,
		#[arg(long, value_name = "N", value_parser = positive_u32)]
		count: Option<u32>,
	},
	/// Search Twitter posts by keyword
	Keyword {
		#[arg(value_name = "QUERY", required = true)]
		query: Vec<String>,
		#[arg(long, value_name = "N", value_parser = positive_u32)]
		limit: Option<u32>,
		/// Top or Latest
		#[arg(long, value_name = "MODE")]
		mode: Option<String>,
	},
	/// Fetch a Twitter post and its replies
	Thread {
		#[arg(value_name = "POST_ID")]
		id: String,
	},
	/// Search Twitter posts by meaning
	Semantic {
		#[arg(value_name = "QUERY", required = true)]
		query: Vec<String>,
		#[arg(long, value_name = "N", value_parser = positive_u32)]
		limit: Option<u32>,
		/// Only posts from this date onwards
		#[arg(long, value_name = "DATE")]
		from: Option<String>,
		/// Only posts up to this date
		#[arg(long, value_name = "DATE")]
		to: Option<String>,
		/// Restrict to these authors; repeatable
		#[arg(long, value_name = "NAME")]
		user: Vec<String>,
		/// Drop these authors; repeatable
		#[arg(long = "exclude-user", value_name = "NAME")]
		exclude_user: Vec<String>,
		/// Discard matches scoring below this
		#[arg(long = "min-score", value_name = "SCORE")]
		min_score: Option<f64>,
	},
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::CommandFactory;

	#[test]
	fn the_definition_is_internally_consistent() {
		// clap's own audit of the shape: duplicate flags, unreachable positionals, a long name
		// declared twice. It is a debug assertion, so it has to be asked for by a test.
		Cli::command().debug_assert();
	}

	/// The failure this whole change exists to remove.
	///
	/// `--limit` absent means no limit, so a value that does not parse must not read as absent.
	/// It used to: `.parse().ok()` turned `2x` into `None` and the run bought everything.
	#[test]
	fn a_limit_that_is_not_a_number_is_refused_rather_than_ignored() {
		let parsed = Cli::try_parse_from(["cms", "alt", "--limit", "2x"]);
		assert!(parsed.is_err(), "a bad --limit must not read as unlimited");
	}

	#[test]
	fn a_limit_of_zero_is_refused_rather_than_meaning_everything() {
		assert!(Cli::try_parse_from(["cms", "alt", "--limit", "0"]).is_err());
	}

	/// The other half, and the likelier typo: the flag name itself.
	#[test]
	fn a_misspelt_flag_is_refused_rather_than_dropped() {
		let parsed = Cli::try_parse_from(["cms", "alt", "--lmit", "2"]);
		assert!(parsed.is_err(), "an unknown flag must not be ignored");
	}

	#[test]
	fn a_good_limit_still_parses() {
		let cli = Cli::try_parse_from(["cms", "alt", "--limit", "2"]).expect("parse");
		match cli.command {
			Command::Alt { limit, .. } => assert_eq!(limit, Some(2)),
			other => panic!("expected alt, got {other:?}"),
		}
	}

	#[test]
	fn an_unknown_runner_names_the_choices() {
		let error = Cli::try_parse_from(["cms", "alt", "--model", "gpt-5"])
			.expect_err("an unknown runner is refused")
			.to_string();
		assert!(error.contains("gpt-oss"), "the error lists what is accepted");
	}

	#[test]
	fn free_arguments_still_reach_the_commands_that_take_them() {
		let cli = Cli::try_parse_from(["cms", "image", "--original", "a.png", "b.png"]).expect("parse");
		match cli.command {
			Command::Image { original, files, .. } => {
				assert!(original);
				assert_eq!(files.len(), 2);
			}
			other => panic!("expected image, got {other:?}"),
		}
	}

	#[test]
	fn a_repeated_option_collects_rather_than_replacing() {
		let cli = Cli::try_parse_from(["cms", "i18n", "--locale", "zh-CN", "--locale", "ja-JP"])
			.expect("parse");
		match cli.command {
			Command::I18n { locale, .. } => assert_eq!(locale, ["zh-CN", "ja-JP"]),
			other => panic!("expected i18n, got {other:?}"),
		}
	}

	#[test]
	fn twitter_keeps_its_nested_commands() {
		let cli =
			Cli::try_parse_from(["cms", "twitter", "semantic", "a", "b", "--limit", "5"]).expect("parse");
		match cli.command {
			Command::Twitter { command: TwitterCommand::Semantic { query, limit, .. } } => {
				assert_eq!(query, ["a", "b"]);
				assert_eq!(limit, Some(5));
			}
			other => panic!("expected twitter semantic, got {other:?}"),
		}
	}
}
