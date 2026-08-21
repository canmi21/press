#!/usr/bin/env python3
"""One lifecycle-hook entrypoint shared by Claude Code and Codex.

Both harnesses send the same event name, working directory, tool name, tool input, and stop-loop
flag for the events this repository uses. Their checked-in settings therefore call this file and
nothing vendor-specific owns policy or output composition. The policy modules stay split by
responsibility; this entrypoint is the one place that routes and combines them.

Dependencies are limited to the standard library, and annotations stay deferred, for the system
Python compatibility required by spec/toolchain.md.
"""

from __future__ import annotations

import json
import sys

sys.dont_write_bytecode = True

import bookmark
import commit
import spec_check
import spec_diff


def post_tool_context(payload: dict) -> str:
	"""Combine every applicable post-tool observation into one valid hook response."""
	return "\n\n".join(
		message for message in (spec_check.context(payload), spec_diff.context(payload)) if message
	)


def handle(payload: dict) -> int:
	event = payload.get("hook_event_name")
	if event == "PreToolUse":
		return commit.handle(payload)
	if event == "PostToolUse":
		message = post_tool_context(payload)
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
	if event == "Stop":
		result = bookmark.response(payload)
		if result:
			print(json.dumps(result))
	return 0


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0
	return handle(payload)


if __name__ == "__main__":
	sys.exit(main())
