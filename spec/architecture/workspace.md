# The shape of the workspace

## What this repo is

One folder holding most of what its owner writes, across every language, so that an agent
working in this directory can scan it, cross-reference it, and reuse it without anything
being published first. Source-level reuse is the point. Publishing a package is something a
library earns after it stabilises, not a precondition for using it.

## Layout

```
spec/       Rules. Start at CLAUDE.md, which indexes this directory.
libs/       Libraries, any language.
apps/       Deployable things, any language.
contents/   Articles. Tracked, because prose is revised and wants diffs.
data/       Assets and the records describing them. Bytes stay out of git; records go in.
projs/      Reserved for large standalone projects. Not created yet.
```

Which of `data/` git keeps, and what happens to an asset once it is stored, are their own
subjects: [data.md](data.md), [media.md](media.md),
[fonts.md](fonts.md) and [delivery.md](delivery.md).

## One name, one thing

A directory under `libs/` is a namespace, not a language choice. `libs/imgsrc` is imgsrc
-- whether that is a Cargo crate, a TypeScript package, or a Rust core with a TypeScript
wrapper around it is an implementation detail living inside.

This is what makes the cross-language plan work: a library whose core is Rust compiled to
wasm and whose surface is TypeScript is still one directory with one name. Splitting
libraries by language at the top level would tear that library in half.

The same applies to `apps/`. A Rust binary and a SvelteKit site sit side by side, named for
what they do. What a member may be called is [naming.md](../naming.md)'s.

How the desktop CMS and the command-line shell divide one application between them is its own
subject: [cms.md](cms.md).

## Libraries export source

A TypeScript library's `exports` point at `./src/*.ts`, not at a built `dist/`. There is no
build step, no `dist/`, and no `prepare` script to run before the repo works.

This is the whole point of the repo. A library that must be built before it can be used is a
library with a publishing ritual attached, and that ritual is exactly what stops code from
accumulating. Consumers here are bundlers -- Vite, the Workers runtime, esbuild -- and they
compile TypeScript directly.

The constraint: this holds only while every consumer bundles. A consumer that runs raw
Node against the package would need a build. If that day comes, add the build to that one
library rather than reinstating it everywhere.

## Web interface primitives

Bits UI is the site's headless behavior layer. It owns the difficult, reusable interaction
contracts -- focus management, keyboard navigation, dismissal and floating placement -- while
the site's tokens and local Tailwind classes continue to own site-only visible decisions.
Importing a styled component kit on top would create a second design system, so project primitives
under `apps/site/src/lib/components/` expose the small set of surfaces the site alone repeats.

A visible primitive repeated by both the public site and CMS belongs to
[`@canmi/primitives`](../../libs/primitives/src/style.css). Both applications consume it directly;
neither becomes the other's template, and extracting it must leave the established consumer
visually unchanged. Two real consumers justify that boundary. A single speculative component does
not, because opening a package per primitive turns reuse into directory ceremony rather than a
coherent shared vocabulary.

Feature directories compose those primitives and keep their own state, copy and specialised
styling. A locale picker, for example, imports the shared menu surface but owns language order,
selection and navigation itself. A primitive is added for a real repeated interaction, not to
predict a future component catalogue; unused Button or Input wrappers are not architecture.

Use a primitive where the interaction is conventional and accessibility-heavy, such as a
menu or popover. Do not force data visualisation through it: Cargo and Tokei deliberately keep
one specialised tooltip for hundreds of SVG regions rather than instantiating a general
component per region. Headless is a boundary for shared behaviour, not a requirement that
every interactive pixel come from the same package.

The visual language stays independent of that boundary. Interface chrome is neutral paper,
a quiet one-pixel border, compact type and a small shadow only on floating surfaces; colour is
reserved for focus, state and data. A categorical chart may be colourful, but its controls,
tooltips and surrounding statistics use the same surfaces as the rest of the site.

## A runtime's globals decide which program checks a file

Type checking runs twice, over two programs: [tsconfig.json](../../tsconfig.json) for the browser
and anything indifferent to a runtime, [tsconfig.workers.json](../../tsconfig.workers.json) for
the two Workers and `libs/store`.

The split is forced rather than chosen. `@cloudflare/workers-types` declares its own
`ReadableStream`, `Response` and `Cache`, and the DOM library declares those names too. Nothing
tells TypeScript they describe the same things, so with both in scope every worker value crossing
a shared boundary is a type error. Two workarounds had grown from that one cause and neither
looked related to the other: `as unknown as ReadableStream` across `libs/store`'s public surface,
and a hand-written structural declaration of `caches.default` in `apps/cdn` because importing the
real one would have made every Hono handler disagree about `Response`. Both are gone; nothing
casts across that boundary now.

**A file belongs to the program whose globals it actually runs against**, which is not always the
directory it sits in. `apps/api/scripts/` is a node script and is checked as one. A worker's
tests are checked _with the worker_, because they exercise worker code and mock worker bindings
-- putting them elsewhere pulls the whole worker into a program that has the browser's globals,
which is the thing being avoided.

What the separation exposed is the argument for it. `apps/cdn` polyfills `ImageData` because
workerd has none, and the polyfill needed a `@ts-expect-error` to install itself -- it was being
checked against a browser's `ImageData`, which it is not. The type now lives in
`apps/cdn/worker-runtime.d.ts` and describes what the polyfill supplies, so the declaration and
the implementation are one claim instead of two that happened to agree.

## Grouping threshold

`apps/` is flat. Introduce a grouping directory only once one category exceeds four members,
and let the growth force it rather than predicting it. Four apps do not need a taxonomy;
`api` and `cdn` announce themselves as infrastructure without a parent directory saying so.

## Extraction threshold

Code moves into `libs/` when it acquires a second consumer, not when someone predicts one.
A library written for a single caller is a guess about what the second caller will need, and
the guess is made at the moment least is known. Waiting means the shared shape is derived from
two real uses instead of one real use and one imagined one.

The counterpart matters as much: once the second consumer exists, extract rather than copy.
`apps/api` read its metadata straight out of R2 while `apps/cdn` read the same bucket through
a store that also knew how to read `data/public`, so the API had no local development at all
-- every lookup was a 404 until `--remote` reached a bucket that only production writes. The
copy was not a duplicated function, it was a capability one side silently lacked.

Extraction is also the moment to write the tests that only make sense for shared code. A
private helper is covered by its one caller; a library is not, because the behaviour each
consumer depends on is no longer visible from any single one of them.

## Where volatile facts live

Directory structure is the skeleton: expensive to change, so it may only carry stable facts.
Which domain an app answers on is not stable. That mapping belongs in a typed map in a
library, where changing it is a one-line edit instead of a rename plus every import plus the
workspace globs.

### Every URL is declared once

`libs/urls` is the only place a URL, hostname, or dev port may be written down. Everything
else imports from it. This covers third-party endpoints too, not just our own hosts -- a CDN
we forward images through is as much a URL as a domain we own.

The URL map is grouped by role:

- `apps`: deployable things in this repo, with development and production entries.
- `internal`: domains the repo owner controls, but that are not apps in this repo.
- `external`: third-party endpoints and hostnames.

**The test: who resolves this URL?**

- _The software_ -- it is fetched, linked against, or served from. It goes in `libs/urls`, with
  no exceptions for app code, libraries, stylesheets, or config.
- _A person reading_ -- a link to a standard, a `# see <url>` note. It stays where it is useful.
  Nothing breaks if it rots except somebody's curiosity.

The earlier version of this rule banned every `https://` outside the library, full stop. That
was wrong on the day it was written: this spec cites four external standards, so the rule was
already broken four times by the document stating it. A rule nobody can follow is not a strict
rule, it is a dead one -- it gets ignored wholesale rather than in the one place it should be.

An identity is not an address. A social handle, an email local part, a feed's tag URI --
these say who someone is, and they live in `site.config.yaml` beside the author's name. What
`libs/urls` owns is where to reach them. The two compose: `URLS.external.social.x` plus the
handle is the profile URL, assembled at the point of use rather than stored a second time as
a whole. Putting the handle in the URL library would make the library the owner of a fact
about a person, and the config the owner of nothing.

Names RFC 2606 reserves -- `.test`, `.example`, `.invalid`, `.localhost`, `example.com` and
its siblings -- are exempt as well, and for a stronger reason than convention: the standard
guarantees they never resolve. A placeholder an API needs because it demands an absolute URL,
or a hostname a test supplies precisely so it gets rejected, cannot become a real endpoint by
accident. Exempting them as a class is what stops the check from accumulating one-off
exceptions.

`mise run refs` enforces the first case and skips the second, treating comments, markdown
links, and `$schema` keys as citations. `$schema` has to be a URL here precisely because these
tools come from mise and there is no `node_modules` to point at -- see
[toolchain.md](../toolchain.md). JSON-LD `@context` values and XML namespaces are exempt for a
different reason: each is a namespace identifier, not an endpoint -- changing one changes what
the document means rather than where anything points.

Generated dependency lockfiles are vendor metadata, not an application address source. A package
manager may copy a dependency's deprecation or funding URL into `pnpm-lock.yaml`; the software does
not resolve it, and the next install owns that line. The reference check therefore skips the
lockfile rather than asking `libs/urls` to duplicate metadata that this repository does not control.

The measure this exists to protect: **moving a domain costs one edit to one file.** Every
literal written elsewhere adds one more place that has to be found, and the ones that get
missed do not fail loudly -- they keep resolving to the old host until someone notices the
traffic. This has already happened here once: a `cdn.canmi.net` literal survived inside a
library long after that host stopped being part of the URL map, invisible because nothing
referenced it by name.

**Rust reads the map through a generated mirror.** A Rust process cannot import a TypeScript
library, so `mise run urls` renders the map into
[`apps/cms/src/urls.rs`](../../apps/cms/src/urls.rs) -- committed, like the records under
`data/build/`, so a checkout compiles without Node having run first. The mirror is never
edited by hand: [`rust.test.ts`](../../libs/urls/src/rust.test.ts) fails `verify` the moment it
disagrees with the map, so the one-edit measure survives the language boundary. The
alternative, exempting Rust from the rule, would have left half the repo carrying literals
that the check answers for everywhere else.

Colors follow the same shape at a smaller scale: OKLCH values are declared in
`libs/tokens` and consumed by name. The rule covers the design system that the site's own UI
and theme are built from; a palette mirrored from an external convention keeps whatever
format that convention ships.

Names there are roles the page fills or hues it holds, never the component that first wanted
one. `--color-note-paper` is the shape to avoid: it makes the palette an inventory of features,
so the second component needing that blue either inherits a name describing something it is not
or copies the value. A hue named as a hue -- `--color-blue`, `--color-blue-ink` -- is a pigment
any surface can pick up, and what a blue box _means_ stays a decision the component makes.

The qualifier on a hue names how it is laid down, borrowing the vocabulary the neutrals already
use: `paper` is a surface, `ink` is a mark. Not how dark it is. A name like `-deep` reads as a
promise about lightness that the dark block then breaks, since a page that inverts needs its
marks to move the other way; `ink` stays true in both because a mark is a mark under either
light.

### A card pointing inside the corpus carries no copy of its own

`::article{path=...}` draws the card the homepage lists, for an article in this repository. It
takes only the path: title, subtitle and date are read off the article it names, never written
into the directive. `::linkcard` is the opposite and stays that way, because what it points at
is outside the corpus and there is nothing to read.

The rule is the URL rule one level up. A title is a volatile fact with one home, and a card that
repeated it would be a second copy that only disagrees -- silently, since a stale title still
renders and still links to the right page. It also gets each locale its own translated title for
free, which a written-in one could never have: the directive is one line and a view is one of
nine.

The cost is that the site's content build runs
[two passes](../../apps/site/src/lib/content/build/articles.ts): every view's frontmatter is
read before anything compiles, because the compiler sees one article at a time while a card
names another. A path no article answers to fails the build rather than degrading to a
placeholder -- unlike an embed, nothing has to be fetched first, so an unresolved path is a typo
and there is no working state it could be mistaken for.

Resolving instead at request time, out of the article index, was the cheaper change and is
rejected on what it cannot reach: the feed and `/llms.txt` are strings baked at compile time, so
a card there would have been a bare path where every other link is a name.

`robots.txt` follows the same shared-base shape, and lives in `libs/robots` rather than in
`libs/urls`. It exports the minimal common definition plus a helper that appends site-specific
rules -- disallowed paths, sitemap entries -- so each site owns its additions while a change to
the shared policy reaches all of them at once. It sits in its own library because generating a
file is not the same job as mapping URLs, even though it consumes them.
