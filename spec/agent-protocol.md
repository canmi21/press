# Agent protocol

How an agent operates in this repo: how to start, who decides what, and how to keep this
ruleset trustworthy over time.

## What spec is for

`spec/` records **decisions and the reasoning behind them**. It is not documentation of the
code, and it does not restate implementation.

- **Write down**: what was decided, why, what the alternative was, and what it costs. Anything
  a future reader could not recover by reading the source.
- **Leave out**: how a function is written, what a config file currently contains, anything
  the code already states plainly. Point at the file instead of copying it.

The test for a spec sentence: if someone changed the code, would this sentence need editing?
If yes, it is describing implementation and belongs in a comment next to that code. If it
would still be true, it is a decision and belongs here.

## One fact, one home, linked from everywhere else

Nothing is written down twice. Every fact has exactly one home, and everything that needs it
links there.

| Layer         | Holds                     | Points at                        |
| ------------- | ------------------------- | -------------------------------- |
| `CLAUDE.md`   | one line per topic        | the full `spec/*.md`             |
| `spec/*.md`   | the decision and why      | the code file that implements it |
| code comments | why this line is this way | back to `spec/*.md` for the rule |

Spec and code are not two descriptions of one thing -- they are **one description with two
halves**. Spec holds the decision, code holds the implementation, and each names the other so a
reader can cross the gap in one hop. A spec file that restates what the code does will drift
away from it; a code comment that re-argues a decision makes the decision editable in two
places, and then it is editable in neither.

When you catch yourself writing something that already exists elsewhere, delete it and link
instead. The duplicate is not redundancy, it is a second thing to keep in sync, and it will
lose sync.

`mise run refs` checks that the links resolve -- markdown links in docs, and `spec/*.md`
citations in code. It is part of `mise run verify`, because a reference that points nowhere
fails silently: nothing breaks, the reader just follows a dead path. It deliberately ignores
path-shaped strings in prose, since the rules use invented names as examples on purpose.

Spec exists to be the place an argument gets settled. When code, an agent, or a later opinion
disagrees about how something should work, this is the reference that decides it -- which only
works if it carries the _why_. A rule with no recorded reason loses every argument against a
plausible-sounding alternative, because nobody can tell whether it was considered or just
inherited.

## Cold start

Starting a conversation with no context about this project:

1. Read `CLAUDE.md` in the repo root.
2. Read the `spec/` files that cover the task at hand.
3. Only then act.

Never act first and consult the rules afterwards. If a rule would have changed what you did,
reading it late is worth nothing.

## Decision authority

Decisions belong to the user. An agent implements them.

Ask the user when:

- The task needs a choice `spec/` does not already answer.
- The choice is structural: architecture, dependency, data model, public API, naming scope.
- The choice is expensive or awkward to reverse.

Do not ask when:

- `spec/` already answers it. Follow the spec.
- The user's described approach implies the answer. An implication carried in the user's own
  wording counts as their decision -- honour it rather than re-asking.
- The choice is local, reversible, and leaves no trace in the result.

Calibration matters in both directions. Escalating every trivial choice wastes the user's
attention; silently settling a structural one takes a decision that was never yours. When
genuinely unsure, ask one focused question instead of a list.

## Checking your own work

Make the change and hand it back. Do not stand up a browser to confirm that a colour is the
colour you just typed, that a radius applied, or that a class landed -- the type checker and
the build already catch the failures that class of edit can have, and the user is a faster
judge of the rest than any screenshot. They will say when something is wrong.

Chrome DevTools MCP is for debugging that is genuinely hard without it: a value that renders
differently from what the source says and nobody can say why, a layout that only breaks at one
size, something that misbehaves after hydration and not before. The signal is that a question
about the running page cannot be answered by reading the code -- not that a change was made and
might in principle be wrong.

The cost is not the tool call. It is the round trip: booting a browser, waiting on a dev
server, screenshotting, reading pixels back, all to restate what the diff already says. That
time is the user's, and confirming the obvious spends it to reach a conclusion the next message
would have delivered for free.

## Selfcheck

Triggered whenever the user says "remember this", "update yourself", or anything of that
shape.

Do not write to agent memory or any agent-private store. That content does not survive into
the next session's view of this project and is invisible to every other agent. Instead:

1. Decide where the rule belongs: an existing `spec/` file, or a new one named for its aspect.
2. Write it there.
3. Re-read `CLAUDE.md` and check it still holds. Are the rules it promotes still the most
   important ones? Is the index complete? Has anything drifted, duplicated, or gone stale?
4. Repair what drifted, then tell the user what moved where.

The test to apply before calling it done: a fresh agent with zero memory of this project,
starting from `CLAUDE.md` alone, must be able to recover the project's constraints and work
correctly. If it could not, the selfcheck is not finished.

## Unprompted selfcheck

The user will not keep asking for this. Adding a tool, adding a config file, or settling a
convention triggers the same review on its own:

- Does a future agent need a rule to use this correctly, or is the config file
  self-explanatory?
- Did resolving this require a judgement that is invisible in the resulting file? Judgements
  leave no trace unless written down -- a config value looks arbitrary six months later.
- Does an existing `spec/` file now contradict what was just added?

Write the rule in the same change as the tool. A tool that lands without its rule is a tool
the next agent will misuse.
