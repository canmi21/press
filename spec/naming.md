# Naming conventions

This document covers filesystem names only: files and directories. Identifiers inside source
code are governed by the language, not by this document -- see [Inside code](#inside-code).

## Default

Lowercase English, hyphen-separated:

```
user-profile.ts
api-client/
2026-07-release-notes.md
```

No uppercase, no spaces, no underscores by default.

## Language exceptions

When a language has an established convention of its own, that convention wins for source
files in that language. The default above applies to everything else in the repo.

| Language | Source file naming | Example |
| --- | --- | --- |
| Rust | Underscores, matching the module name | `user_profile.rs` |
| Python | Underscores, matching the module name | `user_profile.py` |
| Go | Underscores where a separator is needed | `user_profile.go` |

Rust file naming is independent of crate naming. A crate is still `my-crate` with a hyphen in
`Cargo.toml`; only its source files use underscores, because a module name is a Rust
identifier and cannot contain a hyphen.

## Framework exceptions

Some frameworks assign meaning to a filename. Follow the framework -- a renamed file simply
stops working. Examples: `Cargo.toml`, `Dockerfile`, `README.md`, `CLAUDE.md`, Next.js
`[slug]/page.tsx`.

## Inside code

Identifiers follow the language's own convention with no interference from this document:

- TypeScript / JavaScript: `camelCase` variables and functions, `PascalCase` types.
- Rust: `snake_case` items, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants.
- Python: `snake_case` items, `PascalCase` classes.

A `camelCase` variable inside a `user-profile.ts` file is correct. The two rules do not
interact.
