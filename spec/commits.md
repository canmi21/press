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
Co-Authored-By: Grok 4.6 <noreply@x.ai>
```

Claude models use the first address, GPT models the second, Grok the third. Fill in the actual
model short name rather than copying the example -- the point of the trailer is to record which
model, so a stale version defeats it. A new vendor needs a decision here first, which is one line
in this list.

**The address identifies a vendor, not a mailbox.** None of the three reaches anybody, and that is
not a defect. A trailer here answers "which model wrote this" so a later reader knows how much
scrutiny the change had; it was never a way to contact the author. An argument that a model
without a reachable address should be credited in prose instead was made and is wrong -- it would
apply equally to the two that came first.

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

### When `main` moved meanwhile

Other workspaces commit onto the same `main` -- see [toolchain.md](toolchain.md), "Parallel
workspaces". If it advanced while the task was underway, `jj bookmark move` refuses to move it
sideways, and that refusal is the synchronisation point: `jj rebase -d main`, run `mise run
verify` again on the rebased result, then move. The order matters. Each workspace verified
its own change in isolation; the rebased commit is the first place the two changes meet, and
it is what `main` is about to claim.

A conflict does not stop the rebase -- jj records it in the commit -- and it is resolved by
the agent whose change it is, in that workspace, because that is where the reasoning behind
the change still lives. Nobody resolves it on somebody else's behalf.

**Only rewrite commits that belong to your own workspace.** Rebasing, squashing or describing
another workspace's commits rewrites the parents its working copy sits on and leaves it stale
until that agent runs `jj workspace update-stale`, in the middle of whatever it was doing.
The one bookmark everyone moves is `main`; the commits under it are each written by exactly
one workspace, and stay that way until they are on `main`. A base session in particular does
not land other workspaces' commits for them: the trailer below records who shaped a change,
and a change landed by a hand that did not write it makes that record wrong.

The rebase is also where rule changes arrive: `spec/` moves with `main` like everything else,
and [agent-protocol.md](agent-protocol.md) says what to do with the delta.

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
thin adapters with the same event table, each calling the one `hooks/run.py` entrypoint. That
entrypoint owns routing, command selection, and combining output from policies that can both
apply to one event. A vendor adapter never owns policy: if the harness payloads diverge later,
the shared entrypoint normalises their common meaning rather than growing two implementations
that only appear equivalent.

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

`hooks/bookmark.py` runs at the end of a turn and refuses to hand the result back while `main`
does not cover what was committed. It is the only hook here bound to something other than a tool
call, and it has to be: the other two watch the Bash tool, and the moment the completion rule
above talks about is the moment the agent _stops_ calling tools. A turn that ends on its last
`jj describe` runs nothing afterwards for a tool hook to inspect, so the bookmark half of the
rule held only as long as it was remembered -- and it was not. Seventeen commits went by in one
session with the bookmark never moving.

**It refuses rather than moving the bookmark itself**, and that is the whole point of it. jj
ships the move as `experimental-advance-branches`, which [toolchain.md](toolchain.md) leaves off
so that a bookmark's position stays something somebody claimed. A hook that advanced `main`
would be that setting again under another name, and would take the claim away while looking
like enforcement.

It counts only commits that are described and non-empty. Undescribed working-copy changes are
the shape partial work is supposed to have -- the rule above says it stays uncommitted -- so
firing on those would turn a rule that protects unfinished work into one that demands it be
published.

`hooks/spec_diff.py` runs after a `jj rebase` and hands back the diff of `spec/` and
`CLAUDE.md` between the working copy before the rebase and after it. It reads the previous
state out of jj's operation log rather than remembering anything, and prints nothing when no
rule moved. It exists because a rule written in one workspace reaches another only through
`main`, and the moment it does is otherwise invisible: the files change under an agent whose
context still holds the old text. It does not block; whether a changed rule touches the task
is the agent's call, and it is asked at the one moment the answer is cheap.

What all of this does not cover, and why the rules above still have to be read rather than
merely enforced:

- Only Claude Code and Codex. Opencode and any other harness need their own adapter.
- Only the `-m` form. `jj commit` with no message opens an editor, which the hook cannot see.
- Not a human typing in a terminal.
- The bookmark check blocks once. A second stop is let through, because nothing can tell a
  refusal to move it from an inability to, and a gate with no exit is worse than a missed one.

Enforcement is a fast feedback loop, not a fence. The rules hold whether or not something is
watching.
