# CLAUDE.md

Super monorepo. This is the entry point for any agent starting with zero context.
`AGENTS.md` is a symlink to this file.

## How rules are recorded

Every rule the user states gets written down. Never leave one in chat only.

- This file holds the most important, most universal rules, plus the index below.
- Anything narrower gets its own topic file in `spec/`, split by aspect, linked from the index.
- Never restate spec content here. Link to it.
- When the user states a new rule, record it before continuing the task: extend the
  matching `spec/` file, or create a new one and add it to the index.

## Core rules

**Commits** — Conventional Commits. Omit the scope in most cases. Subject starts lowercase,
imperative mood, 96 characters max. See [spec/commits.md](spec/commits.md).

**Language** — Talk to the user in simplified Chinese with English technical nouns left
untranslated. Everything written into a file is English only, no exceptions: code, comments,
docs, commit messages. Only an explicit user request overrides this.
See [spec/voice.md](spec/voice.md).

**Naming** — Files and directories are lowercase English, hyphens allowed. A language with its
own convention wins locally: Rust source files use underscores. Identifiers inside code always
follow the language's own convention. See [spec/naming.md](spec/naming.md).

## Toolchain

- Version control is jj (Jujutsu), colocated with git. Use `jj`, not `git`, for everyday work.
- Tool versions are managed by mise; see `mise.toml`. Do not install toolchains outside it.

## Index

| Topic | File |
| --- | --- |
| Voice and communication | [spec/voice.md](spec/voice.md) |
| Commit conventions | [spec/commits.md](spec/commits.md) |
| Naming conventions | [spec/naming.md](spec/naming.md) |
