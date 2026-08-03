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

## Co-authorship

An assistant that **participated in the design** gets a trailer. One that merely typed out an
instruction does not.

| The user's request                                            | Trailer |
| ------------------------------------------------------------- | ------- |
| "update the dependency", "rename this file", "change X to Y"  | no      |
| "how should we do X", "what's wrong here", a problem to solve | yes     |

The line to draw is authorship, not effort. When the user decided what to do and the assistant
executed it, the assistant wrote no part of the decision and claiming co-authorship
misattributes it. When the user brought a question and the assistant shaped the answer, leaving
the trailer off hides who did that shaping -- which matters later, when someone wants to know
how much scrutiny a change received.

```
Co-Authored-By: Claude Opus-5 <noreply@anthropic.com>
Co-Authored-By: Codex GPT-5 <codex@openai.com>
```

Claude models use the first address, GPT models the second. Fill in the actual model short name
rather than copying the example -- the point of the trailer is to record which model, so a stale
version defeats it. Only these two addresses are used; a new vendor needs a decision here
first.

Applies going forward only. Earlier commits are not rewritten to add trailers.

## Tooling

Version control is jj, colocated with git:

```bash
jj describe -m "feat: add token refresh"   # set the message on the working copy
jj commit -m "feat: add token refresh"     # describe, then start a new change
```

In jj a message is not frozen at commit time -- `jj describe` rewrites it at any point, so
fixing a malformed message never requires an amend dance.

## Completion

When an agent finishes a task that changed repository files, it commits the completed change
and moves the `main` bookmark before handing the result back. A separate request to commit is
not required. Partial or blocked work stays uncommitted, and unrelated changes already in the
working copy are never swept into the task's commit merely to make the tree clean.

## Enforcement

`hooks/commit.py` runs before any jj subcommand that can write a description and does two
things: it runs `jj fix` so the content being committed is already formatted, then it checks
the message. A malformed message is refused with the specific reason, which the agent reads
and corrects on the spot.

The guarded list is derived from which subcommands accept `-m`, not from the two that write
most of the messages. It was `commit` and `describe` alone, and `jj split -m` carried a
malformed co-author trailer straight past it -- caught only because a later `describe` in the
same session was checked and rejected the identical text. A gap of that kind is silent by
construction: the hook cannot report a command it never looked at. So when jj grows another
way to set a message, the list grows with it -- currently `commit`, `ci`, `describe`, `desc`,
`split`, `new`, `squash`, `metaedit`.

The hook lives at the agent-harness layer rather than in jj, because jj offers no hook point
at all: it has no commit hook, and it explicitly refuses aliases that shadow built-in
commands (verified -- `jj` prints `Cannot define an alias that overrides the built-in
command 'commit'`). The moment a message is written is inside the agent's tool call, so that
is where the check has to sit.

The behavior has one home under `hooks/`. `.claude/settings.json` and `.codex/hooks.json` are
thin adapters that bind the same scripts to each harness's `PreToolUse` and `PostToolUse`
events. Command selection lives inside the scripts rather than in either adapter because the
two hook configs do not share a command-predicate field; putting it in one vendor's config
would make the other runner enforce a wider rule.

`hooks/spec_check.py` runs _after_ the commit lands and asks the question the diff
cannot: was a decision made here that nobody wrote down? It fires when a `feat`, `refactor`,
`build`, or `perf` commit touched no rules, and injects a reminder to record the reasoning
while it is still in context.

It deliberately does not block. Many commits of those types settle nothing, and a gate that
fires on false positives gets routed around instead of read. Judging whether a decision was
actually made is the agent's job; the hook only guarantees the question gets asked at the one
moment when the answer is still cheap to write -- before the conversation that produced it is
gone. Bug fixes are excluded on purpose: a fix records its cause at the regression test, not
in the rules.

What all of this does not cover, and why the rules above still have to be read rather than
merely enforced:

- Only Claude Code and Codex. Opencode and any other harness need their own adapter.
- Only the `-m` form. `jj commit` with no message opens an editor, which the hook cannot see.
- Not a human typing in a terminal.

Enforcement is a fast feedback loop, not a fence. The rules hold whether or not something is
watching.
