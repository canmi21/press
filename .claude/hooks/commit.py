#!/usr/bin/env python3
"""PreToolUse hook for `jj commit` and `jj describe`.

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

import json
import os
import re
import shlex
import subprocess
import sys

TYPES = "feat|fix|docs|refactor|perf|test|build|ci|chore|revert"
HEADER = re.compile(rf"^({TYPES})(\([a-z0-9.-]+\))?!?: (.+)$")
LIMIT = 96
SUBCOMMANDS = {"commit", "describe", "desc", "ci"}
SEPARATORS = {"&&", ";", "||", "|"}


def messages_in(command: str) -> list[str]:
	"""Every -m value attached to a commit/describe subcommand in a shell command line."""
	try:
		tokens = shlex.split(command)
	except ValueError:
		return []  # unbalanced quoting; let the normal permission flow deal with it

	found, armed = [], False
	for i, token in enumerate(tokens):
		if token in SUBCOMMANDS:
			armed = True
		elif token in SEPARATORS:
			armed = False
		elif armed and token in ("-m", "--message") and i + 1 < len(tokens):
			found.append(tokens[i + 1])
		elif armed and token.startswith("--message="):
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
	return found


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0

	command = payload.get("tool_input", {}).get("command", "")
	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	problems = [p for message in messages_in(command) for p in problems_with(message)]
	if problems:
		print("Commit message rejected by .claude/hooks/commit.py:\n", file=sys.stderr)
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


if __name__ == "__main__":
	sys.exit(main())
