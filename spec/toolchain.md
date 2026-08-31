# Toolchain, as this repository uses it

The rules that follow the author rather than the project -- shell, secrets, version
control, tool versions, dependency policy, default stacks -- are the meta repository's.
What is here is what this repository alone decides.

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

One checkout runs one set, on the pinned numbers. The slot arithmetic that shifted every port
for a second checkout of this repository is gone with the arrangement it served; what it
protected still holds, and more simply: the CMS is the one process that writes `data/`, and a
second copy of it collides on `CMS_PORT`, which is the mutex doing its job.

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

### Two copies of one package read as a type comparing badly to itself

A workspace member with its own range for something the root also carries can be left on an older
resolution after an update, even when its range admits the new one. Nothing warns; the graph
simply holds both.

**What it looks like is not what it is.** The compiler reports `Excessive stack depth comparing
types 'Plugin<any>' and 'Plugin<any>'` -- a type against what appears to be itself -- plus
`no overload matches this call` on the same object. The two are structurally identical and come
from different copies, so nothing about the message says "duplicate". `pnpm why <package>` is what
names it, and `pnpm dedupe` collapses it when no manifest range is actually in conflict.

Worth recording because the update that produced it changed a compiler major at the same time, and
the first suspicion was the compiler. It was not; both copies had been sitting there since the
member last resolved.

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

