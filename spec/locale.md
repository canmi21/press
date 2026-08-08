# Locale selection

How a reader ends up with one of the nine views of an article. What produces those views is a
separate problem; see [i18n.md](i18n.md).

## One domain, one path, no locale in the URL

Everything is served from `canmi.net`. A locale never appears as a path segment and never as a
subdomain: `/rust-cargo-cranelift-tuning` is that article in every language, and which one a
given reader sees is decided per request.

The alternative shapes were considered and rejected for the same reason. A path prefix or a
subdomain makes the language part of a page's identity, which means every already-published
link belongs to one language forever and adding a language multiplies the URL space. Here a
link is a link to the article, and the reader's own preference decides the rest.

The cost is real and accepted: **a shared URL carries no language.** Sending someone a link
shows them the article in their language, not in yours.

## Browser-facing HTML has three preference sources

An HTML request resolves its locale from the first of these that answers:

1. A `lang` query parameter.
2. The locale cookie.
3. `Accept-Language`.
4. Failing all three, the article's own language.

The last step is what keeps the bare URL honest. It is the `x-default`, so what it serves to a
reader nothing is known about has to be the original; anything else would make `x-default` name
a translation. Paraglide's message fallback is the original as well, for an unrelated reason --
see below, and do not read either as implying the other.

The query parameter wins outright and **also writes the cookie**, so following a language URL
once is choosing it from then on rather than making a one-off override. The browser's language
controls use the other entrance to the same state: they write the closed locale code into that
cookie and reload the current document. The server still performs the same negotiation on the
new request; the client chooses an input, never the rendered view.

Selection runs in the worker on the request, before any HTML is rendered. Every input already
arrives there, and choosing after first paint would make a page render in one language and then
swap. Unlike theme, the content itself differs, so this cannot be a class toggle — see the
caching rule below for what that forces.

Browser-facing collection surfaces use the same resolved code for every article they include.
Homepage cards and article lists take their title, subtitle, description, body, and language
from that article's resolved view; `mw` means each article's own original. The homepage bio is
identity copy and stays in its English source form in every view. Collection UI such as the
Writing heading resolves from the same code through the UI message table. `llms.txt` keeps its
existing behaviour and has no indexed language dimension: an LLM can read any of the views. It
documents the eight translation query codes and `mw` so a machine can ask HTML or Atom for a
specific view, while every `.md` endpoint continues to return the source exactly as written.

Server-only discovery routes do not inherit this negotiation. The sitemap publishes every
indexable view at once. Atom selects from `lang` alone, because each query-specific feed is a
shared-cache resource and neither a cookie nor `Accept-Language` is part of its address.

### Every page negotiates; the exceptions are documents

Being multilingual is what a page here is, so nothing has to opt in. What
[hooks.server.ts](../apps/site/src/hooks.server.ts) carries is the exception, and the exception
is documents: Atom, the sitemap, `robots.txt`, `llms.txt` and the licence text routes.

**A document is recognised by having an extension, and no page has one.** That is a convention
this site already keeps -- `/atom.xml` and `/licenses/full.txt` against `/` and `/licenses` --
so the test reads the distinction that exists rather than restating a list beside it. The
article lookup is tried first, so a slug that happens to carry a dot is still a page.

Stated the other way round it would be a list of pages, and the two failures are not
comparable. A page missing from a list of pages serves the original to every reader and says
nothing about it, which is exactly the kind of thing nobody notices; a document missing from
this list merely negotiates when it did not need to.

The licence page shows both halves at once. The page negotiates like any other, while
`/licenses.txt`, `/licenses/full.txt` and the per-package routes beside it do not: a licence is
not translated, and a translated one would be a different licence.

### Server-only documents leave the page router

Links from an HTML page to Atom or the sitemap perform a full document navigation. They are
server endpoints, not pages in the client route manifest; allowing the SPA router to intercept
one produces its own 404 even though a direct request to the same URL succeeds. The boundary
belongs on the [source link](../apps/site/src/routes/+page.svelte). Making the endpoint reload
itself would first render the wrong route and would require client JavaScript in a document
that never needed it.

## The query parameter is for crawlers, and is removed for readers

A crawler has no cookie and does not run a language switcher, so without a URL that names a
language it can only ever see one of the nine views. **The default URL stays bare and every
other language is reachable at `?lang={code}`.** That is the only reason the parameter exists.

Once the page has loaded, the parameter is removed with `history.replaceState`. The reader
keeps a clean URL, the cookie already holds the choice, and nothing about the page depends on
the parameter still being there. Interactive selection creates no query at all: it writes the
cookie and calls `location.reload()` on the clean address. A reload preserves the current
history entry, so switching language cannot leave an extra, visually identical entry behind
the reader.

The preference cookie is deliberately client-writable. It contains only one value from the
closed locale-code set, and the worker validates it again before use. The server rewrites it on
every locale-aware HTML response, even when its value is unchanged; besides refreshing the
lifetime, this migrates an older `HttpOnly` cookie that client controls could not replace.

Only `lang` is ever touched. Other query parameters are left exactly as they arrived, including
their order, because they belong to whatever put them there. `URLSearchParams` is built into
every browser this site supports; **a query parameter is not a reason to ship a library**, and
the post-load cleanup is the last place to add one.

## Two vocabularies, and only one of them is public

The codes in `?lang=` are internal: `mw`, `de`, `en`, `es`, `fr`, `ja`, `ko`, `zh`, `tw`. They
are short because they are ours, they appear only in a URL we generate and read, and no
consumer outside this repository is expected to interpret them.

Everything a machine other than ours reads gets full BCP-47: `<html lang>`, `hreflang`,
`og:locale`. **The two must never be confused, because two of the internal codes are not
language tags at all** — `tw` is Twi, a language of Ghana, and `mw` is a country code. Emitting
either as an `hreflang` value does not merely mislabel one link; an invalid value makes a
crawler discard the whole set.

So there is exactly one mapping table, in one place, and every public attribute goes through
it: `tw` to `zh-TW`, `zh` to `zh-CN`, and so on.

## `mw` is the article's language, not a language

`mw` means _the original_, and which language that is depends on the article. Today every
article is Chinese; the next one may not be.

So `<html lang>` and `og:locale` for the original view come from the article's own `lang`
frontmatter, never from the code. A code that resolves to a different tag per article is fine
as an internal name and fatal as a public one, which is the same rule as above arriving from a
different direction.

That frontmatter value is validated as a BCP-47-shaped language tag when the source article is
read. A malformed value fails the build once, with the source file named; it must never travel
unvalidated into every original-view public attribute.

## Canonical points at the version being read

Each view starts with its own canonical: the original at the bare URL, every other language at
its `?lang=` form. Ranking weight then accumulates where the content actually is instead of
being handed to a version the reader never asked for, subject to the same-language deferral
below.

The `hreflang` set accompanies every view: the eight locales, plus `x-default` pointing at the
bare URL. Canonical alone says "this is the address of this page"; it does not say "these pages
are translations of one another", and without that a crawler is left to guess from content it
has already decided is similar.

**`mw` is the `x-default` and never an `hreflang` value of its own.** Its tag would have to come
from the article's frontmatter, and that tag always duplicates whichever locale matches it —
`zh` beside `zh-CN` for a Chinese article, `en` beside `en-US` for an English one. Worse, it is
the honest position: an article that deliberately mixes languages has no single one to claim,
and `x-default` already means the version to serve when nothing else matches.

The original stays reachable by `?lang=mw` and through the language switcher. It stops making a
claim about its language; it does not stop being served.

## The article switcher names languages for their own readers

The article metadata row carries the content-language switcher; it is separate from UI-message
translation. Its trigger shows a globe on `mw`, a languages icon on a translation, and the
current view's name in that language's own form.

### The original view labels itself in English

`mw` is a locale like the other eight and its messages are its own, but the words _around_ the
article are English there rather than the article's language. The original is where an author
mixes languages freely; an interface that tried to follow that has nothing to follow. English is
the one choice that does not claim the chrome belongs to whichever language happens to dominate
the prose.

That decides the row beside it too. `mw` is not in the compact-script set, because the set is
about what the label is written in and the label now reads `Original` -- so the language beside
it is a region code, `Original (CN)`, exactly as it is for every other Latin-script view.

The eight translation rows are fixed endonyms and do not change with the active view. The
original is a separate first row: its name is derived from the article's own language tag and
is marked `Original`, because `mw` names authorship rather than a language. Consequently an
original Chinese view and the Simplified Chinese translation may share a displayed language
name while remaining visibly distinct rows.

Selection compares internal codes. Equal codes close the menu; different codes write the
preference cookie and reload the document. Comparing public language tags would make a
same-language translation unreachable, while client routing would leave the worker's resolved
document language behind the content being shown. The query URLs remain crawler addresses and
the no-JavaScript fallback, not the interactive switching transport.

The trigger exposes its expanded state, the selected row is announced, and the menu supports
native activation plus arrow, Home, End, Escape, and Tab keyboard behaviour. Endonyms help a
reader find the right row without first understanding the current content language; keyboard
and screen-reader access are part of that same requirement.

## UI messages come from Paraglide, which negotiates nothing

Interface strings compile through Paraglide JS. Article content does not -- that is a separate
pipeline, see [i18n.md](i18n.md).

Paraglide is a consumer here, never a decider. Its strategy array holds one entry,
`custom-negotiated`, with no built-in strategy behind it, and that strategy reads back the code
the worker already resolved. The reason is the fourth input above: an article's own language is
content-dependent and no library strategy can see it, so approximating the chain with `cookie`
plus `preferredLanguage` would leave two negotiations to disagree in exactly the cases that
matter. The `url` strategy is absent and there is no `reroute` hook, because a locale never
appears in a path here and there is nothing to delocalize.

The client reads that code from `<html data-locale>`, stamped by the server the same way the
theme class is. The preference cookie is readable but remains only one input: a query may have
overridden it, and the article supplies the final fallback. Reading the server's resolved answer
avoids duplicating negotiation and guarantees hydration describes the view that was rendered.

### Two fallbacks that are not the same fallback

`baseLocale` is `mw`, matching the negotiation default above, and the match is a coincidence of
answers rather than one rule stated twice.

Negotiation picks **which view** a reader is given, and lands on the original because the bare
URL is the `x-default`. `baseLocale` supplies **a string the chosen view is missing**, and lands
on the original because that is the one text always written and so the only one that can always
answer -- a reader on Korean who meets an untranslated key sees the original's wording rather
than a blank.

Two questions, two arguments, one answer. Moving either because the other moved would be
changing a decision that was never made.

### Locale identifiers stay internal

Paraglide is configured with the internal codes, `mw` and `tw` included. It never emits them
into a public attribute -- with `url` off they reach no address, and `<html lang>`, `hreflang`
and `og:locale` still go through the mapping table above. `mw` is a locale like any other here,
not a placeholder: it is the author's own voice, mixed as they please, and the interface is part
of what is being read in the original.

### Keys are dotted, and read off the namespace

Message keys are `flat.dot.key`. Paraglide exports each one under its literal string
(`export { notice_polished as "notice.polished" }`), so a key is always read as
`m['notice.polished']` from a namespace import.

The obvious alternative does not work. `import { 'notice.polished' as noticePolished }` is valid
ES2022, type-checks, and survives the production build -- and is `undefined` at runtime under
the dev server's transform, which turns every article page into a 500 that no check catches
because the tests and the build both pass. It was tried; the namespace is not a preference.

The cost of dots is therefore paid in the linter: `import/namespace` rejects computed access and
is turned off, on the grounds that the compiler already performs that check and names the
offending key when it fails. See `.oxlintrc.json`.

Modules under `src/lib` that tests import use relative paths rather than `$lib`, because the
root vitest run resolves no alias.

### A markup message must exist in every locale

A message containing markup compiles to a `parts()` accessor. A locale missing that message
falls back to a plain string function which has no `parts`, and the generated dispatcher then
reads a property the type does not have -- the runtime guards it, the type check does not.

So `baseLocale` covers a missing _translation_, never a missing _shape_. The script-conversion
notice is written in all nine even though only the two Chinese views can display it: six of
those sentences are unreachable today, which is a smaller cost than a type error in generated
code that nobody can edit.

### Two traps worth writing down

The project directory is `apps/site/.inlang/`. The SDK refuses any path not ending in `.inlang`,
so the entire name is the suffix -- a directory called `inlang` loads fine once its metadata
exists and fails on a fresh clone, which is the worst way for this to be discovered.

The compiler reports success when it has loaded no plugin and found no messages. A wrong
`modules` path or a wrong `pathPattern` prints `✔ Successfully compiled` and emits an empty
index; both are resolved relative to the project directory's _parent_. When messages vanish,
check that first rather than the message files. The plugin is a local dependency rather than the
CDN URL the docs show, which keeps its version in the lockfile and out of `libs/urls`.

## The original view reads in the article's own language throughout

Every view resolves its text at its own locale. `mw` is the exception worth stating, because it
has no locale of its own to resolve at: the summary and every image description it shows are the
ones written in the _article's_ language, not translations of them and not the English the
descriptions happen to be authored in.

The alternative was the accident it replaced. Asset descriptions are generated in `en-US` and
translated outward, so the original view was resolving them at `en-US` by default -- a reader of
a Chinese original heard every picture described in a language the article never used. The
summary would have inherited the same shape.

The rule is one sentence: on `mw`, anything with a per-locale version is taken at the locale the
article's `lang` names. The interface chrome is the deliberate exception above -- that is
English, because chrome belongs to the site rather than to the prose.

## A translated article identifies itself

Every non-original article view places a blue note directly below the metadata row. It says in
the current view's UI language that the reader is seeing a translation and names the source
language in that same UI language. The source-language name is the link back to `mw`; ordinary
activation writes the cookie and reloads like the menu, while its `?lang=mw` address remains a
no-JavaScript and modified-click fallback. The original view has no note, because labelling
untouched content as untranslated would repeat what the language switcher already says.

This notice is a state indicator rather than article content. Keeping it beside the metadata
makes its scope clear before the reader reaches the body, and blue is reserved here for the
translated state rather than becoming a general article accent.

### Two states, because a reader may already speak the article's language

A Chinese article read at `zh` is neither the original nor a translation in the ordinary sense.
The view exists because every language gets one, and what it holds is the article regularised --
a misspelling corrected, a mark normalised -- not carried across a language boundary. Telling
that reader they are reading a translation is simply false, and it is false in the one direction
that costs something: they are the reader best placed to go read the original, and the notice
would give them no reason to.

So the copy is a matrix of state against interface language rather than one sentence per
language. When the view's language differs from the article's, the notice names the source
language and links it. When they match, it states the language plainly, says the version may
carry small changes in wording, and recommends the original -- with the link on the word for the
original, since that is what is being recommended rather than merely named.

### `lang` names the main language, not the only one

An article may mix languages. Frontmatter carries one tag because one tag is what an author can
honestly give: the language most of it is in. Every sentence built on that tag has to stay true
of an article that is mostly rather than wholly in it, which is why the notice says "mainly",
"principalmente", "主に" rather than asserting the article simply is in that language. The
qualifier is not hedging -- it is the actual strength of what `lang` records.

### A third state: the same language, the other script

Chinese is published here under two scripts, so a Simplified article can be read at `tw` by
someone who reads Simplified perfectly well. That is neither of the first two states. Nothing
was translated and nothing was polished; characters were mapped. Announcing it as a translation
overstates the distance by a whole language, and announcing it as a light polish understates why
the reader should move -- they can read the original exactly as written, and the only thing
between them and the author is a script they already know.

This state is checked before the translated one, since an article whose language has a sibling
script would otherwise fall through and be described as translated.

Its copy is keyed by the two Chinese views alone rather than added as a third row to the matrix.
The state is reachable only from the view that _is_ the sibling script, so six of the eight rows
could never be shown; a table that can only ever be two-thirds filled is the wrong shape for the
fact, and an optional row would let a real gap look deliberate.

That word is held separately from the same word as a menu row label. English capitalises a
label and not a mid-sentence noun, German capitalises both; one string cannot be correct in both
positions, and merging them would fix one language by breaking another. This is the exception
that proves the one-fact-one-home rule rather than a violation of it: two grammatical positions
are two facts.

### The script is part of the name

A source language is named with its script wherever the script distinguishes it. Chinese is the
only such case here, and `Intl.DisplayNames` already spells the distinction out in every
interface language, so the script is restored onto the tag before it is handed over rather than
eight names being written by hand. `zh` alone answers "中文" or "Chinese", which covers both
scripts and therefore names neither.

This applies to the notice, which spells the language out in full. The switcher's own row for
the original stays short on purpose -- see the compact-label rule above -- and naming the script
there would fight the reason that label is abbreviated at all.

Keeping it out also keeps an unvalidated value away from the one attribute where a bad value is
destructive rather than merely wrong. Frontmatter `lang` still reaches `<html lang>`, where an
error mislabels a page; in an `hreflang` it would discard the whole set.

**An `hreflang` URL must be the canonical URL of the page it names.** Point one at a page that
canonicals elsewhere and the entire set is discarded — which is what makes the next rule a
correctness requirement rather than a tidiness one.

## A translation identical to its original defers to it

An article written entirely in one language will come back from that language's translator
almost unchanged. Two of the nine views are then the same text at two addresses, and the pair
competes with itself.

**When a locale is at least 0.90 similar to the original after normalisation, that locale
canonicals to the bare URL instead of to itself, and its `hreflang` entry points there too.**
Below the threshold it is a translation like any other.

### The comparison is over translatable content only

Only the translatable spans are compared: the source text of those spans against the translated
text of the same spans. Not the assembled views.

This was got wrong first and the wrong version shipped, so the reason is worth stating. An
assembled view is mostly material both sides share — code blocks, links, markdown structure,
untranslated frontmatter — so scoring whole views largely measures how much code an article
contains. Every score rises, and they rise by different amounts per article. Measured that way,
`zh-TW` scored 0.943 on one article and folded into the Simplified original, while scoring
0.890, 0.706 and 0.606 on the others: the same language behaving differently per article, which
is harder to diagnose than behaving wrongly everywhere. Japanese sat at 0.850, one code-heavy
article from the same fate.

Comparing translatable spans separates the cases cleanly. Across the five articles that exist,
the locale sharing an article's language scores 0.947 to 1.000, every other locale scores 0.719
or below, and nothing lands between. The homepage is the useful check: it is written in English,
so `en` scores 1.000 and defers while `zh` scores 0.495 — the rule follows the article rather
than assuming Chinese.

Exact equality was tried before either of these and rejected: only 40 of 52 segments in one
article matched byte for byte, so the rule would never have fired in the case it exists for.

### The threshold is not centred, on purpose

0.90 sits nearer the top of a gap running from 0.719 to 0.947. That is the safer end.

Folding wrongly sends a reader to a language they did not ask for and drops a translation
somebody paid for out of the index. Failing to fold leaves a near-duplicate, which a crawler
consolidates by itself. The two mistakes do not cost the same, so the boundary leans toward not
folding.

This also removes the question of what to do when articles start mixing languages, which
[i18n.md](i18n.md) says they will. A mixed original produces a Simplified Chinese view that
genuinely differs from it, the similarity drops, and the rule stops applying on its own without
anyone revisiting it.

## The sitemap lists addresses, not language codes

The sitemap emits one `url` entry per distinct canonical URL. It does not assume that nine
codes produce nine entries: when a translation defers to the original, both codes share the
bare address and only one entry is emitted for it.

Every article entry carries the complete `xhtml:link` alternate set, including its own URL and
`x-default`. The set is the same one the page head receives from `indexingMetadata`; the
sitemap never reimplements similarity, canonical selection, or the internal-code mapping. A
set that omits its own member, or disagrees with the page head, is invalid as a whole rather
than partially useful.

## Atom puts its language in the URL

The bare `/atom.xml` is the original feed. Each translation is `/atom.xml?lang={code}`, using
the same internal codes as article pages. Atom reads that parameter and nothing else. An
unknown value falls back to `mw`; cookies and `Accept-Language` are deliberately invisible to
the route so they cannot make one cached URL contain different readers' content.

The feed and every entry declare the BCP-47 language of the view actually served. Entry titles,
summaries and bodies come from that resolved view, including translated frontmatter. A page's
feed-discovery link names the same code as the page, so subscribing from a translated view
selects its translated feed.

## HTML is never cached; assets still are

The response body for a page depends on a cookie, so **HTML is served with `Cache-Control:
private, no-store`**. A cached page is a page some other reader's language is about to be
served from. Atom is the deliberate opposite: its language is wholly in the URL, so it remains
`public, max-age=360, s-maxage=360` and is safe in shared caches.

This is the deliberate exception to the rule in [architecture.md](architecture.md) that a
hashed name is cached for a year. HTML carries no hash in its name, so it was never covered by
that rule; saying so here is what stops a later reader from assuming the general case applies.
Everything the page references — images, fonts, styles — keeps its usual lifetime, because none
of it varies by language.

The site worker is not a cache tier and does not need to be. Articles are compiled into it at
build time with all nine views present, so a request is a lookup and a return; there is no
origin to protect and nothing expensive to avoid repeating.
