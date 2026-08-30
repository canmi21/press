# Images and the records that describe them

## Variants stop where the layout does

An image is published at 640, 1280 and 1920 on its long edge, and no further. Nothing on the
site renders wider, so pixels above the cap are weight every reader pays for and nobody sees.
An original below the cap is its own top rung; upscaling is never done.

`cms image --original` adds one more rung at the original resolution for the images where the
detail is the point -- a photograph rather than a screenshot of some text. It is still AVIF
and still lossy, so "original" means the full frame rather than the original file. The choice
is recorded in the manifest rather than inferred, because re-deriving has to reproduce what
was published, and comparing the top variant against the source would guess wrong for every
image that sits below the cap, where the two are the same size for an unrelated reason.

## A description belongs to the image

Alt text is held in the manifest, on the asset, not on the reference. It describes the
picture, and the picture is the same picture wherever it appears -- so one description written
once is inherited by every reference, including the ones written years later. An article that
needs different wording for its own context overrides it; nothing else has to say anything.

`cms alt` fills them by handing the work to a local agent CLI rather than to an API. The
default is `gpt-5.6-terra-medium` through Codex. How each runner is shown the file is
in [i18n.md](../i18n.md). There is no API request to assemble and no key to hold.

The framing in the prompt is the instruction that matters. "Describe this image" produces a
caption -- a label naming the subject. Asking for what someone who cannot see it would need
produces what is actually useful: what kind of image it is, what it contains, and what it is
evidence of. `--limit` exists because each call costs real money, and finding out the prompt
is wrong should be cheap.

## The description is baked in beside the placeholder

The build inlines an image's description the same way it inlines its thumbhash: both belong to
the picture, both come from the manifest, and neither should be repeated in the article that
happens to reference it. An article written before any description existed picks one up on the
next build, without being edited.

Writing `alt` overrides it for one page's context. The two syntaxes differ in what they can
express, and the difference is real: markdown has no way to say "decorative", so `![](x)`
parses to an empty alt meaning unwritten and nothing else. A directive can say it, so
`::image{alt=""}` is a decision and is left alone. A linkcard's cover is decorative by
construction -- the title it illustrates is right beside it.

## A link's name says where it goes; everything else is a description

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

## Where a photograph was taken is worked out offline

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

## A category is closed; a tag is not

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
which concept the identifier denotes. See [i18n.md](../i18n.md) for how those labels are
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

## A card is named by its slug, and that is the exception

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

**Every package page gets its own card.** The dependency tree runs to several hundred packages --
[licenses.json](../../data/build/licenses.json) is the count -- and there are nine views, so this
accepts a card file per package per view, tens of thousands of published bytes each, rather than
collapsing packages into one generic card that does not identify the shared page. The manifest
makes that cost incremental: the full set is paid once, then only a package whose inputs moved is
redrawn.

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

## The title is sized to fit one line

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

## The manifest has versions, and only one is current

`data/metadata.json` and every published record carry a version. Raising it means migrating the file
in place and writing it back, never teaching the reader a second shape -- two readers for two
shapes is how a format stops having a current version at all.

A migration republishes records from the merged manifest rather than re-deriving. The pixels
did not change; only the record did, and spending minutes of AV1 encoding to alter a field
would be paying for an answer already on disk.

## Cropping is presentation, so the browser does it

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

**A link card's cover is cropped the same way, by the same default.** `::linkcard` is a
directive typed on purpose, so the argument above applies to it unchanged: saying nothing about
the ratio reads as "the usual one", not as "leave it alone". It takes `ratio` and `align` too,
and the shared helpers name whichever directive rejected the value.

That this was missing was not a decision, it was a place the rule never reached: the default
lives in the compiler's `::image` branch, while the card only borrowed the *component* -- whose
contract is that an absent crop shows the whole image, because `![]()` depends on it. Covers are
screenshots, so they arrive at whatever shape a window happened to be. The ten in this corpus
ran from 1.52 to 1.96, which is a column of cards at ten heights.

A crop does not reach the feed or the markdown target. Neither runs a layout, and how a page
frames an image is not something the image says.

The scanner reads `::image` for its `src` alone. Missing that would be worse than cosmetic: an
asset referenced only in cropped form would look unreferenced, and the next sweep would delete
it.
