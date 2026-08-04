# Code conventions

## Type checking is not optional

`tsconfig.json` runs `strict` plus `noUncheckedIndexedAccess`. Both are deliberate, and the
second one is the expensive one, so it needs its reason on the record.

`noUncheckedIndexedAccess` makes every array index and every destructured element
`T | undefined`. That is annoying exactly where code is doing something unproven, which is the
point. When it was first switched on here it immediately found three places where a length
check and a destructure sat next to each other with nothing connecting them, plus a narrowing
bug where `Array.isArray` left a `readonly` array unnarrowed in the false branch -- all in code
that passed its whole test suite.

The rule it implies: **guard against the values the code uses, not against a count.**
`if (parts.length >= 5)` proves nothing to a reader or a compiler about `parts[3]`. Destructure
first, then guard on the names. Never reach for `!` or a cast to silence this class of error --
the error is describing a real gap, and silencing it keeps the gap while removing the warning.

## Dependency budgets differ by destination

Where the code runs decides how much a dependency is allowed to cost.

|                                        | Optimise for            | Budget                                               |
| -------------------------------------- | ----------------------- | ---------------------------------------------------- |
| Runs locally (CLI, CMS, build tooling) | correctness, then speed | dependency count and binary size are not constraints |
| Ships to the edge or a browser         | payload                 | every dependency is argued for                       |

A local binary is never downloaded by anyone. Its compile time is paid once per change by the
one machine that builds it, and its size is paid never. Picking a weaker library there to save
megabytes trades a real correctness risk for a saving nobody experiences.

Deployed code is the opposite, and the same problem can deserve opposite answers in the two
places. The favicon resolver is the worked example: the Worker version parsed HTML with
regexes because a Worker has a bundle budget, and the local port uses a real HTML5 tokenizer
because it does not. Measured on adversarial input, the regex approach silently picked up a
commented-out `<link>`, a `<link>` inside a `<script>` string, and left `&amp;` undecoded --
three wrong icons, none of which announce themselves.

So when a dependency looks heavy, the question is not "is this too big" but **"who pays for
this size, and what does refusing it cost instead?"**

## Tests

Colocated with source as `src/*.test.ts`, run by vitest from the repo root.

Test the decisions, not the syntax. What earns a test here: the branch that used to be wrong,
the input shape that comes from outside, the invariant that a future refactor could quietly
break. A test that restates the implementation line by line only makes the implementation
harder to change.

When a bug is found, the fix and a test that would have caught it land together, with the
cause noted at the assertion. A regression test whose reason is not written down gets deleted
by whoever next finds it confusing.

## Nothing is committed unverified

`mise run verify` -- types, lint, tests -- passes before a commit is made. The commit hook
formats automatically, so formatting is never the thing that fails; what is left are the three
checks that a human or an agent can actually get wrong.

This exists because both of the other guarantees are weaker than they look. Tests only cover
what someone thought to test, and the type checker only sees what it is pointed at. Running
them is the cheap part; the expensive part is discovering months later which commit broke
something that nothing was watching.

## Comments

A comment explains a **why** that the code cannot state: the alternative that was rejected, the
constraint from outside, the trap that looks like a bug but is not. Never restate what the
line does.

The specific comment worth writing more often than feels natural: the one next to a value that
looks arbitrary. A config number, a strictness flag, a rule turned off. Those are the ones a
future reader will "clean up" unless the reason is sitting right there. Larger decisions go in
`spec/` instead -- see [agent-protocol.md](agent-protocol.md) for which is which.

### Few, short, and only where the reason is not recoverable

Comments are rationed. Write one where a reader would otherwise get it wrong, and nowhere else.
Ordinary code carrying an ordinary intent gets none -- explaining it adds a second thing to keep
true without making the first any clearer.

Three prose paragraphs above a token, a docstring on every field of a table, a note restating
the branch below it: each is a cost paid on every future read, and paid again by whoever has to
keep it accurate. A file where everything is annotated is one where nothing stands out, which is
the same as having annotated nothing.

Write for someone scanning, not reading. Lead with the point; if the first line does not carry
it, cut down to the line that does. The full argument belongs in `spec/`, with the comment
naming the file rather than repeating it.
