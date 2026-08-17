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

### What `data/` keeps out of git

Not the directory -- the kind of thing. **Text that means something goes in; bytes and bulk
stay out.** `data/metadata.json`, `data/media.yaml` and `data/tags.yaml` are records: a build
resolves every image from the first without one image being present, and the other two hold
descriptions that cost money and tags a person curates. Photographs, derived variants, fonts
and a geocoding database are bytes, and no diff of them says anything.

#### Generated build inputs live under `data/build/`

A record a person writes and a record a tool regenerates are both text worth committing, and
they still do not belong side by side. `data/build/segments.json` is the CMS-derived article
segment layout the site assembles from; nobody edits it, and a diff of it is a consequence
rather than a decision.

The split is there to keep the top of `data/` readable. Everything directly under it is
something a person curates and may be asked about; `data/build/` is output, and it may grow a
file whenever a consumer needs one without that growth being a question.

**Which of them git keeps is decided by one question: does a CI build read it?** A site-only CI
build must not need a Rust toolchain to produce its own inputs, so everything it reads is
committed -- `segments.json`, `crates.json`, `repos.json` and `licenses.json`, each named by the
site's Vite config or its content build. A file only the tool that wrote it ever reads is a
cache, not a build input, and stays out: `opengraph.json` records which cards are current so
`cms og` can skip them, and losing it costs one slow rerun rather than a broken build.

The question is deliberately about the consumer rather than about how the file was produced.
Both kinds are generated, both are text, and a rule phrased on "is it derived" would have to
decide the same case twice.

The rule was first written as "`data/` is never in git", which held until it needed several
exceptions. Those exceptions mean the line was drawn around the wrong thing: the directory
groups assets with the records about them, and it is the records that git wants. So
`.gitignore` there is an allowlist, and the question to ask of a new file is whether reading
its diff would ever tell anyone anything.

Articles sit at the root rather than inside the site that renders them, and in git rather
than in `data/`, because they are neither code nor an asset: they are source text that gets
reviewed and rewritten. What makes them git's is the same thing that makes code git's --
someone will want to know when a sentence changed and what it said before, and no backup
answers that. The images they reference are a different matter and live in `data/`, which is
why moving articles out of the app cost nothing: the bytes that would bloat a repository were
never in it.

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

## Data

Git holds code. `data/` holds everything else -- photos, fetched favicons, drafts -- and the
bytes of it are never committed; the records describing them are, by the allowlist described in
[what `data/` keeps out of git](#what-data-keeps-out-of-git). The skeleton is tracked either way,
so a fresh clone has somewhere to put things.

```
data/
  public/   mirrored to R2, 1:1 with the bucket layout
  draft/    never leaves this machine
  build/    generated records; see below for which of them git keeps
```

**The local directory is the source of truth, not a cache of one.** It is R2 laid out as
plain files, which is why local development reads it directly rather than emulating R2. The
bucket is a mirror of `data/public`, and mirroring runs one way: local writes, the cloud
follows. Nothing in the cloud writes back.

That invariant is what keeps the sync trivial, and it is fragile -- a single worker that
writes into the mirror would make the cloud authoritative for those bytes and force real
conflict resolution. Anything the cloud authors (comments, counters, anything a visitor
produces) gets its own storage, outside the mirror, and is cloud-authoritative there.

**R2 is never publicly accessible.** Every read goes through a worker, so cache headers,
routing, and access are decided in code rather than by a bucket setting. Public access would
also be a second, invisible way to reach the same bytes.

### Publication is a path, not a rule

`mise run sync` mirrors `data/public` and nothing else. What makes that safe is the source
path: rclone is pointed at `data/public` and cannot see the rest of `data/`. A directory added
later is excluded because it was never in scope -- no rule to write, none to forget.

The alternative, syncing `data/` minus a denylist, fails in the worse direction. Miss a rule
there and a draft is published silently; miss one here and a file merely fails to appear,
which is visible the moment you look for it. Between a silent irreversible failure and a loud
harmless one, the structure should make the loud one the only option.

The mirror uses `sync`, not `copy`, so deleting locally deletes remotely. That makes a wrong
source path destructive, which is why the task refuses to run without an explicit destination
and dry-runs unless told `--live`.

### Assets are prepared locally, never in CI

A local build fetches whatever it is missing -- remote favicons, image variants -- and writes
it into `data/public` for the next sync. A CI build does neither; it compiles what is already
there.

The split exists because CI has no writable source of truth. If it fetched, the result would
live only in the deployed artifact, and `data/` would no longer be complete. A reference whose
asset has not been synced yet degrades to a placeholder at request time rather than failing
the build -- one missing image is not a reason to block a release.

### Articles decide which assets exist

`cms` reads `contents/`, never a directory listing. What to derive, what to fetch, what is
missing and what is no longer wanted are all answers to one question: which assets do the
articles reference. Something nothing links to is not an asset, it is a leftover.

An image reference is its own state. It either names a file -- looked for under `data/image`,
where originals are kept and never published -- or it is `{cid}.{ext}`, a content id and the
format that was actually produced. `cms image` turns the first into the second, and that
rewrite is the record that the work is done. No log beside the article can drift from it,
because there is no log. The extension is corrected on later runs too: it is a claim about
what the CDN will serve, and an asset stored as PNG must not be referenced as AVIF.

A linkcard's `favicon` attribute is the opposite -- an instruction to the collector, naming
where a site's icon should come from when its own is not wanted. `cms favicon` resolves it
into that domain's slot, and the page always draws `/favicon/{domain}`. So the attribute is
never rewritten: it is the only record of where the icon came from, and destroying it would
make the choice unrepeatable.

Alongside that attribute, `tone` says which shade the named icon _is_. On its own it only says
what the card renders against, which is no instruction to the collector at all.

A site that publishes one icon publishes it for every context, so it is stored under both
tones. The browser draws that same file on light and dark chrome alike, and an icon meant for
only one of them is something a site goes out of its way to declare. This is recording a fact
rather than substituting: the worker still answers a named tone exactly or not at all, because
a light silhouette on a light surface is worse than a missing icon. The collector knows the
site has one icon; the worker only knows which files exist.

### Missing assets are reported, never fatal

Writing an article before importing its picture is a normal state to be in. `cms check` lists
what is absent and always exits zero; a report that can fail a build is a gate wearing a
report's name, and teaches everyone to skip it.

Severity carries the difference. A missing image leaves a visible hole, so it is a warning. A
missing icon leaves a linkcard that still reads correctly, so it is information. A report
where everything is urgent is a report nobody reads.

Deletion is the one thing that never happens as a side effect. `cms gc` is dry by default and
`mise run gc` only reports, because deriving an asset can be repeated until it is right while
deleting one changes what R2 serves on the next sync. It is recoverable in practice -- the
originals are still in `data/image` and a content id is enough to rebuild from -- but that is
a fact about this repository rather than a property of the command, so it waits to be asked.

### A CI build must be able to build from git alone

The site builds from `data/metadata.json`, `data/media.yaml`, the records under `data/build/`,
`contents/` and `site.config.yaml` -- all committed -- and never reads untracked asset bytes.
The merged image manifest carries every dimension, srcset and placeholder, the article segment
record carries the CMS-derived ids and byte ranges, and `cms embed` writes repository and crate
facts for author-written `::github` and `::cargo` directives. The site watches those generated
records as first-class build inputs. It never fetches widget data in the browser or Worker, so
a checkout renders the complete article with neither asset bytes, network access nor a Rust
toolchain present.

The consequence is a rule: **CI compiles, it never derives.** No `cms` command runs there.
`cms image` would write into a `data/` that vanishes with the container, and it could not read
the originals in any case.

Two things a CI build needs that a local one gets for free, so both are pinned rather than
resolved:

- `packageManager` and `.node-version`, because `mise.toml` does not apply outside this
  machine and the lockfile is only readable by a pnpm new enough to know its format.
- `SENTRY_AUTH_TOKEN`, from the platform's own encrypted build variables. `secrets.json` is
  committed but sops-encrypted, and CI holds no age private key -- so the local path through
  mise cannot work there, and the two routes to the same variable stay separate on purpose.

### Assets are addressed by their content

Every published image asset -- an original and each variant derived from it -- is stored under
the hash of its own bytes, BLAKE3 truncated to 128 bits. The identity of a whole asset is its
original's hash; a variant is a separate object with a separate one.

This is what makes long caching safe without a promise to keep. CJK font chunks use the same
property through `cn-font-split`'s own 128-bit content hash, while the small Latin subsets
deliberately keep readable Google-Fonts-style names and therefore still carry the promise that
bytes at an existing name never change. A content-addressed key cannot denote different bytes
than it did before, because changing the bytes changes the key. Re-encoding at a new quality
produces a new object rather than a redefinition of an old one.

**The key is not the URL.** Objects are stored fanned out over the first four characters of the
id -- `{kind}/{ab}/{cd}/{cid}.{ext}` -- and that split exists for a filesystem mirror, which has
a directory that overflows. R2 has no directories to overflow at all. So the fanout is a storage
detail: a caller asks for `{cid}.{ext}` and the worker puts the prefix and the split back on.
Spelling it into a link would publish the bucket's layout as an interface, and an interface is
the one thing that cannot be reorganised later. The licence texts leaked it for exactly as long
as they had no route of their own and fell through to the direct-key handler; adding one was the
fix, not changing where the bytes live.

The relationships -- which variants belong to which asset, their sizes and formats -- live in
the manifest, not in the key layout. The store answers "give me these bytes"; the manifest
answers "which bytes do I want". Deriving one from the other would mean encoding relationships
into paths, which is how a rename becomes a migration.

The truncation to 128 bits leaves roughly 64-bit collision resistance. That is far beyond what
addressing a lifetime of personal assets requires, and is deliberately not a tamper-evidence
claim. It is also unrelated to an IPFS CID, which is a structured multihash rather than a bare
digest.

Recovering an original after a cropped or converted stand-in was published is an identity
migration, not a new description job. An explicit filename pairing establishes which two
sources depict the same asset; their bytes establish the old and new ids. The recovered source
is derived normally so its dimensions and EXIF come from the real file, while article
references, media labels and translated directive segments move mechanically to the new id.
Paid descriptions and tags are evidence about the picture and are never requested again for a
change of source bytes alone. The old aggregate manifest record is removed only after the new
record exists, or commands that enumerate the manifest would mistake the superseded source for
an unlabelled asset; deleting its published bytes still waits for an explicit garbage collection.

### A dependency's licence is an asset like any other

`cms licenses` records every third-party package the deployables are built out of: the
production closure of the three Workers, and every crate this repository's own tooling
resolves. Workspace packages are excluded -- they are this project, not something it credits.

**Packages are identified by purl**, the Package URL that SPDX and CycloneDX already key an
SBOM by: `pkg:npm/%40sveltejs/kit@2.0.0`, `pkg:cargo/serde@1.0.219`. Two registries answer the
same question in different shapes, and adopting the settled vocabulary avoids inventing an
identity scheme whose escaping rules would then be ours to regret.

**The texts are content addressed**, stored under `license/{ab}/{cd}/{cid}.txt` and served as
`/license/{cid}.txt`, exactly like an image and with the fanout hidden the same way. The
registry, the package and the version appear nowhere in a key. That is the rule
above applied rather than an exception to it: package coordinates are not one shape across
registries -- a scoped npm name carries a slash, a Maven coordinate a colon, a Go module a
whole URL -- so encoding them into paths means inventing an escaping scheme that can never be
changed. It also deduplicates by roughly ten to one, because several hundred crates ship the
same Apache-2.0 text byte for byte, and a registry added later will mostly ship texts already
stored.

Texts are published exactly as they were shipped. Normalising line endings would deduplicate
better and would also mean publishing a licence its author did not write, which is not a trade
available on a legal text.

`license/full.txt` is the exception that proves the layout: one aggregate holding every notice
in full, named rather than content addressed, like an OpenGraph card. It is what the permissive
licences actually ask for -- reproducible in one fetch -- and assembling it per request would
mean a Worker fetching several hundred objects.

Only `data/build/licenses.json` is committed; the texts are published bytes and stay out of
git like every other asset. The record is produced locally because the crate half reads the
cargo registry cache, which no CI container has -- so both halves are collected by one command
into one reviewable diff, rather than half the answer arriving at build time.

**A package that declares no licence fails the command.** It is the one finding in the record
that needs a person, and `data/licenses.yaml` is where that person's answer goes, with the
evidence beside it. An entry there only ever fills a gap, never overrides a package's own
declaration, and the published record marks it as asserted rather than declared -- presenting
a judgement as the package's own statement is the one dishonest thing this record could do.

**The name survives from every usable author field; a GitHub login survives only when the
field identifies it explicitly.** Both registries pack a name, an address and a homepage into
one string, in several spellings and sometimes with no brackets around any of them. A copyright
line carries the name, so that is attribution. An exact GitHub profile URL or GitHub's own
no-reply address also names one public account and may supply its login, but no account is ever
searched for or inferred from a person's name. Other email addresses and personal URLs remain
contact details nobody offered for republication and are discarded.

The package record also carries the description, homepage, documentation and repository URL
declared by the registry metadata. Only HTTP(S) URLs become browser links. A GitHub repository
owner may supply the avatar and profile shown on the repository row, but that owner is not
presented as a package author; repository ownership and authorship are separate claims. GitHub
avatars use the CDN's existing avatar proxy, so the site does not add a second live GitHub data
path or expose readers to a new image origin.

Each package also records **one shortest dependency path from every workspace root that reaches
it**. For npm the roots are the deployed `api`, `cdn` and `site` apps; linked workspace packages
remain visible as intermediate nodes even though they are not third-party credits. For Cargo the
roots are Cargo's workspace members and paths follow the resolved dependency graph. Equal-length
paths settle lexicographically so a regenerated record is stable. Keeping one path per root says
every distinct reason the package is present without publishing the combinatorial set of all
equivalent routes through a graph; the page describes these as representative shortest paths,
not as the only possible paths.

That one path is therefore a representative rather than an inventory, so the record also carries
**every package that depends on each one directly**, and the page splits those from the packages
that only reach it through a chain. The shortest path names one parent; a package pulled in by
four of them was answering a question the paths section cannot. The reverse edges come off the
full graph rather than out of the origin walk, whose shortest-path pruning discards exactly the
second and third parent that are the answer here.

**Only the direct edges are stored; the indirect set is derived where it is displayed.** Two
reasons, and the second is the one that generalises. The record is embedded whole into the site
bundle, so anything written into it is weight on every page load, while walking a few hundred
reverse edges per request is free. And a stored closure is a snapshot of the same edges: it can
only ever agree with them or be wrong, which would leave the record holding two answers to one
question. A generated record keeps the primitive facts and lets the derived ones be derived. A
package reachable both ways is listed once, as direct, because the stronger fact is the true one.

`/licenses` is the page over the same data, grouped by licence and ordered by what each covers.
**An expression is flattened to the licences it names, and a package is filed under each of
them** -- somebody looking for what is Apache-licensed here wants the packages that offer it as
one of two. `AND` flattens the same way while meaning the opposite, so the unflattened
expression stays on the row wherever it is longer than the heading; without that the grouping
would read as a claim that plain MIT is the whole of a package's terms. The group counts
therefore add up to more than the number of packages, which the page says out loud.

Splitting is on whole tokens, case-sensitively, because SPDX writes its operators in capitals
and the tree already contains every way that can go wrong: `LGPL-2.1-or-later` carries a
lowercase `or` inside one identifier, `FSL-1.1-MIT` and `MIT-0` contain shorter identifiers,
`Apache-2.0 WITH LLVM-exception` is one licence rather than two, and `(MIT OR Apache-2.0) AND
NCSA` brackets a disjunction inside a conjunction. `/` is Cargo's deprecated spelling of `OR`.
Every expression in the record is a case in the splitter's test table, and a test fails when
the tree grows one the table has not been updated for. The licence is the one column that would otherwise repeat itself hundreds of
times, so it becomes a heading and the rows underneath get shorter -- and the grouping answers
the question somebody arriving actually has, which is what all of this stands on rather than
what any single package is. An asserted licence is not a group of its own: the packages under
it are MIT, they simply never said so, and the row carries where that is known from.

An identifier is not a family label, so `MIT-0` remains separate from `MIT`. SPDX gives
[MIT No Attribution](https://spdx.org/licenses/MIT-0.html) its own identifier because it removes
the attribution paragraph from the [MIT License](https://spdx.org/licenses/MIT.html). Collapsing
the two would make the directory state a notice-preservation condition the package's terms do
not carry; grouping legal terms by resemblance is not normalisation.

The browser surface follows those two kinds of identity instead of nesting one inside the
other. `/licenses` is the licence directory, `/licenses/{licence}` is one licence's package
directory, and `/licenses/pkgs/{type}/{name}@{version}` is one package. A package route does not
sit below a licence route because an expression can place the same package under several
licences; doing so would give one package several equally plausible addresses. `pkgs` is an
explicit namespace so a registry type or package name can never be mistaken for a licence slug.
The version remains part of the address because the resolved tree may contain several versions
of one package, with different metadata or terms.

The directory root completes that hierarchy with a back link to the homepage above its heading,
in the same place each child route links to its parent. Home is navigation rather than a licence
action, so it stays out of the Packages, index and full-notice control row.

One package page is dense where the source metadata is sparse. Its SPDX expression, credited
people and shipped licence files share one compact terms-and-attribution section instead of each
claiming a tall section of their own. A single SPDX term is one link, not plain text followed by
an identical chip; only a compound expression needs separate links to its terms. Dependency paths
have their own section because they answer a different question: why this package is present.

**The sitemap enters the licence directories and stops there.** `/licenses`, `/licenses/pkgs`,
each registry and each licence term are pages somebody could search for -- what is Apache
licensed here, what comes from crates.io -- and there are a few dozen of them. One package page
is a single row of a directory that is already listed, there are several hundred, and entering
them would make the dependency tree the bulk of this site's sitemap. They stay `noindex,
follow`, so a crawler still walks them and the links out of them count. The entries are derived
from the record rather than written down, because the set of licence terms is whatever the tree
currently resolves to.

That directive is emitted once per page, by the root layout, defaulting to `index, follow` and
overridden by a page returning `robots` from its loader. It was a fixed tag in `app.html`, which
meant a page wanting anything else appended a second one and shipped two contradicting
directives -- working only because crawlers resolve a conflict by taking the most restrictive.
A default that can be replaced is not the same as a default that has to be argued with.

The plain-text documents keep their existing addresses: `/licenses.txt`, `/licenses/full.txt`
and `/licenses/{type}/{name}@{version}.txt`. They are stable legal artefacts rather than the HTML
package pages, so reorganising the browser surface is not a reason to move them.

The page is locale-negotiated like every other page, while the three plain-text routes beside
it are prerendered. A licence is not translated, and those routes vary on nothing.

A package resolved for another platform is not in the record at all. A dependency tree carries
an optional binary for every operating system and only one is ever installed; reporting the
rest as declaring nothing would be false, and would bury the handful that genuinely do.

### What happens to an asset after it is stored

Deriving a picture, describing it, drawing its card, slicing a face and serving any of it are
each their own subject and live beside this file:
[media.md](architecture/media.md), [fonts.md](architecture/fonts.md) and
[delivery.md](architecture/delivery.md).

### Three ignore lists, no sharing

| List   | Question it answers        | Lives in                                     |
| ------ | -------------------------- | -------------------------------------------- |
| git    | is this code?              | `.gitignore`                                 |
| sync   | should the world see this? | the source path of `mise run sync`           |
| backup | would losing this hurt?    | whatever backs up `data/`, outside this repo |

They disagree on exactly the content that matters. `data/` is git's least wanted and backup's
most wanted. `data/draft` is worth backing up and must never publish. Build output is unwanted
by all three.

So no list is ever derived from another. Driving backups from `.gitignore` silently drops
every photo; driving sync from the backup list publishes the drafts.
