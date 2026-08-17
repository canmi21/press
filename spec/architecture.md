# Architecture

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
subjects: [data.md](architecture/data.md), [media.md](architecture/media.md),
[fonts.md](architecture/fonts.md) and [delivery.md](architecture/delivery.md).

## One name, one thing

A directory under `libs/` is a namespace, not a language choice. `libs/imgsrc` is imgsrc
-- whether that is a Cargo crate, a TypeScript package, or a Rust core with a TypeScript
wrapper around it is an implementation detail living inside.

This is what makes the cross-language plan work: a library whose core is Rust compiled to
wasm and whose surface is TypeScript is still one directory with one name. Splitting
libraries by language at the top level would tear that library in half.

The same applies to `apps/`. A Rust binary and a SvelteKit site sit side by side, named for
what they do.

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
[`@canmi/primitives`](../libs/primitives/src/style.css). Both applications consume it directly;
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

### The subscription surface closes both reading paths

The same Newsletter component appears on the homepage and after the body of every article.
The homepage reaches somebody browsing the site; the article tail reaches somebody who has
finished reading. These are two entrances to one subscription, so they share copy, state and
presentation rather than growing page-specific variants that can drift apart. On the homepage,
Newsletter precedes Support so the larger subscription invitation remains part of the reading flow
and the smaller actions finish the page.

An article separates the invitation from its authored body with the same quiet one-pixel rule
used by the homepage's structural surfaces. The rule belongs to that placement, not to the
Newsletter default, because the homepage already arrives at it across a section boundary.
The invitation sits after the semantic `<article>`, not inside it: the table of contents scans
that boundary, so only headings authored as article content can enter its navigation.

Homepage-only interaction stays outside it. Support actions describe the site as a whole and
would turn every article ending into a second homepage footer; an article page ends after its
subscription invitation instead.

### Compact action rails reveal detail on demand

The homepage Support surface holds Like, Google source preference and Sponsor. These are reader
actions and read as one small section; revision and Follow stay off the page until they have a
quieter placement of their own. Visitor, uptime, word-count, update-age and license rows do not
appear on the homepage.

Each Support action presents an icon and its shortest useful identity at rest, while pointer hover
and keyboard focus reveal the full localized instruction in place.

The rail measures each localized short and long label, then springs the button between those live
widths with `motion`. This is computed geometry rather than a fixed hover target: locale, font and
the Like count all change the answer. When the short label is a substring of the instruction, that
shared text stays as one DOM segment. Prefix and suffix segments sit in zero-width masks driven by
the same spring as the pill: a suffix is uncovered after a stationary label, while a prefix pushes
the shared label right as it is uncovered. This makes the copy read as material revealed by the
pill rather than one string replacing another. Every shipped locale preserves that substring for
all three Support actions, with a message contract test guarding the relationship. The component
keeps a crossfade only as a defensive fallback; these actions must not rely on it. Translations
choose an idiomatic local short label first rather than forcing an English noun into every locale.

Like keeps its remembered state legible without making the whole rail permanently heavy. A click
fills the heart and updates the count; leaving returns the button to the ordinary paper surface.
Hovering or focusing a remembered Like inverts it to the ink surface. The same state changes must
remain understandable through `aria-pressed`, and reduced-motion users get the final labels without
the width transition.

Sponsor is deliberately unavailable while U.S. F-1 immigration restrictions apply. Activating it
opens a modal notice instead of navigating away. The rest of the page blurs behind the modal, and
either the close control or any point on that background dismisses it.

That notice is interface copy, so it resolves through the UI message table at the page's own
locale like every other string around it -- heading, sentence and the close control's label
alike. It names the restriction plainly in all nine views rather than softening to a generic
"unavailable": the reader is being told why an offered action does not work, and a reason that
survives translation is the only version of that sentence worth having.

Its heading is visible rather than announced to assistive technology alone. A modal carrying one
sentence and a bare close control reads as a fragment of the page rather than a surface of its
own, so the notice opens with the icon of the action that summoned it beside a heading weighted
like the page's other section headings, with the sentence below in the metadata text colour. The
icon and the close control are each centred on one line box, so a heading that wraps in a longer
locale moves the text without dragging them out of line with its first line.

Data palettes belong to the visualisation that gives them meaning, not to the site theme. The
Cargo palette lives in a component-only stylesheet scoped below `.cargo-widget`; it stays vivid
in both page themes and never becomes a token available to unrelated interface chrome.

### Motion runs at runtime only when the value is not known in advance

`motion` is a dependency, and reaching for `animate()` is the wrong default. It earns its place
where the target is computed -- the article list measures the corpus before it knows what widths
to animate to, and no stylesheet can hold a number that does not exist until the page has read
its own content. When a hover, open or state flip has targets written in the source, running it
through a library puts a per-frame JavaScript cost on an animation CSS was going to composite
anyway.

Wanting spring physics is not a reason to cross that line. A spring is a curve, and a curve can
be sampled once and written as a CSS `linear()` easing -- which is what the library itself emits
when it hands an animation to the browser. Sample it from `motion`'s own generator so the
physics are not reimplemented by hand, then paste the result. The repo keeps the real curve and
spends nothing at runtime.

Sampled once means stored once. The curve lives in `--ease-spring` and every consumer reads it
from there; a second copy of those points is how two animations meant to feel identical begin to
drift apart.

One trap worth stating, because it is invisible until someone wonders why the bounce never
shows: an overshoot has to have somewhere to go. A spring driving `background-size` or a colour
is clipped at its limit, so the overshoot is spent on nothing and the curve should simply be
damped out. A transform or an unconstrained layout dimension such as width has room to show it.

## Workspace wiring

### The desktop CMS is a resident process, not a viewer

The Tauri client stays running. It exists to do two things the command-line shell structurally
cannot: **run the periodic work on a schedule**, and **be the editor articles are written in**.
Everything else it displays is in service of those two.

That is what makes it resident rather than a window someone opens to look at numbers. A schedule
kept by a process that is only alive while a person is watching is not a schedule, and an editor
that has to be relaunched to save is not an editor. Both requirements land on the same place: the
application layer below the shells has to own long-running work and its record, because a CLI
invocation exits and takes its state with it.

The scriptable shell keeps its own reason to exist: builds and local workflows drive it, and a
scheduled task must remain runnable by hand without the desktop app installed. Neither shell is
the fallback for the other.

An operation that runs on a schedule needs things a one-shot command never did -- what ran, when,
whether it succeeded, what it spent, and what must run before it. Recording that is the
application layer's job under the rule below, not a page's.

#### A page offers only work the task substrate can run

A view that has found outstanding work shows the command that closes it. The command becomes a
button only after the operation has moved below both shells and the task substrate can report its
progress and refuse a second copy; known but unmigrated operations remain text. The Derived page
implements that boundary in [derived.ts](../apps/cms/client/derived.ts), while the task centre will
eventually provide the complete catalogue and scheduling surface.

The reason is what these operations are. They run for minutes, several of them spend money on a
model, and they are not safe to run twice at once over the same files. A control that cannot be
watched and cannot reject duplication is a worse version of copying the command, because it looks
like it did something. Half of a run mechanism is not a smaller version of one; it is the part that
lies.

An interactive run calls the same in-process application operation as the CLI. The GUI never owns
a second implementation and never turns terminal output into an API. The Tauri adapter is in
[main.rs](../apps/cms/src-tauri/src/main.rs).

A class that spends money says so wherever it is offered, before anybody reaches for it. A paid
operation does not become a button until that warning is part of the path that starts it.

### The CMS has two shells and one home

`apps/cms` owns both ways a person reaches content management. Its existing Rust binary is the
scriptable shell used by builds and local workflows; the Tauri client is the interactive shell.
They remain in one app because deployment shape does not create a second responsibility. The
Tauri crate is nested at the framework-defined `src-tauri` boundary while the frontend stays a
small Vite entry beside it. Shared CMS operations move behind modules both shells can call when
the interface begins to expose them.

Every CMS capability has one in-process application operation and two optional adapters. The CLI
may expose that operation as a command, and the GUI may expose it through a typed Tauri command,
but neither adapter owns the work. In particular, the GUI never spawns the CLI as a subprocess:
doing so would turn terminal output and exit codes into an accidental internal API, duplicate
process lifecycle concerns, and make the desktop application depend on a separately discoverable
binary. Keeping the operation below both shells gives interactive actions, scripts and scheduled
tasks the same validation, effects and errors. The cost is an explicit library boundary and a
small adapter in each shell; capabilities that exist in only one interface have not yet reached
the shared CMS application surface. An operation that exists for only one provider is still a
shared operation, just not a runner choice -- see [x.md](x.md).

The desktop entry starts empty and takes its colours from `@canmi/tokens`; a second design system
does not begin at the window edge. Its native title follows the HTML `<title>` as that value
changes, and the frontend receives only the Tauri permission needed to do that. The active page
owns that title: an outer page starts with its own name alone and may append the detail it opens,
while the shell does not repeat `CMS` as a parent suffix on every page. `app.canmi.cms` is the
application identifier: it follows reverse-domain order for `canmi.app` and does not end in macOS's
`.app` bundle extension. Platform icon variants live at Tauri's `src-tauri/icons` boundary and the
bundle names them explicitly. Both browser interfaces use Tailwind, so the desktop client consumes
the same Tailwind-facing token surface as the site rather than maintaining an adapter of its own.
Tauri's capability schemas under `src-tauri/gen` are generated build output: they stay untracked
and are excluded from repository reference checks.

On macOS the WebView extends through the title-bar area. The native title and title-bar surface are
hidden, while the native traffic lights remain independently visible over the interface. Keeping
the decorated window with an overlay preserves those platform controls; removing decorations would
remove them as well and turn their behaviour into application code.

The window and sidebar are unpainted, revealing macOS's semantic Sidebar material, while the main
content is one opaque Web surface inset from the top, bottom and right edges. That inset matches the
native traffic lights' distance from the window edge instead of introducing an unrelated frame.
The sidebar begins below a dead zone containing that inset, the controls' height and the same inset
again, so navigation never competes with window chrome. That dead zone extends across the full
window as a fixed, topmost transparent hit surface, so content paint order cannot intermittently
take the drag gesture; double-clicking it retains the platform title bar's maximise behaviour. Native
chrome colours remain a small light-and-dark token group in `@canmi/tokens`: surface, divider, hover
and selection. Selection is deliberately stronger than hover because a persistent location must
remain identifiable without pointer movement. Their alpha is part of each colour rather than an
element-wide `opacity`, because chrome may be translucent without fading its text and icons. The
transparent WebView support this requires macOS private API and therefore trades away Mac App Store
eligibility; the CMS is a local workspace tool, so the native material is the chosen side of that
trade.

The sidebar reads the site's name from `site.config.yaml` rather than carrying a second identity.
It sits in a row of its own immediately below the drag region. The row is two and a half times the
text's line height and centres the line vertically while keeping its own left inset, so title
geometry is independent of both the window controls above and the navigation below.

The shared visual language extends beyond the palette. The CMS uses the site's quiet text
hierarchy, generous content spacing, hairline borders, paper only for contained surfaces and
restrained line icons. Task pages do not acquire branded tiles or ornamental status chrome merely
because the CMS is an operations tool. The opaque main pane is already the content surface, so
Overview does not subdivide it into a dashboard of cards. Metrics and sections sit directly on that
surface and use spacing and hairline dividers for grouping; even an empty health state remains text
rather than acquiring another inset box.

Overview is a workspace brief, not an inventory dashboard. Its one headline says whether anything
needs attention, and real check findings become the body when the answer is yes. Article and media
counts are a quiet metadata sentence beneath that state instead of four equally weighted metrics.
Distribution charts are absent: they describe the corpus without giving the writer an object to act
on. Recently modified article titles and subtitles supply those objects, ordered by authored
`lastmod` with `created` as the first modification. The brief follows the public site's article
column width so one subject owns the reading path; wider inventory views keep their own geometry.
Its top inset is the same responsive length as its horizontal inset, making the brief one balanced
sheet within the main pane. Compact marks identify the live workspace state and actionable
attention; they sit immediately after their labels without entering the text flow, while ordinary
section labels remain text-only. Recently updated rows reuse the public homepage's article preview:
row geometry, paper thumbnail, title, dotted leader, date and subtitle are one
`@canmi/primitives` surface. The site keeps its link, focus, hover and content-derived line motion;
the CMS keeps a read-only static rendering. Those are consumer behaviours rather than two visual
definitions. Labels and article copy retain one uninterrupted left edge, and individual facts stay
unbadged so the icons establish hierarchy without turning every piece of content back into
interface chrome.

Articles is the writing library rather than a translation-coverage dashboard. It keeps section
grouping but presents each article with the same title, paper thumbnail, authored date and subtitle
primitive used by Overview and the public homepage. A healthy article carries no completion badge,
progress bar or repeated locale strip: completion is the resting state, and repeating it across the
page makes status look like the subject instead of the writing. Only work that needs attention --
missing translation segments, missing summaries or stale segments left by an edit -- adds a detail
line to its article. This flat inventory may be wider than Overview's reading column, but it stays on
the main pane instead of dividing every article into a separate card.

The first window is a centred 1280 by 720 logical pixels, a 16:9 default rather than a minimum or
a fixed canvas. After that first launch, geometry belongs to the native shell: Tauri's window-state
plugin saves size, position and maximised state in the application's config directory and restores
them before showing the next window. `localStorage` holds page state, not coordinates whose meaning
depends on monitors and their scale factors. The configured window begins hidden so restoration
does not flash the default rectangle before moving to the saved one.

Theme behaviour is shared separately from its colour values. `@canmi/tokens` remains the palette;
`@canmi/theme` owns the system dark-mode query and the site's pre-paint bootstrap. The desktop shell
follows that system query live, while the public site can still honour its explicit `theme` cookie.

The WebView is one application shell with a persistent left sidebar. Its top-level destinations are
Overview, Articles, Media, Automations and Activity: content and resources are things to manage,
while scheduled work and its history are separate views of what the CMS does to them. Individual
CLI commands do not become navigation destinations. They become tasks inside Automations, with
their runs reported by Activity, so adding another operation does not make the application's
information architecture wider.

The CMS interface is `en-US` only. It is an authoring and operations tool for the local workspace,
not a reader-facing surface, so it does not carry a locale selector, message catalog or translated
UI copy. Internationalisation belongs to interfaces the site's readers use; the CMS manages that
content without localising itself.

`dev-cms` enables the MCP bridge as an optional Cargo feature and exposes Tauri's JavaScript global
only through its runtime development config. The bridge is additionally gated by Rust debug
assertions and binds to loopback, so an agent can inspect and evaluate the native WebView without
putting a debugging server in a release client or on the local network.

The two package managers disagree about strictness, and the layout has to respect that.

**pnpm globs.** `pnpm-workspace.yaml` uses `libs/*` and `apps/*`. pnpm only picks up
directories containing a `package.json`; Rust-only directories are invisible to it. Adding a
Rust library requires no pnpm change.

**Cargo does not glob.** `Cargo.toml` lists members by hand. Cargo errors on any
glob-matched directory that has no `Cargo.toml`, and that error breaks _every_ cargo command
in the repo, not just the one crate. Verified: with `members = ["libs/*"]` plus an `exclude`
list, adding one TypeScript library and forgetting to exclude it takes the whole workspace
down. With explicit members, new TypeScript libraries have no effect at all.

The cost is one line in `Cargo.toml` per Rust crate. The failure it buys off is a hard stop
triggered by the most routine action in the repo.

## Naming

One word, or an abbreviation that can be read aloud. Roughly 4 to 8 characters. Lowercase,
no hyphen unless the name is genuinely two words. See [naming.md](naming.md) for the
filesystem rules that apply inside these directories.

Name for responsibility, never for deployment shape or product. `cdn` describes what it is;
`res` described a slot it happened to occupy. Product and domain names are the worst
candidates of all -- they change. A domain that was a content site can become a redirect
without a single line of its code changing.

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
[toolchain.md](toolchain.md). JSON-LD `@context` values and XML namespaces are exempt for a
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
[`apps/cms/src/urls.rs`](../apps/cms/src/urls.rs) -- committed, like the records under
`data/build/`, so a checkout compiles without Node having run first. The mirror is never
edited by hand: [`rust.test.ts`](../libs/urls/src/rust.test.ts) fails `verify` the moment it
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

`robots.txt` follows the same shared-base shape, and lives in `libs/robots` rather than in
`libs/urls`. It exports the minimal common definition plus a helper that appends site-specific
rules -- disallowed paths, sitemap entries -- so each site owns its additions while a change to
the shared policy reaches all of them at once. It sits in its own library because generating a
file is not the same job as mapping URLs, even though it consumes them.
