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
file whenever a consumer needs one without that growth being a question. Committed all the
same, for the reason the segment layout exists at all: a site-only CI build must not need a
Rust toolchain to produce its own inputs.

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
the site's tokens and local Tailwind classes continue to own every visible decision. Importing
a styled component kit on top would create a second design system, so project primitives under
`apps/site/src/lib/components/` expose the small set of surfaces the site actually repeats.

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

The shared visual language extends beyond the palette. The CMS uses the site's quiet text
hierarchy, generous content spacing, hairline borders, paper only for contained surfaces and
restrained line icons. Task pages do not acquire branded tiles or ornamental status chrome merely
because the CMS is an operations tool. Overview is deliberately the exception: its job is to make
workspace state legible at a glance, so it is a real operational dashboard with compact metric
surfaces and D3 charts. Those charts encode relationships the live snapshot actually carries, such
as content distribution and resource readiness; they do not invent scores or decorative trends to
fill the grid. This keeps the dashboard density useful without giving the rest of the application a
generic admin-product aesthetic.

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
[toolchain.md](toolchain.md).

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

Git holds code. `data/` holds everything else -- photos, fetched favicons, drafts -- and none
of it is ever committed. Only the empty skeleton is tracked so a fresh clone has somewhere to
put things.

```
data/
  public/   mirrored to R2, 1:1 with the bucket layout
  draft/    never leaves this machine
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

### Variants stop where the layout does

An image is published at 640, 1280 and 1920 on its long edge, and no further. Nothing on the
site renders wider, so pixels above the cap are weight every reader pays for and nobody sees.
An original below the cap is its own top rung; upscaling is never done.

`cms image --original` adds one more rung at the original resolution for the images where the
detail is the point -- a photograph rather than a screenshot of some text. It is still AVIF
and still lossy, so "original" means the full frame rather than the original file. The choice
is recorded in the manifest rather than inferred, because re-deriving has to reproduce what
was published, and comparing the top variant against the source would guess wrong for every
image that sits below the cap, where the two are the same size for an unrelated reason.

### A description belongs to the image

Alt text is held in the manifest, on the asset, not on the reference. It describes the
picture, and the picture is the same picture wherever it appears -- so one description written
once is inherited by every reference, including the ones written years later. An article that
needs different wording for its own context overrides it; nothing else has to say anything.

`cms alt` fills them by handing the work to a local agent CLI rather than to an API. The
default is `gpt-5.6-terra-medium` through Codex. How each runner is shown the file is
in [i18n.md](i18n.md). There is no API request to assemble and no key to hold.

The framing in the prompt is the instruction that matters. "Describe this image" produces a
caption -- a label naming the subject. Asking for what someone who cannot see it would need
produces what is actually useful: what kind of image it is, what it contains, and what it is
evidence of. `--limit` exists because each call costs real money, and finding out the prompt
is wrong should be cheap.

### The description is baked in beside the placeholder

The build inlines an image's description the same way it inlines its thumbhash: both belong to
the picture, both come from the manifest, and neither should be repeated in the article that
happens to reference it. An article written before any description existed picks one up on the
next build, without being edited.

Writing `alt` overrides it for one page's context. The two syntaxes differ in what they can
express, and the difference is real: markdown has no way to say "decorative", so `![](x)`
parses to an empty alt meaning unwritten and nothing else. A directive can say it, so
`::image{alt=""}` is a decision and is left alone. A linkcard's cover is decorative by
construction -- the title it illustrates is right beside it.

### A link's name says where it goes; everything else is a description

Everything inside an anchor becomes part of the link's accessible name, so what goes there is
a budget rather than a place to be thorough. A linkcard's cover keeps `alt=""` even though a
description exists for it: an 800-character alt would make the link announce the whole
screenshot before saying its destination, and a reader tabbing through links would sit through
that every time.

The description is offered through `aria-describedby` instead, from an element outside the
anchor. A screen reader announces it after the name and lets the reader skip it, so the
content is available without being in the way. Inside the anchor it would join the very name
it is meant to follow.

The name itself has to carry what the visuals carry. A card's title never said which site it
led to -- the favicon did, and that is `aria-hidden` -- so the domain is added there, along
with the new-tab warning that `:link` directives already emit. That last one was an
inconsistency rather than a new decision.

### Where a photograph was taken is worked out offline

`cms image` reads EXIF once at import, because the original may not be on hand later and the
published variants carry none of it -- a reader downloads pixels and nothing else. Nothing in
that block is trusted about the _file_: EXIF describes what the sensor did, and one sample
reports 4032x3024 for a frame that is 4032x2268 on disk. Dimensions and ratio come from
decoding. Orientation is the exception and must be read, or every derived image comes out
turned.

The address is the one part not in the file. It is looked up from the coordinates against
GeoNames' `cities500` in `data/geo`, indexed into an R-tree, with the timezone from the
polygon the point actually falls in rather than from the nearest town. Offline deliberately: a
geocoding service would make importing a photograph depend on somebody else's uptime, rate
limit and terms, for a fact that never changes once written.

The county comes from the admin2 code the settlement already carries, and the postal code from
a second index of GeoNames' postal points -- found by position and then checked against the
country, because a code is not unique on its own: 27707 is a district of Eumseong and also a
part of Durham.

`district` stays absent. Naming a neighbourhood needs the full GeoNames dump, an order of
magnitude larger than everything else here put together, and deriving one from the nearest
town would state something no source claimed.

Building the postal index costs about fourteen seconds for 1.8 million points. It happens once
per run, which a batch import absorbs and a single import does not, and that is the trade for
never asking anyone.

HEIC decodes through a pure-Rust decoder rather than bindings to libheif. It is HEVC inside a
HEIF container -- the same container AVIF uses, with a different codec, so support for one
says nothing about the other. A system library would be a thing to install on this machine and
again in CI; 249ms for a 4032x2268 frame is nothing against the AV1 encode that follows. Only
the primary image is taken. A phone's HEIC may also hold a depth map, a gain map and the
frames of a live photo, and none of those are wanted yet.

### A category is closed; a tag is not

Five categories -- photograph, screenshot, diagram, document, artwork -- and a distinction that
can be drawn with a tag never earns a sixth. A terminal capture, a browser capture and an
editor capture are all screenshots, and letting each become a kind of its own would grow that
list once for every application that exists. The category says what sort of thing it is; tags
say what is in it.

Tags are raw identifiers: lower case, digits, hyphens. Each identifies one concept without
needing an image to explain it, so an ambiguous word gets qualified -- `cellular-network`, not
`cellular`; `mold-linker` names the product while `mold` names fungal growth. The constraint is
the point -- `TypeScript`, `typescript` and `type-script` would otherwise be three tags for one
thing, and one `mold` for two things would make a correct translation impossible.

What a reader sees lives in `data/tags.yaml`. A technical name has one official display form
and is never translated. An ordinary name records a disambiguated English source label, a
short semantic meaning, and its translated display forms. Its `en-US` form comes from the same
vision answer that creates the tag and retains that answer's provenance. The meaning is not
copy: it is the stable contract that lets both the tagging model and the translator decide
which concept the identifier denotes. See [i18n.md](i18n.md) for how those labels are
translated and cased.

Both answers come from one request, because they are one look at one picture. Asking
separately would pay twice for the same glance and let the two disagree; a `screenshot` tagged
`landscape` is a contradiction only a second request can produce.

The existing tags go into the prompt with their kind, label and meaning, and images are
classified one at a time rather than in parallel. A raw list cannot say whether `mold` is a
linker or fungus, while a model shown nothing invents `terminal-window` beside `terminal` and
`cli` beside both. Four running at once would each name the same thing before any could see the
others.

A malformed tag is dropped, never repaired. Turning `shell terminal` into `shell-terminal`
invents a name nobody chose, which then competes with `terminal` forever.

### A card is named by its slug, and that is the exception

Every other published asset is named by a hash of its bytes. An OpenGraph card is not:
`cms og` writes `opengraph/{slug}.png`, mirroring the article tree, and the page emits that
URL from its own route. Nothing stores a reference, so there is nothing to rewrite and no id
to look up -- the address follows from where the article sits.

The cost is that the name is mutable, and the cache rule already prices it: no hash means a
week rather than a year, which is also what X caches a card for. An edited title takes that
long to circulate, and that is the accepted trade rather than an oversight.

**A card is rendered once per view, and asked for by `?lang=`.** A page served in Japanese that
advertises a Chinese card is telling a reader one thing and a crawler another, so the nine
views each get their own card, with the title and subtitle the sidecar already holds for that
locale. The address is the page's own slug plus the same `?lang=` that selects the page --
`/opengraph/{slug}.png?lang=ja` -- while the bytes are stored under `opengraph/{view}/{slug}.png`.
That is the key-is-not-the-URL rule again: one parameter names what the reader wants and the
worker decides where to read from.

**The home card is a different card, not an article card with the site's name in it.** An
article card answers "what does this page say"; the home card answers "whose site is this", so
it keeps the same three bands and puts a different thing in each: the site name at the top, the
portrait with the author's name and role in the middle, and what there is to read in the
bottom-right -- the corner an article card uses for its date and category, chosen there because
the bottom-left belongs to X. Two cards that share a grammar read as one site; two that share a
template read as one card with a field swapped.

The name and role come from `site.config.yaml`, which is where the page reads them from, and
are not translated: a name is a name, and the job title is one of the things this site leaves
in English. The counts beside them are worded by the same message catalogs the pages use, so a
card and a page never phrase the same fact differently, and they are **characters rather than
words** -- a word is not a unit CJK has, and one number that means different things depending
on which article it came from is worse than no number. The source text is counted for every
view, because the number describes the site rather than the translation being read.

The address is drawn opposite the site name across the top, because the other free corner is
the bottom-left and that one belongs to X. It lives in `site.config.yaml` rather than in
`libs/urls`, and the distinction is real: what is drawn there is a label a person reads off a
picture, not an address anything resolves. The exemption only holds while the two agree, so a
test compares it against the host `libs/urls` declares -- nothing structural can, since one is
read by Rust and the other by the bundler.

The portrait is fetched into `data/` once, like the font and for the same reason: it is bytes
somebody else serves, and a local command should not need the network to draw a card. A clone
without it still renders every card, with that one lacking a portrait rather than the command
refusing to run.

**A card is redrawn when its inputs move, not when its file is missing.** `cms og` records a
hash of everything each card was drawn from in `data/build/opengraph.json` and redraws the ones
whose hash has changed. The older test -- skip anything already on disk -- was always slightly
wrong, since an edited title left the previous card in place until somebody remembered
`--force`; it stopped being defensible once a card started carrying a read count, which changes
without anything in the repository changing at all. The record holds hashes and not the values
behind them, so it stays small and nobody is tempted to read an article out of a build artifact.

The article card's bottom band is two lines: its date and category on the upper one, and what
else the article is available as on the lower. A read count was tried there and dropped -- it is
the one fact on a card that changes while nothing in the repository does, and a figure that can
only be as fresh as the last time somebody ran a local command is a number the card would be
wrong about most of the time. What is left is a badge -- `+8 languages`, and the same
shape in each of the nine -- saying how many other languages this article exists in. `+` carries
"and this many more" without a word for it, which is why the line stays short enough to read at
thumbnail size and identical in form across scripts that share no vocabulary.

**The licence routes get cards too, drawn by the article template.** A title, a line under it
and two lines in the corner is already the shape those pages need, so a fourth layout would be
a second thing to keep in step for no difference a reader could see. What varies is what fills
the slots, and that is a template's job.

Their facts come from the licence record and their words from the message catalogs the pages
read, because copy written into the generator would give the site a second voice -- one that
agrees with the first only until somebody edits one of them. What a card says is deliberately
**not** the page's meta description: that is written for a search result, where nothing sits
above it, so it repeats the name the card already shows in large type directly above the line.
The same distinction the article card makes between `subtitle` and `description`.

A registry card carries no count badge. The only thing left to say in that corner is how many
other registries exist, which is one, and the catalogs have no plural machinery to say it with;
a badge that reads `+1 registries` is worse than an empty corner. Where a count does fit, it is
the `+N` form the article cards use -- `+24 licenses` -- which says "and this many more" without
a word for it and keeps its shape across scripts that share no vocabulary.

**Every package page gets its own card.** There are 727 packages and nine views, so this accepts
6,543 files and roughly 445 MB of published bytes rather than collapsing packages into one
generic card that does not identify the shared page. The manifest makes that cost incremental:
the full set is paid once, then only a package whose inputs moved is redrawn.

Package facts stay literal in every view: its name and description, version, registry display
name and SPDX expression identify the same release whatever language surrounds them. The
description occupies the subtitle rather than borrowing the page's meta description, which
would repeat the package name already drawn above it. An absent description leaves that line
empty, and an absent declared licence leaves an empty badge; inventing English copy for either
would turn missing package metadata into a translated claim the package never made.

A view with no card falls back to the source view rather than to a 404. Translation arrives
per segment and per article, so a missing card is a normal intermediate state; a card in the
wrong language still says what the page is, and a blank rectangle says nothing. An unknown
`?lang=` collapses the same way, which is also what keeps the code from reaching the bucket as
an arbitrary prefix.

PNG, not AVIF, against the rule that says store the newest format. The consumers here are
crawlers for X, Slack and Discord, and they do not read AVIF. When the format a thing is
stored in is decided by software nobody here controls, the rule bends to the reader.

The CDN's `robots.txt` forbids nothing, and two failed attempts is why. `Disallow: /` was
right in principle -- a CDN has nothing worth indexing, and its URLs in results compete with
the pages embedding them -- but crawlers read that file before fetching an `og:image`, so it
hid the one thing a page advertises. Carving out `Allow: /opengraph/` did not help either:
Twitterbot implements the original 1994 draft of the format, which has no `Allow` directive at
all, so it reads the disallow and never sees the exception.

A per-agent block would work and would mean tracking which crawler parses which decade of the
format, forever. What was being protected was mild and the bandwidth is not ours to ration, so
the policy stops being clever. An empty `Disallow:` is the one spelling every parser agrees
means "all of it".

### The title is sized to fit one line

A card's title is shaped at 96px and stepped down until it occupies a single line, stopping at
56px and wrapping below that. Measured, never estimated: where a CJK title breaks has no
relation to its character count, so the only way to know whether a size fits is to lay it out
and look. The same measurement decides where the band below it starts.

A subtitle begins at 38px and stays there when it fits. Package descriptions are authored by
hundreds of upstream projects and can run much longer than an article subtitle, so a long one
steps down to 20px to keep the complete description between the header and the bottom metadata.
Clipping would make the package's own sentence incomplete; allowing it to overlap would make
the version and licence unreadable. This is measured from shaped lines for the same reason as
the title rather than guessed from character count.

The bottom band is aligned right because X draws the domain over the bottom left of every card
it renders. Anything placed there is covered by somebody else's chrome.

### The manifest has versions, and only one is current

`data/metadata.json` and every published record carry a version. Raising it means migrating the file
in place and writing it back, never teaching the reader a second shape -- two readers for two
shapes is how a format stops having a current version at all.

A migration republishes records from the merged manifest rather than re-deriving. The pixels
did not change; only the record did, and spending minutes of AV1 encoding to alter a field
would be paying for an answer already on disk.

### Cropping is presentation, so the browser does it

`::image{src=...}` is how an article names one of its own images. It crops to 16:9, centred,
with `ratio` and `align` to say otherwise. Markdown's `![]()` is left to external images,
which have no manifest entry and nothing to inherit.

One syntax, because a default only holds where writing the thing is itself a decision.
`![]()` is what a hand reaches for without choosing, so giving it an opinionated crop would
have been deciding for the author; a directive has to be typed on purpose, and there the
default reads as "you said nothing else, so this". The first version of this rule kept both
and made the directive opt-in, which optimised for not editing existing articles and paid for
it with two permanent code paths and two rules for alt text -- markdown cannot express
"decorative" at all, so `alt=""` meant one thing in a directive and another in an image.

It is done with `aspect-ratio` and `object-fit`, never by storing another object. A variant per
ratio and alignment would multiply the bucket, and would make a content id mean "this image as
shown here" instead of "this image" -- which would take the addressing model with it, because
`cms gc` reaches assets through the ids articles name. The cost is that the hidden part of the
image is still downloaded; that is the cheaper of the two.

A crop does not reach the feed or the markdown target. Neither runs a layout, and how a page
frames an image is not something the image says.

The scanner reads `::image` for its `src` alone. Missing that would be worse than cosmetic: an
asset referenced only in cropped form would look unreferenced, and the next sweep would delete
it.

### A font pipeline input is disposable

The font pipeline only moves in one direction: a full face under `data/fonts` is input, and web
chunks under `data/public/fonts/{family}` are output. The input is useful only while somebody
may slice that face again. Once the chunks exist it may be deleted, and a family with prebuilt
chunks needs no input of its own. It may still name an input retained by a different family that
owns their shared chunks. Keeping every original forever would turn a temporary build need into
repository policy without buying the browser anything.

The authored [font manifest](../data/fonts.json) records that distinction. Ioskeley Mono has
eight prebuilt chunks and no retained input; that is a complete family, not a missing source.

That manifest stays in `data/` because it is a curated asset record and the slicing pipeline must
read it without depending on a TypeScript runtime. Consumers do not cross that directory boundary:
the root export of [`@canmi/fonts`](../libs/fonts/package.json) provides the typed family list and
its package stylesheet paths. This keeps one authored list while making the library the public
answer to what a family is and where its stylesheet lives.

### A runtime font is a separate dependency

An application may independently need a full face at runtime. `cms og` renders arbitrary titles
with LXGW WenKai and therefore loads that one full TTF by path; a web subset cannot answer for a
character it does not contain. That runtime dependency is why the 24MB file stays. No other
published family gets a retained full face merely because this one has two roles.

### Latin and CJK use different slicing strategies

Latin faces are a few hundred kilobytes and are split into the handful of named writing-system
subsets Google Fonts uses -- `latin`, `latin-ext`, and the other groups a face publishes. Their
readable filenames are stable cache interfaces. CJK faces are tens of megabytes, so they are
split by character frequency into hundreds of `unicode-range` chunks: common characters arrive
first, and content hashes name the output because no person benefits from reading those names.

The strategy is explicit in the manifest rather than inferred from glyph coverage. Coverage says
what a face contains, but not whether its existing readable URLs are a compatibility promise;
inferring would let a font update silently change both its publication layout and cache identity.
See the [font runbook](../libs/fonts/README.md) for the operational side.

A selectable family is the name a person picks, not a set of bytes. Its generic fallback completes
the CSS stack, and its faces say which local or redistributable typefaces may satisfy that choice.
Metric compatibility decides what may substitute; it does not decide which choices are offered.
Two families therefore remain separate entries when their local-first stacks differ, even if they
share the same published chunks. Keeping the choice and its sources together prevents a second
selectable-font list from disagreeing with the published faces.

The stylesheets live in `libs/fonts`, apart from the colour tokens. They are a different kind
of fact -- what a family is and where its files are, rather than what the site looks like --
and the CJK sheet alone is 75KB gzipped, which nothing should import until the site actually
sets that family.

### A hash in the name buys a year

Cache lifetime follows one rule everywhere: **a name carrying a content hash is cached for a
year and marked `immutable`, but only on a 2xx. Anything else is cached for five minutes.**

HTML is the one thing that is not cached at all, because its body varies by the reader's
locale cookie. See [locale.md](locale.md).

The year is an observation, not a promise. Changing the bytes changes the hash and therefore
the URL, so a hashed name cannot come to mean anything else and nobody has to remember to bust
it. CJK font chunks work that way too. Latin subset names are the deliberate exception: a name
such as `IoskeleyMono-Regular-latin.woff2` is readable and stable, so re-subsetting must publish
a new filename or every reader keeps the old bytes for a year.

Errors get five minutes rather than nothing. A missing favicon is requested on every page
view, and without any caching each one is a full trip to the origin. Five rather than a year
because an error is a statement about right now -- the asset it refers to may be published a
minute later, and a year-long 404 would outlive its own reason.

A route that stores its own response has to stamp the header before storing it, which is
earlier than the middleware runs. So the value is one exported constant that both use, rather
than two spellings that agree until they do not.

### Formats are produced here, not at the edge

Cloudflare's image transformations cannot read AVIF below an Enterprise plan, and even there
the source is capped at 1200px while these variants go to 1920. The format chosen for storage
is the one format that pipeline cannot open. Measured: an AVIF source returns
`ERROR 9520: Original image has unsupported format` where the identical request against a PNG
source succeeds.

So the CDN decodes and re-encodes in the worker, using WASM codecs. That removes the plan
tier, the monthly quota and the dimension ceiling together, and the cost is bounded because
the extension is the entire request -- there is no size parameter to vary, so a caller cannot
invent work. Results are held in the edge cache, so the decode is paid once per colo rather
than once per reader.

Only the decoders for what is stored and the encoders for what is asked for. The AVIF
_encoder_ is deliberately absent: 1.1MB compressed against 332KB for the decoder, and
`cms image` already produces AVIF locally where the time costs nothing.

### The extension asks for a format

Only AVIF is stored. `/image/{cid}.avif` is served straight from the bucket; any other
extension is a request to convert that same object, which the worker satisfies through
Cloudflare's image transformations.

Cloudflare counts a conversion once per image regardless of how many formats it ends up
serving, so the whole fallback chain costs one transformation rather than a second and third
copy of the library. Storage would be nearly free either way -- what a stored fallback really
costs is the sync, the derive time, and a second thing to keep consistent.

No `?format=` parameter, because the extension already says which format is wanted and two
spellings of one request fragment the cache key. It also caps the exposure: only a size that
was derived exists as an object, so nobody can burn the monthly transformation quota by
asking for arbitrary dimensions.

The failure mode to remember is that exceeding the quota does not degrade -- new conversions
return an error while already-cached ones keep serving. That is why the request path a browser
takes by default is the stored AVIF, and conversion is only ever the fallback.

### Caching is the worker's job now

The old CDN served these files through a static-assets binding and set their cache policy in
a `_headers` file: `/fonts/*` for one year, `immutable`. That file has no equivalent once a
worker reads from R2, so the policy has to be reasserted in worker code or it is silently lost
-- the assets keep working while being re-fetched on every visit.

The trap inside the old policy is worth keeping in view. Latin font filenames carry no content
hash: `IoskeleyMono-Regular-latin.woff2` is a stable name. Declaring it `immutable` for a year
promises that the bytes at that name never change, so re-subsetting the font requires a new
filename. CJK chunks already carry content hashes and need no such promise. Whatever replaces
`_headers` has to preserve both cases rather than pretending all font names have one shape.

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
