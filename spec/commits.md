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

## Enforcement

`.claude/hooks/jj-commit.sh` runs before any `jj commit` or `jj describe` and does two
things: it runs `jj fix` so the content being committed is already formatted, then it checks
the message. A malformed message is refused with the specific reason, which the agent reads
and corrects on the spot.

The hook lives at the agent-harness layer rather than in jj, because jj offers no hook point
at all: it has no commit hook, and it explicitly refuses aliases that shadow built-in
commands (verified -- `jj` prints `Cannot define an alias that overrides the built-in
command 'commit'`). The moment a message is written is inside the agent's tool call, so that
is where the check has to sit.

What this does not cover, and why the rules above still have to be read rather than merely
enforced:

- Only Claude Code. Codex, opencode, and any other harness need their own equivalent.
- Only the `-m` form. `jj commit` with no message opens an editor, which the hook cannot see.
- Not a human typing in a terminal.

Enforcement is a fast feedback loop, not a fence. The rules hold whether or not something is
watching.
