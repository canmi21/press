#!/usr/bin/env python3
"""Move paid translations from legacy ids to canonical article segment ids."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
LAYOUT = ROOT / "data/build/segments.json"
NORMALISER = ROOT / "apps/cms/scripts/normalize-markdown.ts"
ENTRY = re.compile(r"^  ([0-9a-f]{32}):\n?$")


def run(command: list[str], **kwargs: Any) -> subprocess.CompletedProcess[Any]:
	return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def normalise(path: Path) -> str:
	result = run(
		["node", str(NORMALISER), str(path)],
		stdout=subprocess.PIPE,
		stderr=subprocess.PIPE,
		text=True,
	)
	return result.stdout


def source_of(article: bytes, span: dict[str, Any]) -> str:
	return article[span["start"] : span["end"]].decode("utf-8")


def is_directive(article: bytes, span: dict[str, Any]) -> bool:
	return span["region"] == "body" and source_of(article, span).strip().startswith("::")


def sidecar_blocks(text: str) -> tuple[str, dict[str, str]]:
	lines = text.splitlines(keepends=True)
	starts = [(index, match.group(1)) for index, line in enumerate(lines) if (match := ENTRY.match(line))]
	if not starts:
		return text, {}
	prefix = "".join(lines[: starts[0][0]])
	blocks: dict[str, str] = {}
	for offset, (start, segment_id) in enumerate(starts):
		end = starts[offset + 1][0] if offset + 1 < len(starts) else len(lines)
		blocks[segment_id] = "".join(lines[start:end])
	return prefix, blocks


def rewrite_id(block: str, segment_id: str) -> str:
	lines = block.splitlines(keepends=True)
	ending = "\n" if lines[0].endswith("\n") else ""
	lines[0] = f"  {segment_id}:{ending}"
	return "".join(lines)


def main() -> int:
	old_layout = json.loads(LAYOUT.read_text(encoding="utf-8"))
	old_articles: dict[str, bytes] = {}
	for relative in old_layout["articles"]:
		path = ROOT / "contents" / relative
		old_articles[relative] = path.read_bytes()
		path.write_text(normalise(path), encoding="utf-8")

	run(["cargo", "run", "-q", "-p", "cms", "--", "segments"])
	new_layout = json.loads(LAYOUT.read_text(encoding="utf-8"))

	mappings: dict[str, dict[str, str]] = {}
	directives: dict[str, set[str]] = {}
	for relative, old_spans in old_layout["articles"].items():
		old_article = old_articles[relative]
		directive_ids = {
			span["id"] for span in old_spans if is_directive(old_article, span)
		}
		kept = [span for span in old_spans if not is_directive(old_article, span)]
		new_spans = new_layout["articles"].get(relative, [])
		if len(kept) != len(new_spans):
			raise RuntimeError(
				f"{relative}: normalisation changed segment structure "
				f"({len(kept)} old non-directives, {len(new_spans)} new)"
			)
		mapping: dict[str, str] = {}
		for old, new in zip(kept, new_spans, strict=True):
			if old["region"] != new["region"]:
				raise RuntimeError(f"{relative}: segment regions no longer align")
			previous = mapping.setdefault(old["id"], new["id"])
			if previous != new["id"]:
				raise RuntimeError(f"{relative}: one legacy id maps to multiple canonical ids")
		mappings[relative] = mapping
		directives[relative] = directive_ids

	migrated = 0
	removed = 0
	unmatched: list[str] = []
	for relative, mapping in mappings.items():
		article = ROOT / "contents" / relative
		sidecar = article.with_suffix(".i18n.yaml")
		if not sidecar.exists():
			continue
		prefix, blocks = sidecar_blocks(sidecar.read_text(encoding="utf-8"))
		output: dict[str, str] = {}
		for old_id, block in blocks.items():
			if old_id in directives[relative]:
				removed += 1
				continue
			new_id = mapping.get(old_id)
			if new_id is None:
				unmatched.append(f"{sidecar.relative_to(ROOT)}: {old_id}")
				new_id = old_id
			if new_id in output:
				raise RuntimeError(f"{sidecar.relative_to(ROOT)}: canonical id collision at {new_id}")
			output[new_id] = rewrite_id(block, new_id)
			migrated += old_id != new_id
		sidecar.write_text(
			prefix + "".join(output[key] for key in sorted(output)), encoding="utf-8"
		)

	print(f"normalised {len(old_articles)} articles")
	print(f"migrated {migrated} sidecar entries")
	print(f"removed {removed} directive entries")
	if unmatched:
		print(f"unmatched {len(unmatched)} sidecar entries:", file=sys.stderr)
		for finding in unmatched:
			print(f"  {finding}", file=sys.stderr)
		return 1
	print("unmatched 0 sidecar entries")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
