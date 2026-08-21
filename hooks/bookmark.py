#!/usr/bin/env python3
"""Shared Stop hook: refuse to hand back a task with commits `main` has not been moved over.

spec/commits.md requires an agent that finished a task to commit it *and* move the `main`
bookmark before handing the result back. The two existing hooks cannot see that moment. Both
are scoped to the Bash tool, and a turn ends wherever the agent stops calling tools -- often
straight after the last `jj describe`, with nothing running afterwards to be inspected. So the
rule held only for as long as the agent remembered it, and seventeen commits went by in one
session without the bookmark moving once.

Stop is that moment, stated as an event. This is the only hook here that is not attached to a
tool call.

**It refuses; it never moves the bookmark itself.** Doing the move here would be the same
behaviour spec/toolchain.md turns off on purpose -- jj ships auto-advance as
`experimental-advance-branches`, and the reason it stays disabled is that a bookmark which only
moves when told to is a bookmark whose position means something. A hook that advanced `main`
would restore auto-advance under another name, and worse, without a record: the position would
no longer be a claim anybody made. Refusing keeps the claim, and the claim keeps the meaning.

What counts is a commit that is **described and not empty**. Undescribed working-copy changes
are the shape partial or blocked work is supposed to have, and spec/commits.md says it stays
uncommitted -- so this must not fire on it, or the rule that protects unfinished work would
start demanding it be published.

Dependencies are limited to the standard library, for the reason given in spec/toolchain.md.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

sys.dont_write_bytecode = True

BOOKMARK = "main"

# Described, non-empty commits reachable from the working copy that the bookmark does not cover.
# `empty()` drops the working copy when it holds nothing; the template drops anything still
# without a description, which is what unfinished work looks like.
REVSET = f"::@ ~ ::{BOOKMARK} ~ empty()"
TEMPLATE = 'if(description, change_id.short() ++ "\\t" ++ description.first_line() ++ "\\n")'


def unmerged() -> list[tuple[str, str]]:
	"""Commits ahead of the bookmark, newest last. Empty when jj cannot answer."""
	try:
		result = subprocess.run(
			["jj", "log", "--no-graph", "--ignore-working-copy", "-r", REVSET, "-T", TEMPLATE],
			capture_output=True,
			text=True,
			timeout=30,
		)
	except (OSError, subprocess.SubprocessError):
		return []
	# A repository without the bookmark, or without jj at all, is not this hook's business to
	# report on. Both leave a non-zero status, and both mean there is nothing to enforce.
	if result.returncode != 0:
		return []

	found = []
	for line in result.stdout.splitlines():
		if "\t" in line:
			change, subject = line.split("\t", 1)
			found.append((change, subject))
	return found


def response(payload: dict) -> dict:
	"""Return the continuation decision for one lifecycle payload, if any."""
	# Set when this hook already blocked once and the agent is being asked to continue. Blocking
	# again on the next stop would be a loop with no exit, since the hook cannot tell a refusal
	# to move the bookmark from an inability to.
	if payload.get("stop_hook_active"):
		return {}

	cwd = payload.get("cwd")
	if cwd and os.path.isdir(cwd):
		os.chdir(cwd)

	ahead = unmerged()
	if not ahead:
		return {}

	listed = "\n".join(f"  {change}  {subject}" for change, subject in ahead)
	reason = (
		f"`{BOOKMARK}` does not cover {len(ahead)} commit(s) on the way to the working copy:\n"
		f"{listed}\n\n"
		f"spec/commits.md: a finished task is committed *and* the bookmark is moved before the "
		f"result is handed back. Committed work is finished work -- partial work is meant to "
		f"stay uncommitted -- so anything listed above belongs under the bookmark.\n\n"
		f"Run `jj bookmark move {BOOKMARK} --to @-` (or `--to @` when the working copy is itself "
		f"the described commit), then finish the turn. If the move is refused as sideways, another "
		f"workspace advanced `{BOOKMARK}` meanwhile: `jj rebase -d {BOOKMARK}`, re-run `mise run "
		f"verify`, then move -- see spec/commits.md. Do not push: that is the user's to run, per "
		f"spec/toolchain.md."
	)

	return {"decision": "block", "reason": reason}


def main() -> int:
	try:
		payload = json.load(sys.stdin)
	except (json.JSONDecodeError, ValueError):
		return 0
	result = response(payload)
	if result:
		print(json.dumps(result))
	return 0


if __name__ == "__main__":
	sys.exit(main())
