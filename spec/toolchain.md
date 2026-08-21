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

The pinned numbers are the **base** workspace's. An overlay workspace binds each of them
shifted by its slot -- see [Parallel workspaces](#parallel-workspaces) -- so the pin still
holds per directory, and what it protects still holds per machine: the CMS, the one process
that writes `data/`, keeps `CMS_PORT` in every workspace and does not shift. Two overlays
that both try to run it collide, which is the mutex doing its job.

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

How `main` advances when more than one workspace is committing is [commits.md](commits.md)'s;
what the workspaces are is the next section's.

## Parallel workspaces

More than one agent works on this repository at once, each in its own jj workspace. There are
no feature bookmarks and no branches: every workspace commits onto the one linear `main`,
and the history git sees is the same as it would be with one author. What jj adds is that
each workspace has its own working copy and its own `@`, all visible in one `jj log`, on one
change graph, in one operation log. Coordination happens through that graph and nowhere else
-- see [agent-protocol.md](agent-protocol.md).

### One base, any number of overlays

The **base** is this checkout: jj's `default` workspace, `~/workspace`, sitting on `main`
with an empty working copy. It is the user's, and it is where the machine-level things live
-- the bytes under `data/`, the CMS, the full set of dev servers, the tmux session that keeps
them up. Nothing about the base is special to jj; what makes it the base is that everything
else points at it.

An **overlay** is a sibling directory `~/workspace-{n}` holding jj workspace `workspace-{n}`,
where `n` is its **slot** -- a positive integer that decides its dev ports and nothing else.
`mise run workspace add {n}` creates one on `main`, writes the slot into a gitignored
`mise.local.toml` (the only file that makes a checkout an overlay), links the bytes under
`data/` back into the base, and installs dependencies. `mise run workspace forget {n}` removes
it, refusing while it holds work `main` does not cover. One agent works in one overlay.

The names are derived, not chosen: directory, jj workspace, and slot are one number, so
`workspace-2@` in a log, `~/workspace-2` on disk, and ports two strides above the base's are
visibly the same thing. They are named by seat rather than by feature because a workspace outlives the
feature it was opened for and gets the next one. None of it reaches git: a jj commit carries
no workspace, and the workspace name survives only in the local operation log, which no clone
receives.

An overlay has no `.git` directory -- jj keeps one repository under the base's `.jj` and the
overlay holds a pointer to it -- so `git` commands do not work there. Everything jj-shaped
does, including the hooks, and `mise run gc` runs from the base.

### An overlay runs what it changes; the rest is the base's

The base runs every dev server, on the pinned ports. An overlay starts only the app it is
actually changing, on its slot's ports, and reaches everything it did not start on the base.
Two agents both changing the site cost two site servers and one API; two agents changing
different apps cost one server each. The cost tracks the number of things being changed, not
the number of people changing them, which is what keeps this from becoming a full stack per
seat.

Which is which is decided by looking, not by declaring: the site's Vite config probes its
slot's API and CDN ports once at startup and takes whichever answers, base otherwise. So the
rule for an agent is one line -- **start the app you are changing before the site**, or
restart the site after -- and there is no state to keep in step. The answer is baked into the
client bundle, which is why it is taken at startup and not per request. In the base, slot 0,
nothing is probed: the base ports are the base by definition.

`libs/urls` owns the arithmetic (`slotPort`, `developmentUrls`) and the injected override the
site's dev server defines; the wrangler workers take their slot ports as flags from the mise
tasks, computed from the same table. Bare `node`, vitest, and the workers themselves see no
override and get the base addresses, so the Rust mirror of the URL map renders the same in
every workspace and `mise run verify` is stable across them.

The base API accepts any loopback origin while it is itself answering on a loopback host,
because an overlay's site calls the base API from a port the allowlist cannot name and the
base cannot know how many slots exist. Production never answers on a loopback host, so the
clause is dead there.

### What is not shared

`node_modules`, `target/`, `.wrangler/` and `.svelte-kit/` are per checkout, and so is
`.cms/` -- which an overlay never has, because it never runs the tasks that write it. Rust
compiles once per overlay; the dev profile keeps that to the crates here, with dependencies
cached at `opt-level = 3`. Sharing `target/` across workspaces was considered and left: cargo
locks it, but two agents building different revisions would thrash one cache, and the cost it
saves is paid once per overlay rather than per edit.

`data/` is the exception in the other direction -- shared by construction, read-only from an
overlay. [data.md](architecture/data.md) has that rule.

### The base session

The base needs nobody in it. When there is an agent there, its work is housekeeping: `mise
run workspace refresh` after a push so the base's working copy follows `main` and the dev
servers reload against it, `mise run verify` on the merged result, the periodic selfcheck of
`spec/`, `mise run gc`. It does not commit for others and it does not rebase their work --
[commits.md](commits.md) says why -- so it is optional, and the user's own terminal is a fine
substitute. It also owns the servers below, and it is the one place a task that writes
`data/` or reaches R2 is run from.

**The base's dev servers live in one tmux session, `workspace-dev`, one window per server,
named for it: `site`, `api`, `cdn`, plus `cms` when the desktop CMS is wanted.** `mise run
base up` makes that true idempotently -- it creates what is missing, restarts a window whose
server has exited, and leaves a running one alone; `base status` says what each window is
doing and `base down` kills the lot. "Start the base's servers" means `mise run base up`,
whoever is asked. The window is a shell with the mise task typed into it rather than the task
as the window's command, so a server that dies leaves its last output on screen instead of
closing the window that would have shown it.

tmux is a Homebrew install, not a mise tool, and this is the exception to "mise owns every
tool" that [the hook scripts](#the-other-exception-hook-scripts) already are: a tmux server
outlives any directory and is shared with sessions started elsewhere, and a client whose
version is pinned per directory would meet a server started under another and refuse it. The
tool that has to agree with itself across the whole machine is installed once for the whole
machine.

The always-on set is the three servers, not the CMS. Those are called by something -- a
browser, an overlay's site -- and are useful without anyone looking at them; the CMS is a
window a person uses, so it comes up when asked (`mise run base up cms`) and not before.

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

### `latest` is the pin, until something breaks

Most tools here are declared `latest` on purpose. A version number is a claim that _this_
release is the one this repository works with, and for a linter or a CLI that claim is almost
never true -- it is simply the release that happened to be current on the day somebody typed it.
What the number then buys is a tool that stops improving until a person remembers to raise it,
and a diff every few months that says nothing except that time passed.

**A pin is a bug report, not hygiene.** It is written when a specific release actually breaks
this repository, and it carries the reason beside it: which version, what it broke, and what
would let the pin be lifted. A pin with no such note is indistinguishable from one nobody has
revisited, so the next reader cannot tell whether it is still needed.

`rust = "stable"` is the same rule wearing a channel name rather than `latest`.

The exposure this accepts is real and is the point: a tool can change under a build nobody
touched. `mise run verify` is what makes that survivable -- the change surfaces as a failing
check on the next run rather than as behaviour nobody notices, which is the trade a pin makes in
the opposite direction and worse, by deferring the same discovery indefinitely.

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

`hooks/` is deliberately outside mise's reach. The real policies live there once and use
`#!/usr/bin/env python3` with nothing beyond the standard library. Both vendor directories
bind the same `hooks/run.py` entrypoint; `.claude/settings.json` and `.codex/hooks.json` are
only the glue that exposes it to each harness.

The reason is the failure mode. mise activation is shell-scoped, and a hook is launched by
the agent harness rather than by an interactive shell. If a hook's interpreter came from
mise and mise were not active, the hook would fail to start -- and a hook that fails to
start enforces nothing while looking exactly like a hook that passed. Silent
non-enforcement is worse than no enforcement, because it is believed.

Compatibility is a check, not a claim made once. `mise run verify` invokes the exact shared
entrypoint with representative Claude Code and Codex payloads under `/usr/bin/python3`, the
interpreter both adapters run. This catches syntax and annotation features newer than the
macOS system Python before either harness has to discover them one failed hook at a time.
Version pinning would buy nothing here and would cost the guarantee that the hook starts
outside an activated shell.

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

### A Rust CLI parses with clap, and wears cargo's colours

Arguments are declared as types with clap's derive API rather than read out of `std::env::args`
by hand. The property being bought is refusal: a hand-rolled loop matches the flags it knows and
ignores the rest, so a typo is silence. `cms` had five commands where that silence spent money --
`--limit` was read with `.parse().ok()`, and an unparsed limit is no limit, so `--limit 2x` and
`--lmit 2` both bought the whole library. Derive fixes both at once because the flag set _is_ the
type; there is no second list of known names to keep in step.

This was argued against on dependency grounds and the argument was wrong: `cms` runs locally, and
[code.md](code.md) already says dependency count is not a constraint there. A local binary's
compile time is paid by the one machine that builds it and its size is paid by nobody, so
refusing a parser to save megabytes trades a correctness property for a saving no one
experiences. The rule was already written; it just was not read.

Help is clap's own, styled with `anstyle` to cargo's palette -- green headings, cyan literals.
Not clap's defaults: this stands next to `cargo` in the same terminal, and one constant saves its
reader a second colour language. The hand-written usage text it replaced had drifted anyway, with
one command's description printed under another's name.
