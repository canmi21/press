# Toolchain

## Version control

jj (Jujutsu), colocated with git -- `.jj` and `.git` sit side by side in the repo root. Use
`jj` for everyday work; reach for `git` only where jj has no equivalent.

Remote `origin` is <https://github.com/canmi21/workspace.git>, default branch `main`.

jj has no branches, only bookmarks, and bookmarks do not advance on their own. `jj commit`
leaves `main` where it was; move it explicitly:

```bash
jj bookmark move main --to @-
jj git push -b main
```

To get git-like auto-advance instead, enable it in config:

```toml
[experimental-advance-branches]
enabled-branches = ["glob:*"]
```

The key is still named `branches` rather than `bookmarks` -- a leftover from before jj renamed
the concept.

## Tool versions

mise owns every toolchain version; see `mise.toml` in the repo root. Do not install a language
runtime outside it. mise activation is directory-scoped, so tools resolve to the versions
pinned here only inside this repo.

## Default stacks

| Area | Stack |
| --- | --- |
| Binaries and applications | Rust + Cargo |
| Web | TypeScript + pnpm + Svelte |

These are defaults, not a whitelist. Reaching outside them is a structural decision and
belongs to the user -- see [agent-protocol.md](agent-protocol.md).
