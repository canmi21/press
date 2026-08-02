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

The query parameter wins outright and **also writes the cookie**, so choosing a language once
is choosing it from then on. That is what makes the parameter a switch rather than a one-off
override, and it is why the language switcher needs no separate mechanism.

Selection runs in the worker on the request, before any HTML is rendered. Every input already
arrives there, and choosing after first paint would make a page render in one language and then
swap. Unlike theme, the content itself differs, so this cannot be a class toggle — see the
caching rule below for what that forces.

Browser-facing collection surfaces use the same resolved code for every article they include.
Homepage cards and article lists take their title, subtitle, description, body, and language
from that article's resolved view; `mw` means each article's own original. `llms.txt` keeps its
existing behaviour and has no indexed language dimension: an LLM can read any of the views.

Server-only discovery routes do not inherit this negotiation. The sitemap publishes every
indexable view at once. Atom selects from `lang` alone, because each query-specific feed is a
shared-cache resource and neither a cookie nor `Accept-Language` is part of its address.

## The query parameter is for crawlers, and is removed for readers

A crawler has no cookie and does not run a language switcher, so without a URL that names a
language it can only ever see one of the nine views. **The default URL stays bare and every
other language is reachable at `?lang={code}`.** That is the only reason the parameter exists.

Once the page has loaded, the parameter is removed with `history.replaceState`. The reader
keeps a clean URL, the cookie already holds the choice, and nothing about the page depends on
the parameter still being there.

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

`mw` means *the original*, and which language that is depends on the article. Today every
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
