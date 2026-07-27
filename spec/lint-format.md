# Linting and formatting

## The boundary

**A linter owns semantics. A formatter owns layout.** Anything the formatter already
rewrites, the linter must not report. This holds for every language in the repo, not just the
ones listed below.

The reason is that two tools with authority over the same byte disagree eventually, and when
they do, the loop never converges: the formatter rewrites, the linter complains, a fix
re-triggers the formatter. Deleting the overlap is the only stable arrangement.

## Assignment

| Language | Formatter (layout) | Linter (semantics) |
| --- | --- | --- |
| TypeScript, JavaScript, Svelte | oxfmt | oxlint |
| Rust | rustfmt | clippy |

All four are installed and versioned by mise; see [toolchain.md](toolchain.md).

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

**clippy carries no style lints.** When Rust code lands, clippy stays on `correctness`,
`suspicious`, and `complexity`. The `style` group overlaps rustfmt and stays off unless a
specific rule is shown to cover something rustfmt does not touch.

## Indentation

Tabs, width 2, in every language including Rust and TypeScript.

`.editorconfig` is the single source of truth. oxfmt reads it directly, so `.oxfmtrc.json`
deliberately sets no indentation keys. rustfmt does not read it, so `rustfmt.toml` restates
`hard_tabs` and `tab_spaces` -- the one place the same fact is written twice, and only
because rustfmt gives no alternative.

YAML is the sole exception: the YAML specification forbids tabs for indentation, so
`[*.{yml,yaml}]` uses 2 spaces. This is a language constraint, not a preference.
