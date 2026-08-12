# Font pipeline

The [architecture spec](../../spec/architecture.md#latin-and-cjk-use-different-slicing-strategies)
owns the decisions behind disposable inputs, independent runtime dependencies, and the two
slicing strategies. This is the runbook for applying them.

[`data/fonts.json`](../../data/fonts.json) is the authored manifest for every published web-font
family. It lives under `data/` because it is a curated, language-neutral asset record consumed by
the Python pipeline. [`@canmi/fonts`](package.json) owns the consumer boundary: its root export
imports that source and exposes `fontFamilies`, a typed list containing each family's id, display
name, and package stylesheet. Site and editor code therefore do not reach into `data/` or repeat a
second font list.

```ts
import { fontFamilies, type FontFamily } from '@canmi/fonts';
```

Run the check without touching published font bytes:

```sh
mise run fonts --check
```

Build one generated family, or every generated family:

```sh
mise run fonts lxgw-wenkai
mise run fonts --all
```

The check validates the manifest against every published directory and stylesheet, then performs
a one-glyph smoke slice through the same `cn-font-split` Node FFI module used by a build. A green
check therefore proves that the platform core loads and produces WOFF2 output.

The build writes to a sibling temporary directory and replaces a family's published directory
only after `cn-font-split` succeeds. `cn-font-split` is an exact root `devDependency` because the
task imports its Node module. Its allowed install script installs the platform core under
`node_modules`; `mise run fonts --check` exercises that native path, so a missing or unloadable
core fails the check rather than producing a misleading inventory-only success.

The splitter is not byte-reproducible across runs: the same input and options can change one or
more content-hashed chunk names. Do not casually rebuild an already-published family. Generate it
in a temporary directory and compare its chunk-name fingerprint and formatted stylesheet first;
atomic publication prevents partial output, but it does not prevent cache-URL churn from a
successful re-slice.

## Manifest entries

Use `named-subsets` for a small Latin-script face. List each face's `stem`, `style`, `weight`,
`input`, and writing-system `subsets`. The pipeline emits `{stem}-{subset}.woff2` and the matching
`unicode-range` declarations.

Use `frequency-chunks` for a large CJK face. It accepts one face and emits 32-character
content-hashed chunk names plus its generated stylesheet.

Set `chunks` to `prebuilt` when output is complete and its input is no longer retained. The build
command then reports the family as complete without looking for an input. Set `runtimeInput`
only when application code independently opens a full face, and name that consumer beside it.
Set it to `null` when the full face is a pipeline input only.

Current generated CJK families are LXGW WenKai and 糖果味的夏天. The latter uses the internal
family name `TGWDXT` and covers 7,864 codepoints: 6,864 CJK, 95 Latin, 83 hiragana, and 86
katakana. Its CJK coverage selects `frequency-chunks`; no application opens its full TTF at
runtime.

## Add or update a family

1. Put each full face needed for slicing under `data/fonts/`; do not fetch fonts in the task.
2. Add the family and its explicit strategy to the manifest. A generated face needs an `input`;
   a prebuilt face may set it to `null`.
3. Run `mise run fonts <family>` and then `mise run fonts --check`.
4. Apply the input-retention decision recorded in the
   [architecture spec](../../spec/architecture.md#a-font-pipeline-input-is-disposable).
