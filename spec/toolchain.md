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

Pushing is the user's to run. Do not offer it at the end of a task and do not ask whether to
push -- the user handles it. Commit and move the bookmark; stop there.

To get git-like auto-advance instead, enable it in config:

```toml
[experimental-advance-branches]
enabled-branches = ["glob:*"]
```

The key is still named `branches` rather than `bookmarks` -- a leftover from before jj renamed
the concept.

## Tool versions

mise owns every tool, not just language runtimes. Linters, formatters, and CLIs go in
`mise.toml` too -- currently node, rust, pnpm, jj, oxlint, oxfmt. Do not install a tool
globally, and do not add a developer tool as a `devDependencies` entry when mise can carry it.
One manifest, one answer to "what version is this".

Three consequences worth knowing:

- mise activation is directory-scoped, so these versions resolve only inside this repo.
- Tools installed via mise have no `node_modules` presence, so JSON `$schema` keys must point
  at a hosted URL rather than a local path. `.oxlintrc.json` does this.
- A mise-installed tool cannot load a plugin that lives in `node_modules`. oxfmt's `svelte`
  option needs to resolve `svelte/compiler`, so it stays out of `.oxfmtrc.json` until an app
  actually depends on svelte. Enabling it early does not fail quietly -- it aborts the whole
  format run. The same trap waits for any other plugin-shaped option.

Prefer the mise registry short name (`oxlint`) over a backend-qualified one (`npm:oxlint`);
both resolve to the same package, and the short form keeps `mise.toml` readable.

## Dependency policy

`pnpm-workspace.yaml` sets `minimumReleaseAge: 1440` -- a package version must have been
public for 24 hours before it can be installed, transitive dependencies included. Most npm
compromises are found and yanked inside that window.

`minimumReleaseAgeExclude` holds the exemptions. Keep the list short: every entry is an
accepted risk, justified only when the wait costs more than it protects.

## Default stacks

| Area                      | Stack                      |
| ------------------------- | -------------------------- |
| Binaries and applications | Rust + Cargo               |
| Web                       | TypeScript + pnpm + Svelte |

These are defaults, not a whitelist. Reaching outside them is a structural decision and
belongs to the user -- see [agent-protocol.md](agent-protocol.md).
