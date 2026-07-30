#!/usr/bin/env python3
"""PostToolUse hook for `jj commit`: notice decisions that were not written down.

A commit that adds a feature, restructures something, or changes the toolchain usually
settled a question along the way -- which option was taken, and what it cost. That reasoning
is invisible in the diff, and once the conversation that produced it is gone, nobody can tell
whether an alternative was rejected or never considered. `spec/` is where it survives.

This hook runs after the commit succeeds and injects a reminder when a decision-bearing
commit touched no rules. It never blocks: plenty of `feat` commits genuinely settle nothing,
and a gate that fires on false positives gets worked around rather than heeded. Judging
whether a decision was made is the agent's job -- this only makes sure the question is asked
while the reasoning is still in context.

Dependencies are limited to the standard library, for the reason given in spec/toolchain.md.
"""

import json
import os
import subprocess
import sys

# Types that usually carry a decision. Absent by design: docs, test, style, chore, fix, ci --
# a bug fix records its cause at the test, not in the rules.
DECISION_TYPES = ("feat", "refactor", "build", "perf")
RULE_PATHS = ("spec/", "CLAUDE.md", "AGENTS.md")


def run(args: list[str]) -> str:
	result = subprocess.run(args, capture_output=True, text=True, timeout=30)
	return result.stdout if result.returncode == 0 else ""


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0

	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	subject = run(["jj", "log", "--no-graph", "-r", "@-", "-T", "description.first_line()"])
	commit_type = subject.split(":", 1)[0].split("(", 1)[0].strip()
	if commit_type not in DECISION_TYPES:
		return 0

	summary = run(["jj", "diff", "--summary", "-r", "@-"])
	paths = [line.split(maxsplit=1)[1] for line in summary.splitlines() if " " in line]
	if not paths:
		return 0

	touched_rules = any(p.startswith(RULE_PATHS) for p in paths)
	if touched_rules:
		return 0

	print(
		json.dumps(
			{
				"hookSpecificOutput": {
					"hookEventName": "PostToolUse",
					"additionalContext": (
						f"Just committed `{subject.strip()}` without touching spec/ or CLAUDE.md.\n"
						f"If this change settled a question -- picked one option over another, "
						f"accepted a tradeoff, added a tool, or established a convention -- record "
						f"the decision and its reasoning now, while it is still in context. "
						f"Amend the same commit with `jj describe` or add to it directly; a "
						f"follow-up commit separates the rule from the change it came from.\n"
						f"If it settled nothing, say so briefly and move on. See "
						f"spec/agent-protocol.md for what belongs in spec and what does not."
					),
				}
			}
		)
	)
	return 0


if __name__ == "__main__":
	sys.exit(main())
