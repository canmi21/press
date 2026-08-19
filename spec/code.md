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

### Local code does not hand-roll a standard

The budget above is a permission. This is the obligation that goes with it: **where a format has
a de-facto standard and a known library implements it, local code uses the library**, even to
reach one small corner of what it does. Not a regex over the shape that usually appears, not a
`split_once` that covers the cases seen so far.

The cost of refusing is not a bug so much as a _disagreement_, and disagreements in a parser are
the quiet kind. `cms og` read article frontmatter with a hand-rolled split while the segment
layout read the same frontmatter with `serde_yaml_ng`, and the card looks its translation up by
the title string the first one produced. A folded scalar -- `title: >-` across two lines -- gives
the hand-rolled reader `>-`, so the card is titled with the fold marker and, worse, finds no
translation under it and falls back to the source language in every view, reporting nothing.
Quoted scalars with an escaped quote inside fail the same way. Nothing is wrong; something is
merely different, everywhere, silently.

**Two readings of one format is the shape to watch for.** Wherever a second reader appears for
something already parsed elsewhere, it is the same library or it is a defect waiting for the
first input that separates them.

This licence is bounded by destination exactly as the table above is. Nothing here permits a
library into a Worker or a browser bundle to save writing twenty lines; there, the payload is the
constraint and the trade runs the other way.

## Errors are types on the way up and one message at the edge

**A fallible operation names what can go wrong as its own type**, derived with `thiserror`. Not a
`String`, and not a panic. A caller that receives a type can match on the case; a caller that
receives a string can only print it, and a caller that receives a panic cannot do either.

The distinction that decides between an error and a panic is **whose mistake it is**. A person
hand-writing frontmatter will mistype it, so unreadable frontmatter is an ordinary event and
returns [`Malformed`](../apps/cms/src/i18n/segment.rs). A panic is for the state that cannot
arise unless this code is already wrong. `cms i18n` used to abort on a stray colon in an article,
with a message naming the fault and not the file, which left a binary search through the corpus
as the way to find out which article it was.

**Context is added by whoever has it.** The parser is handed text and does not know the path;
the caller read the file and does. So the type stays about the failure and the caller attaches
the article to it, rather than every layer carrying a path it never uses.

### At the boundary the chain becomes one message

The two shells converge differently, and both flatten only at the very end.

**The CLI** turns whatever reached it into a single message on stderr and a failing exit code.
Nothing above that point needs the distinction, because there is nobody left to act on it --
`anyhow` is the shape of that boundary, holding the chain until the moment it is printed.

**The desktop shell** keeps the chain, because it has more than one thing to do with it: an
Activity entry records what failed and where, while what a person is shown is the same flattened
sentence the CLI would print. Discarding the cause at the adapter would leave the record as
useless as the message.

So the rule is directional. Types going up, one message coming out, and the flattening happens
once, at the shell, never in the middle.

**Adoption is partial and deliberate.** The CLI boundary is in place: every command returns
`anyhow::Result<ExitCode>`, and `run` is the one place that prints. What still returns a bare
`String` is named rather than left to be found -- `licenses::npm::collect`, `i18n::parallelism`,
`i18n::selected_locales`, `runner::model_override` and the opengraph renderer. Each is bridged
with `anyhow::Error::msg` at the call, which makes the string the message and loses it as a
cause; converting them to `thiserror` is worth doing as its own work.

### `Err` is "could not run"; a failing exit code is not

A command returns `Ok(ExitCode::FAILURE)` when it ran and has something to report -- items that
failed inside a batch that finished -- and `Err` only when it could not run at all. Collapsing
the two would make `cms alt` over a library where one description failed indistinguishable from
`cms alt` in a directory that is not a repository, and the second is worth a different reaction
from whoever typed it.

## Unused is not the same as dead

A thing with no consumer today is not automatically waste. **The question is whether it completes
a set that would be incoherent without it**, and if so it stays -- reported once here rather than
rediscovered as a defect by every review that greps for callers.

Two members of the tree are there on exactly this basis:

- `--color-green-ink` and `--color-red-ink` in `libs/tokens`. Blue's is in use; the three are one
  set of hues under one naming scheme, and deleting two of them leaves the next component wanting
  a green mark either inventing an `oklch` or borrowing a name that means something else.
- The italic and bold cuts of Ioskeley Mono. Only the regular weight is reachable under the
  current highlighting themes -- see [fonts.md](architecture/fonts.md) for the measurement -- and
  a monospace family cut down to one weight is a family that has to be re-cut the first time
  anything wants emphasis.

**What makes this safe rather than an excuse is the cost.** Both are paid in storage nobody reads
and bytes nobody downloads: a `@font-face` is a declaration, not a request, so an unreachable cut
costs a reader nothing at all. The same argument does not licence an unused dependency, an unused
export or an unreachable branch, each of which is paid on every build, every audit and every read.

So the test has two halves, and both must hold: **would the set be incoherent without it, and is
its cost paid by nobody?** Where the answer to either is no, it is dead and goes.

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

### `FIXME` is a problem; `TODO` is a plan

Two markers, and the line between them is whether anything is **wrong**.

|         | Means                                                    | Deleted when                            |
| ------- | -------------------------------------------------------- | --------------------------------------- |
| `FIXME` | Something is wrong here and is waiting on something else | the code is fixed, or `spec/` adopts it |
| `TODO`  | Nothing is wrong; something is planned and not built yet | it is built, or the plan is dropped     |

**`FIXME` is a debt.** The code knowingly departs from a rule in `spec/`, or behaves in a way
somebody would call a bug if they met it cold. It says which rule, why it stands, and what has to
happen first. `cms tn` and `cms embed` carry one because their operations live inside the CLI
adapter, which [architecture/cms.md](architecture/cms.md) does not allow.

**`TODO` is not a debt**, which is why it needs its own word rather than a softer `FIXME`. It
marks something deliberately unfinished with no bad consequence while it waits -- a value
captured for a component nobody has written, a hook left where an extension will go. Nothing
misbehaves; there is simply less than there will be. `VITE_COMMIT_HASH` is the standing example:
the build captures it because only the build can, and the footer meant to show it is planned.

Both say what they are waiting for. A marker that does not is indistinguishable from one nobody
has revisited, which makes every marker in the tree worth a little less.

**Why mark at all, rather than fix or forget.** An unmarked departure gets rediscovered by every
review as though it were new, and each rediscovery costs the same conversation about whether it
was a decision or an oversight. The other way out -- writing an exemption into `spec/` -- is
worse for a `FIXME`: an exemption reads as settled, so the thing stops being a departure and the
code quietly becomes the rule. The debt stays where the code is, visible to a plain search.

Keeping them apart is what keeps either useful. A tree where both words mean "look at this
sometime" has one marker wearing two spellings, and a search for real problems returns a list
nobody finishes reading.
