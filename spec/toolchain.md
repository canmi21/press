# Toolchain

## Shell

The user's interactive shell is **fish**, configured in `~/.config/fish/config.fish`.

This matters in two directions, and they are opposite:

- **A command written for the user to run** must be fish syntax. `set -gx KEY value`, not
  `export KEY=value`. Command substitution is `(cmd)`, not `$(cmd)`.
- **A command an agent runs through its own tooling** goes through that tool's shell, which
  on this machine is zsh, so it must be POSIX syntax. Writing fish syntax there fails with a
  parse error, and writing bash syntax into a fish snippet fails the same way in reverse.

Two more platform facts that cost time when forgotten: this is macOS, so `sed` is BSD sed
and does not support `\b` word boundaries -- use `perl -pi -e` for word-boundary replaces.
And mise activation is directory-scoped and shell-based, so a command run from a shell
without mise sees none of the pinned versions.

## Secrets

Credentials live in `secrets.json`, encrypted with [sops](https://getsops.io) to an
[age](https://age-encryption.org) key and **committed that way**. mise decrypts it on entering
the directory, so every tool reads the values from the environment and nothing keeps a second
copy. `.sops.yaml` names the recipient public key, which is safe to commit -- it encrypts and
cannot decrypt.

The private key lives at `~/.config/sops/age/keys.txt`, outside the repo, and belongs in a
password manager as well. Losing it costs a round of credential rotation, not the repo.

Committing the ciphertext rather than gitignoring a plaintext `.env` is what makes a fresh
machine reproducible: restore one key, clone, and every credential the project needs is
already there. A gitignored `.env` leaves no record of _which_ secrets exist, so setting up
again means rediscovering them one failure at a time.

Only genuinely secret values go in. Facts like `RCLONE_CONFIG_R2_TYPE = "s3"` stay in
`mise.toml`, so a diff of the encrypted file always means a credential actually changed.

**A value the browser must have cannot be a secret.** Anything compiled into a client bundle
-- an analytics token, a Sentry DSN, a publishable API key -- is readable from devtools by
anyone who loads the page. Storing it encrypted does not hide it; it only hides from the
reader of this repository that it is already public. Those go in `libs/urls` or plain config,
labelled for what they are.

The same credential can be secret elsewhere. The API worker's Sentry DSN is a different
project that never reaches a browser, so it stays a wrangler secret. What decides is exposure,
not the kind of thing it is.

### Tokens are scoped to one bucket

An R2 API token is created for a single bucket, not for the account. The sync task runs
`rclone sync`, which deletes whatever the source does not have, so a token that can reach a
second bucket makes a mistyped remote destructive there too.

Scoping is the same move as leaving a private bucket unbound from any worker: the boundary
holds because the credential cannot cross it, not because whoever typed the command was
careful. Pulling data out of an old bucket therefore uses a separate read-only token -- read
access is all that job needs, and it cannot damage the only copy of anything.

This is visible in normal use: `rclone lsd r2:` returns 403 because listing buckets is an
account-level operation the token deliberately lacks. Naming the bucket works; enumerating
them does not.

### Three things that will bite

**JSON, not dotenv.** mise parses a `.env` as plain dotenv before looking for sops metadata
and fails on the age block. JSON puts that metadata under a `sops` key it recognises.

**No empty placeholders.** sops leaves `""` untouched, and mise then rejects the whole file
because a value lacks the `ENC[` prefix. Write `"unset"` instead of `""`.

**The circular trap.** A broken `secrets.json` makes `mise exec` fail in this directory --
including for the sops you would use to repair it. Call sops by absolute path when fixing it.

`mise run secrets` refuses any file `.sops.yaml` claims that is not actually encrypted, and
runs first in `verify` because it is the only check there guarding something irreversible: a
plaintext credential reaching the remote is not undone by deleting the file.

## Workers answer on custom domains only

`workers_dev` and `preview_urls` are off everywhere. Every generated hostname is another route
to the same worker, reached without whatever sits in front of the custom domain, and nobody
watches those addresses.

The cost is real and accepted: there is no URL to open between uploading a version and
promoting it, so a deploy is the first time the code meets production. What replaces that
check is `wrangler dev`, which runs the same code against the same bindings, plus the fact
that a worker with no route configured serves nothing until a domain is pointed at it by
hand.

That last point is what makes replacing a worker safe. A first deploy under a new name is
inert -- it creates the worker and attracts no traffic. Deploying over an existing worker of
the same name replaces it in place and keeps its routes and custom domains attached, so
replacing one never requires deleting it first. Nothing is deleted until whatever supersedes
it has been seen serving real traffic.

## Deploying is a consequence of pushing

Cloudflare builds from the connected repository, so a push is what ships. Nobody runs a deploy
by hand as the normal path, and an agent runs one neither by hand nor on request -- pushing is
the user's, and so is everything downstream of it.

The `deploy-*` tasks stay as a fallback for the case where the platform's build is broken or a
worker has to be created before its settings exist. Using one means production now holds
something no commit accounts for, which is worth doing knowingly and not by habit.

Two things follow from CI holding the build. It compiles what is in git and derives nothing --
see [workspace.md](architecture/workspace.md) -- and the toolchain has to be pinned in files CI can
read, because `mise.toml` is not one of them.

## Dev ports are pinned

Every dev server binds a fixed port and **fails when that port is taken**. Vite gets
`strictPort: true`; anything else refuses to fall back. Auto-incrementing to the next free
port is never acceptable.

The reason is not tidiness. A tool that drifts to the next port starts a second instance
silently, and a second instance of something that writes to `data/` means two processes
fetching and overwriting in the same directory. The port collision is the cheapest mutex
available -- the operating system provides it for free, and it fails loudly at the only moment
anyone can act on it.

A port both a TypeScript tool and a Rust binary need is declared in `mise.toml` under `[env]`,
not in `libs/urls`. The single-source rule asks for one place to edit, not one particular
file, and a TypeScript library cannot be read by a Rust process -- putting a cross-language
fact there would force the duplication the rule exists to prevent. URLs only the TypeScript
side resolves still belong in [workspace.md](architecture/workspace.md)'s URL map.

## The Tauri dev watcher is told where the frontend is

`tauri dev` watches every directory Cargo reaches through a local `path` dependency, not just
`src-tauri`. `cms-app` depends on `cms = { path = ".." }`, so the watched set includes all of
`apps/cms` -- which is also where the frontend sources live. Editing a `.ts` or `.css` file
therefore triggered a cargo rebuild and an app restart, and the restart discarded the Vite hot
update that had already applied. The symptom reads as "HMR is broken"; HMR was fine and was
being overwritten a second later.

The fix is a `.taurignore` listing the paths Vite owns. **It belongs beside the crate being
watched -- `apps/cms/.taurignore` -- and not in `src-tauri`,** which is where the obvious guess
puts it. Patterns are gitignore syntax resolved against the directory holding the file, so a
copy in `src-tauri` reads `client/` as `src-tauri/client/`, matches nothing, and fails silently:
the app goes on restarting and the file looks like it was ignored rather than misplaced. Both
directions were measured by touching a file and watching the process id.

Ignoring too much fails the same way round the other side, so the list names frontend paths
explicitly rather than excluding everything but `src/`. A Rust edit must still rebuild.

This is a consequence of the frontend and the Rust crate sharing one directory. An app whose
crate lives entirely under `src-tauri` never sees it, and is not the layout here.

## Version control

jj (Jujutsu), colocated with git -- `.jj` and `.git` sit side by side in the repo root. Use
`jj` for everyday work; reach for `git` only where jj has no equivalent.

The remote is `origin` and the default branch is `main`; the URL lives in git's own config,
not here, because a URL written into a spec document is a second place for it to go stale.

jj has no branches, only bookmarks, and **bookmarks do not advance on their own**. `jj commit`
leaves `main` where it was, so moving it is a separate, deliberate step (`jj bookmark move`).
Auto-advance is available as an opt-in jj config -- `experimental-advance-branches`, still named
for branches from before the rename -- and is deliberately not enabled. A bookmark that only
moves when told to is a bookmark whose position means something.

Pushing is the user's to run. Do not offer it at the end of a task and do not ask whether to
push. Commit and move the bookmark; stop there.

## Tool versions

mise owns every tool, not just language runtimes. Linters, formatters, and CLIs belong in
`mise.toml` alongside the runtimes -- read that file for the current set. Do not install a tool
globally, and do not add a developer tool as a `devDependencies` entry when mise can carry it.
One manifest, one answer to "what version is this".

Three consequences worth knowing:

- mise activation is directory-scoped, so these versions resolve only inside this repo.
- Tools installed via mise have no `node_modules` presence, so JSON `$schema` keys must point
  at a hosted URL rather than a local path. `.oxlintrc.json` does this.
- A mise-installed tool cannot load a plugin that lives in `node_modules` -- node resolves
  modules from the requiring file's own path, so a binary under mise's install tree never
  reaches this repo's packages however it is invoked. That is why oxfmt moved to
  `package.json` when its `svelte` option (which resolves `svelte/compiler`) was switched on:
  the option is not gated on an app depending on svelte, it is structurally unreachable from a
  mise install. Enabling a plugin-shaped option on a mise-installed tool does not fail
  quietly -- it aborts the whole format run. `mise.toml` adds `node_modules/.bin` to the
  directory-scoped path so `oxfmt` stays invocable exactly as before.

Prefer the mise registry short name (`oxlint`) over a backend-qualified one (`npm:oxlint`);
both resolve to the same package, and the short form keeps `mise.toml` readable.

### The JavaScript toolchain lives in package.json

`typescript`, `vite`, and `vitest` are root `devDependencies`, not mise tools. mise's registry
carries none of them, and that is the right outcome rather than a gap:

- `vitest` is imported by the test files themselves (`import { it } from 'vitest'`), so it has
  to be resolvable from `node_modules`. A binary on `$PATH` cannot satisfy an import.
- `typescript` is looked up out of `node_modules` by editors, language servers, and vite.
- `vite` is what vitest runs on, and every web app will depend on it directly.

The general rule behind the exception: **a tool belongs to the package manager whose
resolution model it participates in.** oxlint reads files and writes files, so mise carries
it. oxfmt did too until its `svelte` option made it resolve `svelte/compiler` through node's
module graph -- participation that moved it here, with `svelte` beside it to satisfy that
peer dependency. Anything the code itself imports, or that another JS tool resolves by module
name, belongs in `package.json`.

Task entry points stay in mise regardless of where the tool lives, so there is one place to
look for "how do I run this" whichever ecosystem the binary came from. `mise tasks` lists them;
`mise run verify` is the one a change has to pass. Tasks are defined in `mise.toml`, except
where a task needs real logic, which goes in `.mise/tasks/` as an executable file.

### The other exception: hook scripts

`hooks/` is deliberately outside mise's reach. The real scripts live there once and use
`#!/usr/bin/env python3` with nothing beyond the standard library. Vendor directories contain
only the glue that binds those scripts to each harness: `.claude/settings.json` and
`.codex/hooks.json`.

The reason is the failure mode. mise activation is shell-scoped, and a hook is launched by
the agent harness rather than by an interactive shell. If a hook's interpreter came from
mise and mise were not active, the hook would fail to start -- and a hook that fails to
start enforces nothing while looking exactly like a hook that passed. Silent
non-enforcement is worse than no enforcement, because it is believed.

Verified: the commit hook runs unchanged on macOS's system Python 3.9.6 and on the current
Homebrew and mise builds, because it touches only `json`, `re`, `shlex`, `subprocess`, and
`os`. Version pinning would buy nothing here and would cost the guarantee that it always
starts.

This exception covers hook scripts only. Everything a human or an agent invokes on purpose
still belongs in `mise.toml`.

## Dependency policy

`pnpm-workspace.yaml` sets `minimumReleaseAge: 1440` -- a package version must have been
public for 24 hours before it can be installed, transitive dependencies included. Most npm
compromises are found and yanked inside that window.

`minimumReleaseAgeExclude` holds the exemptions. Keep the list short: every entry is an
accepted risk, justified only when the wait costs more than it protects.

**Cargo has no equivalent delay and is not getting one.** The question is asked by the npm rule
sitting here, and the answer is the user's: they watch that ecosystem themselves. So a crate
published minutes ago can enter a build, and when one breaks it, the fix is a lockfile pin at the
version that worked -- named in the commit, with no manifest constraint and no `[patch]`, so that
a later update takes the repair as soon as it exists.

The rule bites in a way worth expecting: a range that would otherwise resolve to the newest
release quietly resolves to the newest _mature_ one instead. That is the policy working, not a
resolution failure, and the declared range should still name the version the fix landed in
rather than whatever happens to be installable today. It becomes an error only when nothing in
range is old enough, which is what an exemption is for.

### An advisory in a development-only dependency is not a production one

Where a vulnerable package runs decides how much it costs. A deployed Worker uses workerd's own
fetch and never loads the HTTP client its build tooling depends on; a bundler plugin's glob
matcher never sees a request. Fixing those clears an alert list, which is worth something --
a list nobody can read is a list that hides the next real one -- but it is not the same work as
patching something on the request path, and a security fix that says which of the two it is
saves the next reader from having to re-derive it.

That ranking decides how much risk a fix may carry. Forcing a patch across an exact upstream pin
is reasonable for a clean list and unreasonable if it can break a build; where an override does
that, its comment says what was verified afterwards.

## Default stacks

| Area                      | Stack                      |
| ------------------------- | -------------------------- |
| Binaries and applications | Rust + Cargo               |
| Web                       | TypeScript + pnpm + Svelte |

These are defaults, not a whitelist. Reaching outside them is a structural decision and
belongs to the user -- see [agent-protocol.md](agent-protocol.md).
