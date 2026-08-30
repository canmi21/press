# CLAUDE.md

Super monorepo. This is the entry point for any agent starting with zero context.
`AGENTS.md` is a symlink to this file.

## Start here

No context on this project? Read this file, then the `spec/` files covering your task, before
touching anything. If `spec/` does not answer a decision you are about to make, ask the user.
Decisions belong to the user, not to you -- unless their described approach already implies
the answer. See [spec/agent-protocol.md](spec/agent-protocol.md).

## How rules are recorded

Every rule the user states gets written down. Never leave one in chat only.

- This file holds the most important, most universal rules, plus the index below.
- Anything narrower gets its own topic file in `spec/`, split by aspect, linked from the index.
- Never restate spec content here. Link to it.
- When the user says "remember this" or "update yourself": do not write to agent memory. Audit
  `spec/` and this file instead, then verify a zero-memory agent could recover the project
  from `CLAUDE.md` alone. See [spec/agent-protocol.md](spec/agent-protocol.md).

## Core rules

**Commits** -- Conventional Commits. Omit the scope in most cases. Subject starts lowercase,
imperative mood, 96 characters max. See [spec/commits.md](spec/commits.md).

**Language** -- Talk to the user in simplified Chinese with English technical nouns left
untranslated. Everything written into a file is English only, no exceptions: code, comments,
docs, commit messages. Only an explicit user request overrides this. "App" means a standalone
application, a deployed service, or the desktop client depending on context -- read which from
the sentence rather than asking every time. See [spec/voice.md](spec/voice.md).

**Naming** -- Files and directories are lowercase English, hyphens allowed. A language with
its own convention wins locally: Rust source files use underscores. Identifiers inside code
always follow the language's own convention. Vendor names stay at the binding edge -- name a
module for what it does, not for who supplies it. See [spec/naming.md](spec/naming.md).

**Lint vs format** -- a linter owns semantics, a formatter owns layout. Whatever the formatter
rewrites, the linter must not report; disable the overlap on the linter side. Holds for every
language. See [spec/lint-format.md](spec/lint-format.md).

## Layout

```
spec/   rules      libs/   libraries, any language      apps/   deployables, any language
contents/  articles, tracked because prose wants diffs  repos/  separate repos, ignored here
data/   assets, plus the records about them: bytes stay out of git, records go in
```

A directory under `libs/` is a namespace, not a language choice -- one name, one thing, even
when a Rust core and a TypeScript wrapper live inside it. `pnpm-workspace.yaml` globs;
`Cargo.toml` lists members by hand because a glob there breaks every cargo command the moment
a TypeScript-only directory appears. Name for responsibility, never for deployment shape or
product. See [spec/architecture/workspace.md](spec/architecture/workspace.md).

## Toolchain

**The user's shell is fish.** Any command written for them to run must be fish syntax --
`set -gx X y`, not `export X=y`; `$(cmd)` is not fish. Commands an agent runs through its own
tool go through that tool's shell instead, which is usually not fish, so the two are written
differently on purpose.

Version control is jj (Jujutsu), colocated with git -- use `jj`, not `git`. Bookmarks do not
advance on their own. Pushing is the user's to run; do not offer it. mise owns every tool
version, linters and formatters included; see `mise.toml`. Default stacks are Rust + Cargo for
binaries and applications, TypeScript + pnpm + Svelte for web.
See [spec/toolchain.md](spec/toolchain.md).

Several agents may work at once, each in its own jj workspace, all committing onto one `main`:
`~/workspace` is the base, `~/workspace-{n}` are overlays. Agents coordinate through the change
graph only, never by messaging each other. Know which workspace you are in before acting.
An agent never stops a dev server, its own included, and starts one it finds missing; a process
is only ever killed by pid, never by name.
See [spec/toolchain.md](spec/toolchain.md) and [spec/agent-protocol.md](spec/agent-protocol.md).

Indentation is tabs at width 2 in every language, YAML excepted. `.editorconfig` is the source
of truth.

## Index

Grouped by what you are about to do. A topic that grew past one file became a directory, and
each of its files is listed here rather than behind a second index.

**Starting, and working with the user**

| Topic                                     | File                                             |
| ----------------------------------------- | ------------------------------------------------ |
| Cold start, decision authority, verifying | [spec/agent-protocol.md](spec/agent-protocol.md) |
| Voice and communication                   | [spec/voice.md](spec/voice.md)                   |
| Commit conventions and their enforcement  | [spec/commits.md](spec/commits.md)               |

**How the repository is shaped**

| Topic                                     | File                                                             |
| ----------------------------------------- | ---------------------------------------------------------------- |
| Layout, namespaces, extraction thresholds | [spec/architecture/workspace.md](spec/architecture/workspace.md) |
| What git keeps, what sync publishes       | [spec/architecture/data.md](spec/architecture/data.md)           |
| The two CMS shells and one application    | [spec/architecture/cms.md](spec/architecture/cms.md)             |
| Separate repositories nested under repos/ | [spec/architecture/repos.md](spec/architecture/repos.md)         |
| Naming conventions                        | [spec/naming.md](spec/naming.md)                                 |

**Assets, and how they reach a reader**

| Topic                                     | File                                                           |
| ----------------------------------------- | -------------------------------------------------------------- |
| Image variants, descriptions, tags, cards | [spec/architecture/media.md](spec/architecture/media.md)       |
| The font pipeline                         | [spec/architecture/fonts.md](spec/architecture/fonts.md)       |
| Formats and caching at the edge           | [spec/architecture/delivery.md](spec/architecture/delivery.md) |

**Writing code**

| Topic                          | File                                       |
| ------------------------------ | ------------------------------------------ |
| Type checking, tests, comments | [spec/code.md](spec/code.md)               |
| Linting and formatting         | [spec/lint-format.md](spec/lint-format.md) |
| Toolchain and default stacks   | [spec/toolchain.md](spec/toolchain.md)     |

**The site a reader sees**

| Topic                                     | File                                     |
| ----------------------------------------- | ---------------------------------------- |
| Interface behaviour, focus rings, styling | [spec/styling.md](spec/styling.md)       |
| Serving a reader their language           | [spec/locale.md](spec/locale.md)         |
| Telling a search engine what changed      | [spec/indexing.md](spec/indexing.md)     |
| Newsletter, likes, D1, and client cache   | [spec/engagement.md](spec/engagement.md) |
| Analytics clients, dev behaviour, ids     | [spec/analytics.md](spec/analytics.md)   |

**Work that takes minutes and spends money**

| Topic                                  | File                               |
| -------------------------------------- | ---------------------------------- |
| Long-running tasks and their execution | [spec/tasks.md](spec/tasks.md)     |
| Translating article content            | [spec/i18n.md](spec/i18n.md)       |
| Single-provider Twitter lookups        | [spec/twitter.md](spec/twitter.md) |
