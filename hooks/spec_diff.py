#!/usr/bin/env python3
"""Shared PostToolUse hook for `jj rebase`: show the rules that changed under the agent.

In parallel work, one workspace lands a decision in spec/ while another is mid-task with the
old text in its context. There is no channel to push that change across -- deliberately, see
spec/agent-protocol.md -- so it arrives the way code does: at the next rebase onto main. This
hook makes that arrival visible. After a rebase it diffs spec/ and CLAUDE.md between what the
working copy was before the rebase and what it is now, and hands the delta back. A few dozen
lines, exactly the rules that moved, at the one moment they matter: the agent's next commit is
about to land beside them.

Never blocks, and reads nothing when nothing changed. Whether a changed rule affects the task
at hand is the agent's judgement; this only guarantees the change is seen.

The "before" state comes from jj's operation log, which is why no hook has to remember it: the
newest run of rebase operations is found, and the working copy as of the operation just before
that run is the previous state. jj snapshots the working copy before it rebases, so both sides
carry the agent's own edits and the diff is purely what main brought in.

Dependencies are limited to the standard library, for the reason given in spec/toolchain.md.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

sys.dont_write_bytecode = True

from jj_command import invocations

RULE_PATHS = ["spec", "CLAUDE.md"]
# A rebase op is described `rebase ...`; the snapshot jj takes first is `snapshot working copy`.
REBASE_PREFIX = "rebase "
SNAPSHOT = "snapshot working copy"
MAX_LINES = 200


def run(args: list[str]) -> str:
	try:
		result = subprocess.run(args, capture_output=True, text=True, timeout=30)
	except (OSError, subprocess.SubprocessError):
		return ""
	return result.stdout if result.returncode == 0 else ""


def rebased(command: str) -> bool:
	return any(subcommand == "rebase" for subcommand, _ in invocations(command))


def operation_before_rebase() -> str | None:
	"""The id of the operation just older than the newest run of rebase operations."""
	log = run(
		[
			"jj", "op", "log", "--no-graph", "--ignore-working-copy", "--limit", "40",
			"-T", 'id.short() ++ "\\t" ++ description ++ "\\n"',
		]
	)
	ops = [line.split("\t", 1) for line in log.splitlines() if "\t" in line]
	# Skip forward to the newest rebase, then through everything the same command produced.
	i = 0
	while i < len(ops) and not ops[i][1].startswith(REBASE_PREFIX):
		i += 1
	if i == len(ops):
		return None
	while i < len(ops) and (ops[i][1].startswith(REBASE_PREFIX) or ops[i][1] == SNAPSHOT):
		i += 1
	return ops[i][0] if i < len(ops) else None


def context(payload: dict) -> str:
	"""Return the model-visible rule delta for one lifecycle payload, if any."""
	command = payload.get("tool_input", {}).get("command", "")
	if not rebased(command):
		return ""

	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	before = operation_before_rebase()
	if not before:
		return ""
	previous = run(
		["jj", "log", "--at-op", before, "--ignore-working-copy", "--no-graph", "-r", "@", "-T", "commit_id"]
	).strip()
	if not previous:
		return ""

	diff = run(
		["jj", "diff", "--ignore-working-copy", "--git", "--from", previous, "--to", "@", "--", *RULE_PATHS]
	)
	if not diff.strip():
		return ""

	lines = diff.splitlines()
	shown = "\n".join(lines[:MAX_LINES])
	if len(lines) > MAX_LINES:
		shown += (
			f"\n... {len(lines) - MAX_LINES} more lines; run "
			f"`jj diff --git --from {previous[:12]} --to @ -- spec CLAUDE.md` for the rest."
		)

	message = (
		"The rebase brought rule changes from main into this working copy. Read them before the "
		"next commit -- they were written by another workspace while this task was underway, and "
		"spec/agent-protocol.md says a rebase is where they are taken in. If one changes what "
		"this task should do, adjust; if none does, carry on.\n\n" + shown
	)
	return message


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0
	message = context(payload)
	if message:
		print(
			json.dumps(
				{
					"hookSpecificOutput": {
						"hookEventName": "PostToolUse",
						"additionalContext": message,
					}
				}
			)
		)
	return 0


if __name__ == "__main__":
	sys.exit(main())
