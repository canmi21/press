from __future__ import annotations

import contextlib
import io
import json
import pathlib
import subprocess
import unittest
from unittest import mock

import run

ROOT = pathlib.Path(__file__).resolve().parents[1]
ENTRYPOINT = ROOT / "hooks" / "run.py"
SYSTEM_PYTHON = "/usr/bin/python3"
# jj answers about the nearest repository, which is a sub-repo whenever the shell sits inside
# one, so the entrypoint is found by walking up to the checkout that has hooks/.
# See spec/architecture/repos.md.
COMMAND = (
	'd=$(jj workspace root 2>/dev/null || pwd); '
	'while [ ! -f "$d/hooks/run.py" ] && [ "$d" != / ]; do d=$(dirname "$d"); done; '
	'[ -f "$d/hooks/run.py" ] && exec /usr/bin/python3 "$d/hooks/run.py" || exit 0'
)


def invoke(payload: dict) -> subprocess.CompletedProcess:
	return subprocess.run(
		[SYSTEM_PYTHON, str(ENTRYPOINT)],
		input=json.dumps(payload),
		capture_output=True,
		text=True,
		cwd=ROOT,
		check=False,
	)


class SharedEntrypointTest(unittest.TestCase):
	def test_both_vendor_adapters_are_identical(self) -> None:
		with (ROOT / ".claude" / "settings.json").open() as source:
			claude = json.load(source)["hooks"]
		with (ROOT / ".codex" / "hooks.json").open() as source:
			codex = json.load(source)["hooks"]

		self.assertEqual(claude, codex)
		for groups in codex.values():
			for group in groups:
				self.assertEqual(len(group["hooks"]), 1)
				self.assertEqual(group["hooks"][0]["command"], COMMAND)

	def test_codex_post_tool_payload_runs_under_system_python(self) -> None:
		result = invoke(
			{
				"session_id": "codex-session",
				"turn_id": "codex-turn",
				"cwd": str(ROOT),
				"hook_event_name": "PostToolUse",
				"model": "gpt-test",
				"permission_mode": "default",
				"tool_name": "Bash",
				"tool_use_id": "codex-tool",
				"tool_input": {"command": "pwd"},
				"tool_response": {"output": str(ROOT)},
			}
		)

		self.assertEqual(result.returncode, 0, result.stderr)
		self.assertEqual(result.stdout, "")

	def test_claude_post_tool_payload_runs_under_system_python(self) -> None:
		result = invoke(
			{
				"session_id": "claude-session",
				"transcript_path": None,
				"cwd": str(ROOT),
				"permission_mode": "default",
				"hook_event_name": "PostToolUse",
				"tool_name": "Bash",
				"tool_input": {"command": "pwd"},
				"tool_response": "ok",
				"tool_use_id": "claude-tool",
			}
		)

		self.assertEqual(result.returncode, 0, result.stderr)
		self.assertEqual(result.stdout, "")

	def test_adapter_command_resolves_from_a_repository_subdirectory(self) -> None:
		result = subprocess.run(
			COMMAND,
			input=json.dumps(
				{
					"cwd": str(ROOT / "apps" / "site"),
					"hook_event_name": "PostToolUse",
					"tool_name": "Bash",
					"tool_input": {"command": "pwd"},
				}
			),
			capture_output=True,
			text=True,
			cwd=ROOT / "apps" / "site",
			shell=True,
			check=False,
		)

		self.assertEqual(result.returncode, 0, result.stderr)
		self.assertEqual(result.stdout, "")

	def test_pre_tool_rejection_keeps_the_shared_exit_contract(self) -> None:
		result = invoke(
			{
				"cwd": str(ROOT),
				"hook_event_name": "PreToolUse",
				"tool_name": "Bash",
				"tool_input": {"command": "jj commit -m 'Not conventional'"},
			}
		)

		self.assertEqual(result.returncode, 2)
		self.assertIn("Commit message rejected", result.stderr)

	def test_stop_loop_guard_is_silent(self) -> None:
		result = invoke(
			{
				"cwd": str(ROOT),
				"hook_event_name": "Stop",
				"stop_hook_active": True,
			}
		)

		self.assertEqual(result.returncode, 0, result.stderr)
		self.assertEqual(result.stdout, "")

	def test_post_tool_context_is_one_json_document(self) -> None:
		output = io.StringIO()
		with (
			mock.patch.object(run.spec_check, "context", return_value="first"),
			mock.patch.object(run.spec_diff, "context", return_value="second"),
			contextlib.redirect_stdout(output),
		):
			status = run.handle({"hook_event_name": "PostToolUse"})

		self.assertEqual(status, 0)
		parsed = json.loads(output.getvalue())
		self.assertEqual(
			parsed["hookSpecificOutput"],
			{"hookEventName": "PostToolUse", "additionalContext": "first\n\nsecond"},
		)


if __name__ == "__main__":
	unittest.main()
