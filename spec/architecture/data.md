# Where bytes and records live

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

## What `data/` keeps out of git

Not the directory -- the kind of thing. **Text that means something goes in; bytes and bulk
stay out.** `data/metadata.json`, `data/media.yaml` and `data/tags.yaml` are records: a build
resolves every image from the first without one image being present, and the other two hold
descriptions that cost money and tags a person curates. Photographs, derived variants, fonts
and a geocoding database are bytes, and no diff of them says anything.

### Generated build inputs live under `data/build/`

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

## Publication is a path, not a rule

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

## Assets are prepared locally, never in CI

A local build fetches whatever it is missing -- remote favicons, image variants -- and writes
it into `data/public` for the next sync. A CI build does neither; it compiles what is already
there.

The split exists because CI has no writable source of truth. If it fetched, the result would
live only in the deployed artifact, and `data/` would no longer be complete. A reference whose
asset has not been synced yet degrades to a placeholder at request time rather than failing
the build -- one missing image is not a reason to block a release.

## Articles decide which assets exist

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

## Missing assets are reported, never fatal

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

## A CI build must be able to build from git alone

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

## Assets are addressed by their content

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

## A dependency's licence is an asset like any other

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

## The bytes belong to the machine, and one checkout holds them

`data/` is machine-level, not checkout-level. It is the local truth R2 mirrors and the thing a
backup is taken of, and a machine has one of it: the base workspace's -- see
[toolchain.md](../toolchain.md), "Parallel workspaces". An overlay workspace holds no bytes
of its own. `mise run workspace add` links every path under `data/` that git does not carry
back into the base, entry by entry where a directory mixes tracked records with untracked
bytes, so a checkout of records plus links reads exactly like the base. Deleting an overlay
loses nothing, and the backup story does not change because a second directory appeared.

**An overlay reads `data/`; it never writes it.** Writing is the CMS's and the sync task's,
and both run from the base -- the CMS because it is a machine-wide singleton on its pinned
port, the sync because a mirror with two sources is not a mirror. An overlay that changes CDN
or CMS code still sees the real bytes through the links, which is what makes the change
testable there; producing new bytes is a base job.

Records are the one place the two workspaces can disagree, and it is safe in the direction it
happens: an overlay that adds an image writes its record and, through the link, its bytes into
the shared tree, so another workspace briefly holds bytes it has no record for -- unused, and
harmless. The reverse, a record without bytes, is what a per-checkout copy would produce and
the links prevent.

Pointing every reader at a `DATA_ROOT` environment variable instead of linking was
considered and left. It would move every `data/` path in two languages for a benefit that only
appears with a second workspace, and the links deliver the same sharing with no code touched.
It stays the option to take if the link step ever proves brittle.

## What happens to an asset after it is stored

Deriving a picture, describing it, drawing its card, slicing a face and serving any of it are
each their own subject and live beside this file:
[media.md](media.md), [fonts.md](fonts.md) and
[delivery.md](delivery.md).

## Three ignore lists, no sharing

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
