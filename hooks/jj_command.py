"""Find jj invocations inside the shell command shape agent hooks receive."""

from __future__ import annotations

import os
import shlex

SEPARATORS = {"&&", ";", "||", "|"}
GLOBAL_OPTIONS_WITH_VALUE = {
	"-R",
	"--repository",
	"--at-operation",
	"--color",
	"--config",
	"--config-file",
}


def _segments(command: str) -> list[list[str]]:
	try:
		tokens = shlex.split(command)
	except ValueError:
		return []

	segments: list[list[str]] = [[]]
	for token in tokens:
		if token in SEPARATORS:
			segments.append([])
		else:
			segments[-1].append(token)
	return [segment for segment in segments if segment]


def _is_command(segment: list[str], at: int) -> bool:
	if at == 0:
		return True
	prefix = segment[:at]
	if prefix[-1] == "--":
		return True
	if all("=" in token and not token.startswith("=") for token in prefix):
		return True
	return os.path.basename(prefix[0]) in ("command", "env")


def invocations(command: str) -> list[tuple[str, list[str]]]:
	"""Return each jj subcommand and its remaining arguments."""
	found = []
	for segment in _segments(command):
		for at, token in enumerate(segment):
			if os.path.basename(token) != "jj" or not _is_command(segment, at):
				continue
			subcommand_at = at + 1
			while subcommand_at < len(segment):
				candidate = segment[subcommand_at]
				if candidate in GLOBAL_OPTIONS_WITH_VALUE:
					subcommand_at += 2
					continue
				if candidate.startswith("-"):
					subcommand_at += 1
					continue
				found.append((candidate, segment[subcommand_at + 1 :]))
				break
			break
	return found
