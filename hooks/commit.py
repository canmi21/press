#!/usr/bin/env python3
"""PreToolUse hook for every jj subcommand that writes a commit description.

Two jobs, in order:
  1. Run `jj fix`, so what gets committed is already formatted.
  2. Validate the commit message, refusing the command with a specific reason when it is
     malformed. Exit 2 sends stderr back to the agent as the rejection reason, which is
     what lets it correct the message itself.

jj offers no hook point of its own: it has no commit hook, and it refuses aliases that
shadow built-in commands. The moment a message is written is inside the agent's tool call,
so the check sits there.

Dependencies are deliberately limited to the Python standard library -- no jq, no bash, no
mise lookup. A hook that fails to start fails silently, and a silent hook is worse than no
hook, so it must not depend on the environment being set up correctly.

See spec/commits.md and spec/lint-format.md.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True

from jj_command import invocations

TYPES = "feat|fix|docs|refactor|perf|test|build|ci|chore|revert"
HEADER = re.compile(rf"^({TYPES})(\([a-z0-9.-]+\))?!?: (.+)$")
LIMIT = 96
# Every jj subcommand that can write a description, not just the two that usually do. The
# list was `commit` and `describe` alone, and `jj split -m` walked a malformed trailer
# straight past it -- caught only because a later `describe` in the same session was checked
# and rejected the identical message. A gap here is silent by construction, so this is
# derived from which subcommands accept -m rather than from which ones come to mind.
SUBCOMMANDS = {
	"commit",
	"ci",
	"describe",
	"desc",
	"split",
	"new",
	"squash",
	"metaedit",
}
# Whether an assistant co-authored a change is a judgement no script can make, so only the
# shape is checked here: if a trailer is present at all, it has to be one of the two agreed
# forms. See spec/commits.md for when to add one.
COAUTHOR = re.compile(r"^Co-Authored-By:", re.IGNORECASE)
COAUTHOR_OK = re.compile(
	r"^Co-Authored-By: (?:Claude [A-Za-z0-9.-]+ <noreply@anthropic\.com>"
	r"|Codex [A-Za-z0-9.-]+ <codex@openai\.com>)$"
)


def messages_in(command: str) -> list[str]:
	"""Every -m value attached to a commit/describe subcommand in a shell command line."""
	found = []
	for subcommand, arguments in invocations(command):
		if subcommand not in SUBCOMMANDS:
			continue
		for i, token in enumerate(arguments):
			if token in ("-m", "--message") and i + 1 < len(arguments):
				found.append(arguments[i + 1])
			elif token.startswith("--message="):
				found.append(token.split("=", 1)[1])
	return found


def problems_with(message: str) -> list[str]:
	subject = message.splitlines()[0] if message else ""
	match = HEADER.match(subject)
	if not match:
		types = TYPES.replace("|", ", ")
		return [
			f"{subject!r}\n    is not a Conventional Commit. Expected `type: subject`,\n"
			f"    where type is one of: {types}"
		]

	found, text = [], match.group(3)
	if text[:1].isupper():
		found.append(f"{subject!r}\n    subject must start lowercase")
	if text.endswith("."):
		found.append(f"{subject!r}\n    subject must not end with a period")
	if len(subject) > LIMIT:
		found.append(f"{subject!r}\n    subject is {len(subject)} chars, limit is {LIMIT}")

	for line in message.splitlines()[1:]:
		trailer = line.strip()
		if COAUTHOR.match(trailer) and not COAUTHOR_OK.match(trailer):
			found.append(
				f"{trailer!r}\n"
				f"    malformed co-author trailer. Expected exactly one of:\n"
				f"      Co-Authored-By: Claude <Model>-<Version> <noreply@anthropic.com>\n"
				f"      Co-Authored-By: Codex <Model>-<Version> <codex@openai.com>"
			)
	return found


def handle(payload: dict) -> int:
	"""Apply the commit policy to one lifecycle payload."""
	command = payload.get("tool_input", {}).get("command", "")
	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	messages = messages_in(command)
	# Both harnesses can match Bash but only Claude Code has a handler-level `if` filter.
	# Keep the selection here so the shared hook behaves identically behind either adapter.
	if not messages:
		return 0

	problems = [p for message in messages for p in problems_with(message)]
	if problems:
		print("Commit message rejected by hooks/commit.py:\n", file=sys.stderr)
		for problem in problems:
			print(f"  - {problem}\n", file=sys.stderr)
		print(
			"Fix the message and run the command again. Scope is omitted in most cases.\n"
			"Full rules: spec/commits.md",
			file=sys.stderr,
		)
		return 2

	# The message is acceptable, so format before the commit is written. jj's repo config is
	# not version controlled, so JJ_CONFIG has to be pointed at the tracked jj.toml here
	# rather than relying on mise having been activated in whatever shell invoked us.
	root = subprocess.run(
		["jj", "workspace", "root"], capture_output=True, text=True
	).stdout.strip()
	if root:
		env = dict(os.environ)
		home = os.path.expanduser("~")
		env["JJ_CONFIG"] = f"{home}/.config/jj/config.toml:{root}/jj.toml"
		# A broken formatter is not a reason to refuse work, so failures here are ignored.
		subprocess.run(["jj", "fix"], capture_output=True, env=env, timeout=120)
	return 0


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0
	return handle(payload)


if __name__ == "__main__":
	sys.exit(main())
