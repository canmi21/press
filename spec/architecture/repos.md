# Sub-repositories under `repos/`

## What `repos/` is

Each directory under `repos/` is **its own repository**, cloned into place. The workspace
ignores them: git and jj never carry them, and nothing here is a submodule.

They sit inside the workspace on the filesystem for one reason -- so that an agent working in
this directory can reach both halves at once. A sub-repo's application reads `libs/` at a real
path, an agent fixing a bug there can write the fix into `libs/` in the same session, and only
the half that nothing else could use stays behind in the sub-repo. Moving the sub-repo out to
a sibling directory would cost exactly that: two checkouts an agent has to be told about
separately, and a library change that has to be described rather than made.

## What earns a directory here

A sub-repo is for an application published on its own -- its own issue tracker, its own
release, its own readers. Everything else stays in `apps/`.

The reason is never size, and never "it feels separate". It is that **somebody outside this
workspace needs to look at it, file against it, or build it**. An application nobody outside
will touch gains nothing from a second repository and pays for it in every rule below.

## The boundary that decides where a line of code lives

Inside a sub-repo, two kinds of code sit next to each other and they belong in different
repositories:

- **Vendor-specific and business-specific logic** -- what this one application does, the
  service it talks to, the shape its own data happens to have. Stays in the sub-repo.
- **Generic, reusable logic** -- anything a second application here could want. Goes into
  `libs/`, in the same session, as a normal workspace change.

This is [workspace.md](workspace.md)'s extraction threshold applied across a repository
boundary, and the threshold does not move because the boundary is there: code goes into `libs/`
when it acquires a second consumer, not because a sub-repo would look tidier without it.

**The user decides which side a given piece falls on.** The distinction is about how much of
the logic is bound to one vendor or one product, and that is a judgement about intent, not one
an agent can read off the code. Ask when it is not obvious; a wrong guess here is expensive in
both directions -- a generic helper trapped in a sub-repo is invisible to everything else, and
a vendor-shaped one lifted into `libs/` makes the library lie about what it is for.

## Why not a submodule

jj has no submodule support, and the failure is quiet rather than loud. Colocating over a
repository that has one prints:

```
ignoring git submodule at "libs/inner"
```

jj then preserves the existing gitlink in every commit it writes but never updates it and never
checks the content out. So the pointer can only be moved by a `git add` plus `git commit` in a
colocated repo -- mixing hand-run git commits into jj's history, which is the one thing this
setup is arranged to avoid.

A nested repository needs no mechanism at all: jj stops at any directory holding its own `.jj`
or `.git`, does not snapshot it, and resolves a `jj` command run inside it against the inner
repository. That behaviour is what `repos/` uses, and it costs nothing to get.

**`.gitignore` still has to list `repos/*/`.** Not for jj's sake -- for git's. A plain
`git add -A` in this colocated repo happily writes the very gitlink the paragraph above rejects:

```
warning: adding embedded git repository: repos/foo
160000 31cfb453... 0	repos/foo
```

The ignore entry is what stops a git-side command from reintroducing a submodule by accident.

## How a sub-repo reaches the workspace's libraries

A sub-repo declares a git dependency pointing into this repository, which is public and
therefore already a resolvable dependency source:

```json
"@canmi/urls": "git+https://github.com/canmi21/workspace#path:/libs/urls"
```

No publishing, no version numbers, and no build step added to the library -- the `exports`
still point at `./src/*.ts` and pnpm delivers the source as written.

**The specifier floats, the lockfile pins.** `package.json` names a ref and never a revision;
`pnpm-lock.yaml` records the commit that ref resolved to. `pnpm install` holds that commit,
`pnpm update` re-resolves it to the current tip. It is the same model as a semver range and a
resolved version, with one difference worth stating: a git ref's "release" is the moment
something is pushed, so pushing this repository is what publishes a library change.

**Inside the workspace the same declaration means the local source.** `pnpm-workspace.yaml`
lists `repos/*` and carries an override per library a sub-repo consumes:

```yaml
overrides:
  '@canmi/urls': 'workspace:*'
```

The override is committed here in the open. A contributor never clones this repository, so it
reaches nothing they build. What it buys is that an uncommitted edit to `libs/urls` is live in
the sub-repo immediately, which is the whole reason the sub-repo sits inside the workspace.

**The two lockfiles do not collide, because they are in two repositories.** A root install
resolves through this repository's lockfile and leaves `repos/*/pnpm-lock.yaml` untouched; a
contributor's install uses the sub-repo's own. Nothing has to arbitrate between them.

**`apps/` is deliberately the opposite.** A member of `apps/` has no lockfile of its own and
shares the workspace's, because nobody outside ever resolves it. The difference between the two
directories is not style: it is whether an install can happen somewhere this repository is not.

## A library consumed from outside may not use `workspace:*`

The `workspace:` protocol is a coordinate system that only exists inside this repository. A
library that uses it cannot be resolved from anywhere else:

```
[ERR_PNPM_WORKSPACE_PKG_NOT_FOUND] In : "@canmi/urls@workspace:*" is in the dependencies
but no package named "@canmi/urls" is present in the workspace
This error happened while installing the dependencies of @canmi/robots@0.0.0
```

Today `@canmi/imgsrc` and `@canmi/robots` each carry one such dependency and are therefore
workspace-only. That is not a defect while nothing outside wants them. It becomes one silently:
the set of libraries a sub-repo can use shrinks every time one library starts depending on
another, and nothing announces it at the moment the dependency is added.

So a library that a sub-repo consumes states its dependencies in a form that resolves anywhere.
`repos check` fails when a sub-repo names a library that cannot be resolved from outside, which
is the only reliable moment to notice.

## Raw TypeScript stops at `node_modules`

`libs/` exports source rather than a build, and [workspace.md](workspace.md) already qualifies
that: it holds only while every consumer bundles. The limit is sharper than that sentence
suggests. Node refuses to strip types from anything under `node_modules`, whatever the flags:

```
Error [ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING]: Stripping types is currently
unsupported for files under node_modules
```

Inside the workspace this never comes up, because every consumer is Vite, workerd or esbuild. A
sub-repo hits it the first time it wants a plain node script that imports a library. When that
day comes, add a build to that one library, as workspace.md says -- do not work around it in the
sub-repo, because the workaround would be invisible from the library it is compensating for.

## The pin drifts silently, so a check makes it loud

Developing against local source while contributors build against a pinned commit is the
arrangement's one real cost. The two disagree in three ways, and only the first is loud on its
own:

1. The pinned revision was never pushed. A contributor's install cannot fetch it, and their CI
   fails -- loudly, but only where you are not looking.
2. `libs/` moved since the pin. They resolve an older library: it may compile and behave
   differently, which is worse than failing.
3. `libs/` has uncommitted changes. Nothing outside this checkout can build against them yet,
   and you will not notice, because your own build reads the working copy.

`mise run repos check` reports all three and runs inside `mise run verify`. It reads whatever
sub-repos are present and is a no-op when none are, so a checkout without them -- an overlay,
a fresh clone -- pays nothing.

The third state is the one worth having. The first two can only be found after committing; the
third warns while the change is still in the working copy, which is the cheapest moment there is.

This is the shape [`rust.test.ts`](../../libs/urls/src/rust.test.ts) already uses for the
generated Rust URL mirror: a snapshot taken across a boundary, plus a check that fails the
moment it stops agreeing with its source. There the boundary is a language; here it is a
repository.

## Restoring the local link after a bump

`pnpm update --ignore-workspace` resolves the way a contributor would, which is what updates the
outward lockfile -- and it replaces the sub-repo's `node_modules` with the fetched copies. A
following root `pnpm install` does **not** put the local links back: it reports `Already up to
date`, and `--force` does not change that. The sub-repo's `node_modules` has to be removed first.

Left to a person to remember, this fails as a silent fall out of local development: library edits
stop taking effect and nothing says why. `mise run repos bump <name>` is the three steps as one
command, and exists for that reason rather than for convenience.

## No manifest, until forgetting one is likely

Nothing records the list of sub-repos and their remotes. A fresh clone of this workspace has an
empty `repos/`, and the clones are re-made by hand.

That is the [grouping threshold](workspace.md) argument again: let the growth force the
structure. A manifest is a file to keep in sync with reality, and while the number of sub-repos
is small enough to remember, it would only ever be the second place the truth is written. Add
one when a sub-repo has actually been forgotten, not in anticipation.
