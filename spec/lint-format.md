# Linting and formatting

## The boundary

**A linter owns semantics. A formatter owns layout.** Anything the formatter already
rewrites, the linter must not report. This holds for every language in the repo, not just the
ones listed below.

The reason is that two tools with authority over the same byte disagree eventually, and when
they do, the loop never converges: the formatter rewrites, the linter complains, a fix
re-triggers the formatter. Deleting the overlap is the only stable arrangement.

## Assignment

| Language                       | Formatter (layout) | Linter (semantics) |
| ------------------------------ | ------------------ | ------------------ |
| TypeScript, JavaScript, Svelte | oxfmt              | oxlint             |
| Rust                           | rustfmt            | clippy             |

All four are installed and versioned by mise; see [toolchain.md](toolchain.md).

## Formatting is applied, never negotiated

Formatting is written, not checked. `jj fix` pipes changed files through oxfmt and rustfmt
and rewrites the revisions in place -- history included, not just the working copy. There is
no "check formatting" step to pass and nothing for an agent to decide.

So: **never hand-format, never argue with the formatter, never turn a rule off to make a
file pass.** If the formatter and the linter want different bytes, that is a broken
configuration, not a judgement call. Fix the config and record why, per the section below.
An agent that finds itself reasoning about whitespace has already gone wrong.

Because `jj fix` repairs history rather than guarding a commit, formatting can never be
"too late" here. A badly formatted commit from last week is one `jj fix -s` away from
correct.

## Baseline

Start from each tool's own default configuration. Deviate only to resolve a real conflict or
a real defect, and record the reason in a comment next to the setting. A config file in this
repo should read as a short list of justified exceptions, not a restatement of the defaults.

## Adding a linter or formatter

Whenever a new tool of either kind is added, do this before committing it:

1. List the rules the linter enables by default (`oxlint --print-config`, `clippy -W help`,
   or the equivalent).
2. Find the ones that overlap with what the formatter rewrites.
3. Verify the overlap empirically -- feed the formatter a file that violates the rule and see
   whether it fixes it. Do not settle this by reading rule names.
4. Turn the overlapping rules off on the linter side, with a comment naming the formatter
   that owns them.

## Recorded decisions

**`no-irregular-whitespace` is off in oxlint.** Verified by writing a U+00A0 into a `.ts`
file: oxfmt rewrites it to a plain space unprompted. Leaving the rule on would report a
problem the formatter has already solved.

**An oxlint disable comment takes no reason suffix.** ESLint 9 allows
`// eslint-disable-next-line rule -- why`, and oxlint does not: the ` -- why` is read as part of
the rule list, matches no rule, and the whole directive is silently ignored. Nothing is
reported, so the only symptom is the original warning still being there. Write the reason as an
ordinary comment on the line above and keep the directive bare:

```ts
// Reason the rule does not apply here.
// eslint-disable-next-line no-new
```

The suffix form used to survive in the tree on `svelte/` rules, where the broken syntax never
surfaced because those rules can never fire -- see the section below. Every such directive has
since been replaced by a plain comment stating the reason, which is also the only correct form
for a `svelte/*` rule name: a directive naming a rule that does not exist is decoration.

**clippy carries no style lints.** When Rust code lands, clippy stays on `correctness`,
`suspicious`, and `complexity`. The `style` group overlaps rustfmt and stays off unless a
specific rule is shown to cover something rustfmt does not touch.

**oxfmt never touches `contents/`.** An article's bytes are load-bearing in a way code's are
not: `data/build/segments.json` fingerprints byte ranges of each article, and the translation
sidecars key paid work off those fingerprints. A formatter reflowing frontmatter produces a
semantically identical file whose segments all read as edited -- a stale layout error at build
time, and re-translation charges for prose nobody changed. Verified when the svelte option
landed: reflowing one article's frontmatter broke the corpus tests until the file was
restored. `.oxfmtrc.json` ignores the directory, and the ignore holds for `--stdin-filepath`
runs too, so `jj fix` inherits it.

## What oxlint sees in a `.svelte` file

Verified against oxlint 1.75.0 by probing a file that violates two rules at once:

| Region                        | Linted                                            |
| ----------------------------- | ------------------------------------------------- |
| `<script>` block              | Yes -- `no-debugger` reports normally             |
| Unused bindings in that block | No -- suppressed, since the template may use them |
| Template markup               | No -- not parsed at all                           |

**There is no svelte plugin and none is being written.** oxlint ships fifteen built-in plugins
and `vue` is the only framework among them; there is no `--svelte-plugin` to match
`--vue-plugin`. The upstream compatibility table records "No template linting yet" for Svelte
and Vue alike, and template support is an open issue with no date.

The consequence: any `svelte/*` rule name is inert here. It is not disabled, it does not exist,
and a disable comment naming one is decoration. Write the reason as a plain comment.

Do not reach for ESLint to close the gap. Running `eslint-plugin-svelte` beside oxlint puts two
linters over the same files, which is the arrangement the boundary at the top of this file
exists to prevent. What covers svelte templates today is `svelte-check`, which `verify` already
runs; it answers a type question rather than a lint one, and that is the coverage there is.

## Indentation

Tabs, width 2, in every language including Rust and TypeScript.

`.editorconfig` is the single source of truth. oxfmt reads it directly, so `.oxfmtrc.json`
deliberately sets no indentation keys. rustfmt does not read it, so `rustfmt.toml` restates
`hard_tabs` and `tab_spaces` -- the one place the same fact is written twice, and only
because rustfmt gives no alternative.

YAML is the sole exception: the YAML specification forbids tabs for indentation, so
`[*.{yml,yaml}]` uses 2 spaces. This is a language constraint, not a preference.
