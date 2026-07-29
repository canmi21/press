# Commit conventions

## Format

[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <subject>

[optional body]
```

## Scope

Omit the scope in most cases. Add one only when it carries information the subject line
cannot: a large monorepo package boundary that is not already obvious from the change.
`feat: add token refresh` is preferred over `feat(auth): add token refresh` when the diff
already makes the area clear.

## Types

| Type       | Use for                                   |
| ---------- | ----------------------------------------- |
| `feat`     | New user-facing capability                |
| `fix`      | Bug fix                                   |
| `docs`     | Documentation only                        |
| `refactor` | Behavior-preserving restructure           |
| `perf`     | Performance work                          |
| `test`     | Tests only                                |
| `build`    | Build system, dependencies, toolchain     |
| `ci`       | CI configuration                          |
| `chore`    | Everything else, including repo bootstrap |
| `revert`   | Reverting a previous commit               |

## Subject line

- Starts lowercase.
- Imperative mood: `add`, not `added` or `adds`.
- No trailing period.
- 96 characters max, including the `type: ` prefix.

## Body

Optional. Wrap at 96 characters. Explain why, not what -- the diff already shows what.

## Language

English only, no exceptions. See [voice.md](voice.md).

## Tooling

Version control is jj, colocated with git:

```bash
jj describe -m "feat: add token refresh"   # set the message on the working copy
jj commit -m "feat: add token refresh"     # describe, then start a new change
```

In jj a message is not frozen at commit time -- `jj describe` rewrites it at any point, so
fixing a malformed message never requires an amend dance.
