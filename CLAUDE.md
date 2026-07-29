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
docs, commit messages. Only an explicit user request overrides this.
See [spec/voice.md](spec/voice.md).

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
data/   assets, in the tree but never in git            projs/  reserved, not created yet
```

A directory under `libs/` is a namespace, not a language choice -- one name, one thing, even
when a Rust core and a TypeScript wrapper live inside it. `pnpm-workspace.yaml` globs;
`Cargo.toml` lists members by hand because a glob there breaks every cargo command the moment
a TypeScript-only directory appears. Name for responsibility, never for deployment shape or
product. See [spec/architecture.md](spec/architecture.md).

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

Indentation is tabs at width 2 in every language, YAML excepted. `.editorconfig` is the source
of truth.

## Index

| Topic                                     | File                                             |
| ----------------------------------------- | ------------------------------------------------ |
| Cold start, decision authority, selfcheck | [spec/agent-protocol.md](spec/agent-protocol.md) |
| Layout, workspaces, data                  | [spec/architecture.md](spec/architecture.md)     |
| Voice and communication                   | [spec/voice.md](spec/voice.md)                   |
| Commit conventions                        | [spec/commits.md](spec/commits.md)               |
| Naming conventions                        | [spec/naming.md](spec/naming.md)                 |
| Linting and formatting                    | [spec/lint-format.md](spec/lint-format.md)       |
| Toolchain and default stacks              | [spec/toolchain.md](spec/toolchain.md)           |
