#!/usr/bin/env python3
"""Shared PostToolUse hook for `jj commit`: notice decisions that were not written down.

A commit that adds a feature, restructures something, or changes the toolchain usually
settled a question along the way -- which option was taken, and what it cost. That reasoning
is invisible in the diff, and once the conversation that produced it is gone, nobody can tell
whether an alternative was rejected or never considered. `spec/` is where it survives.

This hook runs after the commit succeeds and injects a reminder when a decision-bearing
commit touched no rules. It never blocks: plenty of `feat` commits genuinely settle nothing,
and a gate that fires on false positives gets worked around rather than heeded. Judging
whether a decision was made is the agent's job -- this only makes sure the question is asked
while the reasoning is still in context.

A narrower signal was tried here and cut. Pairing each changed file with the spec files its
comments cite looks sharper, but it fires on every edit to any file that merely mentions a
rule for background -- which is most edits. Citation is not co-evolution. The pair graph is
still built, in .mise/tasks/refs, where it backs a check that has no false positives at all:
a reference pointing at a file that does not exist is unambiguously broken.

Dependencies are limited to the standard library, for the reason given in spec/toolchain.md.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

sys.dont_write_bytecode = True

from jj_command import invocations

# Types that usually carry a decision. Absent by design: docs, test, style, chore, fix, ci --
# a bug fix records its cause at the test, not in the rules.
DECISION_TYPES = ("feat", "refactor", "build", "perf")
RULE_PATHS = ("spec/", "CLAUDE.md", "AGENTS.md")


def run(args: list[str]) -> str:
	result = subprocess.run(args, capture_output=True, text=True, timeout=30)
	return result.stdout if result.returncode == 0 else ""


def writes_commit(command: str) -> bool:
	"""Whether one shell segment asks jj to create a commit."""
	return any(subcommand in ("commit", "ci") for subcommand, _ in invocations(command))


def context(payload: dict) -> str:
	"""Return the model-visible reminder for one lifecycle payload, if any."""
	command = payload.get("tool_input", {}).get("command", "")
	# Codex matches the tool name but has no handler-level command predicate, so the portable
	# hook owns the narrower selection itself.
	if not writes_commit(command):
		return ""

	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	subject = run(["jj", "log", "--no-graph", "-r", "@-", "-T", "description.first_line()"])
	commit_type = subject.split(":", 1)[0].split("(", 1)[0].strip()
	if commit_type not in DECISION_TYPES:
		return ""

	summary = run(["jj", "diff", "--summary", "-r", "@-"])
	paths = [line.split(maxsplit=1)[1] for line in summary.splitlines() if " " in line]
	if not paths:
		return ""

	if any(p.startswith(RULE_PATHS) for p in paths):
		return ""

	lines = [
		f"Committed `{subject.strip()}` without touching spec/ or CLAUDE.md.",
		"If this change settled a question -- chose one option over another, accepted a "
		"tradeoff, added a tool, established a convention -- record the decision and its "
		"reasoning now, while it is still in context. Put it in the commit itself rather than a "
		"follow-up, so the rule and the change it came from stay together. If it settled "
		"nothing, say so briefly and move on. spec/agent-protocol.md defines what belongs in "
		"spec and what belongs in a code comment instead.",
	]

	return "\n".join(lines)


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
