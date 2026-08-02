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

| Language | Source file naming                      | Example           |
| -------- | --------------------------------------- | ----------------- |
| Rust     | Underscores, matching the module name   | `user_profile.rs` |
| Python   | Underscores, matching the module name   | `user_profile.py` |
| Go       | Underscores where a separator is needed | `user_profile.go` |

Rust file naming is independent of crate naming. A crate is still `my-crate` with a hyphen in
`Cargo.toml`; only its source files use underscores, because a module name is a Rust
identifier and cannot contain a hyphen.

## Framework exceptions

Some frameworks assign meaning to a filename. Follow the framework -- a renamed file simply
stops working. Examples: `Cargo.toml`, `Dockerfile`, `README.md`, `CLAUDE.md`, Next.js
`[slug]/page.tsx`.

## Asset directories are singular; infrastructure is plural

Under `data/`, a directory holding addressable resources is named in the singular:
`image/`, `opengraph/`, `favicon/`, `meta/`, and `video/` or `audio/` when they arrive. Each
entry is a thing a reader asks for by name, and the directory reads as the type of that one
thing -- `image/{cid}.avif` is _an_ image.

`fonts/` is plural, and the exception is the rule working rather than breaking. Fonts are not
resources anyone requests individually; they are one self-hosted set the site loads as a
whole, closer to a dependency than to content. The plural marks that difference at a glance,
so a directory name says which kind of thing is inside before anything is opened.

## Vendor names stay at the edge

Name things for what they do, not for who supplies them. A vendor name belongs only in the
thin layer that binds to that vendor. Everywhere else -- the module names, the directories,
the types, the functions -- use the objective description of the technology or the function
being performed.

`apps/cdn` is a CDN whether Cloudflare, Fastly, or a box in a closet serves it. Naming it
`apps/r2` or `apps/cloudflare` would describe the current bill, not the job.

**Why.** A vendor name scattered through a codebase is a bet that the vendor is permanent,
and that bet has no upside. When it loses you get the worst of both: either a rename that
touches every layer, or a name that now lies -- an `r2.ts` that talks to S3. And even while
the bet is still good, the name carries less information than the alternative, because at
the point of use you want to know what a module _does_, not who invoices for it.

**Where a vendor name is correct.** Three places, all of them the boundary:

- Files the vendor defines. `wrangler.jsonc` is wrangler's file; renaming it breaks it.
- Dependency identifiers. `@cloudflare/workers-types` names a real package.
- The single adapter module that speaks the vendor's protocol and exposes your interface.

**The test.** If the vendor were replaced tomorrow, how many names would have to change?
One. If the answer is more, the vendor has leaked past the boundary.

This rule and the "name for responsibility" rule in
[architecture.md](architecture.md) are the same instinct at two scales: the volatile fact --
product, domain, deployment shape, supplier -- never gets carved into the part that is
expensive to change.

**The inverse also holds: a vendor's thing must not squat on a generic name.** `libs/svg-canvas`
styles SVG diagrams written in Claude's authoring convention. It was briefly `libs/canvas`,
which was wrong in the opposite direction from the usual mistake -- a vendor-specific
boundary had taken the name a real, general canvas library will want. A precise convention
name such as `svg-canvas` is fine; the generic `canvas` name stays free for the thing that
earns it.

Inside that library the convention's own selectors (`.svg-canvas`, `.c-purple`) are left
untouched, because they are the contract with the markup. The library name was the only part
we owned.

Worked example from this repo: the commit hook is `hooks/commit.py`, not `jj-commit.py` and not
a copy under either agent's vendor directory. Validating a commit message is not a jj-specific
job; it would survive a move to another VCS untouched. The `jj-` prefix named the current
supplier of the commits, which is exactly the information the filename did not need.

## Inside code

Identifiers follow the language's own convention with no interference from this document:

- TypeScript / JavaScript: `camelCase` variables and functions, `PascalCase` types.
- Rust: `snake_case` items, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants.
- Python: `snake_case` items, `PascalCase` classes.

A `camelCase` variable inside a `user-profile.ts` file is correct. The two rules do not
interact.
