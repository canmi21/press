#!/bin/bash
# PreToolUse hook for `jj commit` and `jj describe`.
#
# Two jobs, in order:
#   1. Format the working copy with `jj fix`, so what gets committed is already correct.
#   2. Validate the commit message, and refuse the command with an explanation if it is
#      malformed. Exit 2 sends stderr back to the agent as the reason, which is what lets
#      it fix the message itself instead of waiting for a human or for CI.
#
# jj has no commit hook of its own and refuses aliases that shadow built-in commands, so
# this is the only point where a message can be caught before it is written.
# See spec/lint-format.md and spec/commits.md.

set -uo pipefail

input=$(cat)
cwd=$(jq -r '.cwd // empty' <<<"$input")
command=$(jq -r '.tool_input.command // empty' <<<"$input")

[ -n "$cwd" ] && cd "$cwd" 2>/dev/null

root=$(jj workspace root 2>/dev/null) || exit 0
export JJ_CONFIG="${HOME}/.config/jj/config.toml:${root}/jj.toml"

python3 - "$command" <<'PY'
import re, shlex, subprocess, sys

TYPES = "feat|fix|docs|refactor|perf|test|build|ci|chore|revert"
HEADER = re.compile(rf"^({TYPES})(\([a-z0-9.-]+\))?!?: (.+)$")
LIMIT = 96

command = sys.argv[1]
try:
	tokens = shlex.split(command)
except ValueError:
	sys.exit(0)  # unparseable quoting; let the normal permission flow handle it

# Collect every -m/--message that follows a `commit` or `describe` subcommand, so command
# chains like `jj fix && jj commit -m "..."` are still caught.
messages, armed = [], False
for i, tok in enumerate(tokens):
	if tok in ("commit", "describe", "desc", "ci"):
		armed = True
	elif tok in ("&&", ";", "||"):
		armed = False
	elif armed and tok in ("-m", "--message") and i + 1 < len(tokens):
		messages.append(tokens[i + 1])
	elif armed and tok.startswith("--message="):
		messages.append(tok.split("=", 1)[1])

problems = []
for msg in messages:
	subject = msg.splitlines()[0] if msg else ""
	m = HEADER.match(subject)
	if not m:
		problems.append(
			f"{subject!r}\n"
			f"    does not match Conventional Commits. Expected `type: subject`,\n"
			f"    where type is one of: {TYPES.replace('|', ', ')}"
		)
		continue
	text = m.group(3)
	if text[:1].isupper():
		problems.append(f"{subject!r}\n    subject must start lowercase")
	if text.endswith("."):
		problems.append(f"{subject!r}\n    subject must not end with a period")
	if len(subject) > LIMIT:
		problems.append(f"{subject!r}\n    subject is {len(subject)} chars, limit is {LIMIT}")

if problems:
	print("Commit message rejected by .claude/hooks/jj-commit.sh:\n", file=sys.stderr)
	for p in problems:
		print(f"  - {p}\n", file=sys.stderr)
	print(
		"Fix the message and run the command again. Scope is omitted in most cases.\n"
		"Full rules: spec/commits.md",
		file=sys.stderr,
	)
	sys.exit(2)

# Message is acceptable, so format before the commit is written. A failure here must not
# block the commit -- a broken formatter is not a reason to refuse work.
subprocess.run(["jj", "fix"], capture_output=True, timeout=120)
sys.exit(0)
PY
