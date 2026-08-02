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

## Three sources, in one order

A request resolves its locale from the first of these that answers:

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

Collection surfaces use the same resolved code for every article they include. Homepage cards,
article lists, `llms.txt`, and Atom entries take their title, subtitle, description, body, and
language from that article's resolved view; `mw` means each article's own original. These
responses vary by the same request inputs as an article page and therefore are not shared-cache
content either.

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

Each of the nine views is its own canonical: the original at the bare URL, every other language
at its `?lang=` form. Nine self-canonical URLs, one per language, so ranking weight accumulates
where the content actually is instead of being handed to a version the reader never asked for.

The full `hreflang` set accompanies every view — all nine alternates plus `x-default` pointing
at the bare URL. Canonical alone says "this is the address of this page"; it does not say "these
pages are translations of one another", and without that a crawler is left to guess from
content it has already decided is similar.

**An `hreflang` URL must be the canonical URL of the page it names.** Point one at a page that
canonicals elsewhere and the entire set is discarded — which is what makes the next rule a
correctness requirement rather than a tidiness one.

## A translation identical to its original defers to it

An article written entirely in one language will come back from that language's translator
almost unchanged. Two of the nine views are then the same text at two addresses, and the pair
competes with itself.

**When a locale's assembled text is at least 0.90 similar to the original after normalisation,
that locale canonicals to the bare URL instead of to itself, and its `hreflang` entry points
there too.** Below the threshold it is a translation like any other.

The threshold is measured, not chosen for roundness. Across the four articles that exist, a
same-language pair scores between 0.95 and 1.00 — the differences are punctuation the model
normalised, `"` becoming `“”` and `?` becoming `？` — while a genuine translation of the same
article scores between 0.33 and 0.54. Nothing lands between 0.54 and 0.95, so the boundary sits
in an empty region and small errors in either direction change no outcome.

Comparison is on assembled output rather than on stored segments, because that is what a reader
and a crawler actually receive. Exact equality was tried first and rejected: only 40 of 52
segments in one article matched byte for byte, so the rule would have failed to fire in exactly
the case it exists for.

This also removes the question of what to do when articles start mixing languages, which
[i18n.md](i18n.md) says they will. A mixed original produces a Simplified Chinese view that
genuinely differs from it, the similarity drops, and the rule stops applying on its own without
anyone revisiting it.

## HTML is never cached; assets still are

The response body for a page depends on a cookie, so **HTML is served with `Cache-Control:
private, no-store`**. A cached page is a page some other reader's language is about to be
served from.

This is the deliberate exception to the rule in [architecture.md](architecture.md) that a
hashed name is cached for a year. HTML carries no hash in its name, so it was never covered by
that rule; saying so here is what stops a later reader from assuming the general case applies.
Everything the page references — images, fonts, styles — keeps its usual lifetime, because none
of it varies by language.

The site worker is not a cache tier and does not need to be. Articles are compiled into it at
build time with all nine views present, so a request is a lookup and a return; there is no
origin to protect and nothing expensive to avoid repeating.
