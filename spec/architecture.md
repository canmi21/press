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
data/       Binary assets. In the tree, never in git.
projs/      Reserved for large standalone projects. Not created yet.
```

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

## Workspace wiring

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

The site builds from `assets.json`, `contents/` and `site.config.yaml` -- all committed --
and never reads `data/`. That is what the merged manifest is for: it carries every dimension,
srcset and placeholder, so a page renders correctly with not one image byte present. A
checkout is a complete build input.

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

Every published asset -- an original and each variant derived from it -- is stored under the
hash of its own bytes, BLAKE3 truncated to 128 bits. The identity of a whole asset is its
original's hash; a variant is a separate object with a separate one.

This is what makes long caching safe without a promise to keep. The fonts above are cached for
a year under names carrying no hash, so "these bytes never change" is a rule someone has to
remember. A content-addressed key cannot denote different bytes than it did before, because
changing the bytes changes the key. Re-encoding at a new quality produces a new object rather
than a redefinition of an old one.

The relationships -- which variants belong to which asset, their sizes and formats -- live in
the manifest, not in the key layout. The store answers "give me these bytes"; the manifest
answers "which bytes do I want". Deriving one from the other would mean encoding relationships
into paths, which is how a rename becomes a migration.

The truncation to 128 bits leaves roughly 64-bit collision resistance. That is far beyond what
addressing a lifetime of personal assets requires, and is deliberately not a tamper-evidence
claim. It is also unrelated to an IPFS CID, which is a structured multihash rather than a bare
digest.

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

`cms alt` fills them by handing the work to the local `claude` CLI rather than to the API.
That binary is a whole agent with a Read tool of its own, so naming a path in the prompt is
enough: there is no multimodal request to assemble, no image to encode, and no key to hold. It
is slower and dearer per call, and neither matters for a batch that runs once per imported
picture.

The framing in the prompt is the instruction that matters. "Describe this image" produces a
caption -- a label naming the subject. Asking for what someone who cannot see it would need
produces what is actually useful: what kind of image it is, what it contains, and what it is
evidence of. `--limit` exists because each call costs real money, and finding out the prompt
is wrong should be cheap.

### The manifest has versions, and only one is current

`assets.json` and every published record carry a version. Raising it means migrating the file
in place and writing it back, never teaching the reader a second shape -- two readers for two
shapes is how a format stops having a current version at all.

A migration republishes records from the merged manifest rather than re-deriving. The pixels
did not change; only the record did, and spending minutes of AV1 encoding to alter a field
would be paying for an answer already on disk.

### Cropping is presentation, so the browser does it

`![](cid.ext)` shows the whole image. `::image{src=...}` shows it cropped, defaulting to 16:9
and centred, with `ratio` and `align` to say otherwise. Writing the directive is itself the
request to crop, which is why its defaults are a shape rather than "no change".

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

### A hash in the name buys a year

Cache lifetime follows one rule everywhere: **a name carrying a content hash is cached for a
year and marked `immutable`, but only on a 2xx. Anything else is cached for five minutes.**

The year is an observation, not a promise. Changing the bytes changes the hash and therefore
the URL, so a hashed name cannot come to mean anything else and nobody has to remember to bust
it. `/fonts/` is kept for a year too and is the exception that shows the difference: those
filenames carry no hash, so re-subsetting a font has to produce a new filename or every reader
holds the old one for a year.

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

The trap inside the old policy is worth keeping in view. Those font filenames carry no content
hash: `IoskeleyMono-Regular-latin.woff2` is a stable name. Declaring it `immutable` for a year
promises that the bytes at that name never change, so re-subsetting the font requires a new
filename. Whatever replaces `_headers` inherits that promise, or breaks it for everyone
holding a cached copy.

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
