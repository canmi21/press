# Searching the corpus

`mise run search` reconciles an Algolia index with what this site actually serves. It is the
third of three things that happen after a deploy, and the only one a reader interacts with:
[indexing.md](indexing.md) tells other people's search engines what moved, this one answers
queries typed into this site.

## One index, and the locale is a filter

An article has nine views ([locale.md](locale.md)), and a reader is only ever in one of them.
Every record carries its internal locale code, every query filters on exactly one, and that is
the whole of it. Nine independent searches out of one index. A reader in `zh` never sees a `tw`
result.

The design this replaced split the corpus into two indices, one per writing system, on the
grounds that `queryLanguages` and `indexLanguages` are index settings rather than record
fields -- a filter chooses which records come back, and cannot retroactively change how their
text was cut into tokens. The reasoning was sound and the premise was not: **declaring those
settings changes nothing this corpus can detect.**

### Measured, because the documentation contradicted itself

Algolia's normalization guide says a generic CJK process applies whether or not a language is
declared -- ICU word detection, falling back to sequential character matching. Its support
documentation says an unconfigured index splits on spaces and punctuation and can return
nothing for a language that uses neither. Both cannot be true, and the difference decides
whether this file describes one index or two.

Five index variants over the same 212 records -- the corpus split into sections, in `mw`, `ja`,
`de` and `en` -- queried thirty ways across those four languages:

| Variant                                    | Result    |
| ------------------------------------------ | --------- |
| No language settings                       | baseline  |
| `['zh', 'en']`                             | identical |
| `['zh', 'en', 'de', 'es', 'fr']`           | identical |
| `ignorePlurals` + `removeStopWords`, bare  | identical |
| the same two flags plus `['zh','en','de']` | identical |

Identical in hit count and in the order of the top five, for `渲染`, for the compound
`编译期渲染`, for `レンダリング` and `ハイドレーション`, for `Komponente` against `Komponenten`, for
`component` against `components`, and for a full sentence carrying stop words. The engine
segments Chinese and Japanese, and folds German and English inflections, without being told
which language it is looking at.

**The control matters more than the result.** A run where nothing changes is also what a run
looks like when the settings never landed, and this key cannot read settings back -- `GET
settings` returns `403`, the restricted ACL doing its job. So a sixth index was built with
`searchableAttributes` limited to `title`, and a body-only term dropped from fourteen hits to
zero. Settings writes land; these particular settings simply do not do anything here.

One correction the run forced: German compound splitting is `decompoundedAttributes`, a
separate setting, not something `queryLanguages` switches on. The earlier argument for a Latin
index rested on a knob that was never being turned.

So: one index, no language settings, and the only setting that earns its place is
`searchableAttributes` -- the one thing measured to change behaviour.

### Which deletes the mapping table before it was written

The vendor's language codes never leave the vendor, because none are ever sent. The internal
codes ship as tags exactly as they are written -- `mw`, `zh`, `tw` -- and nothing has to
translate them.

That retires a trap this repository had already documented once. `LOCALE_CODES` is
character-for-character identical to Algolia's ISO 639-1 codes in seven of nine places, which
would make an identity mapping look correct while being silently wrong twice: **`tw` is Twi, a
language of Ghana**, as [locale.md](locale.md) records in another context, and Algolia has no
`zh-Hant` to map Traditional Chinese onto in any case. A table plus a test holding it against
`LOCALE_CODES` was the plan. Sending nothing is better than mapping correctly.

### `mw` still needs records of its own

`mw` is not a language -- it is the original, and which language that is depends on the
article. It is also the fallback the locale negotiation lands on, so it serves the reader who
asked for originals and the reader whose language this site does not have.

None of that is a settings problem any more, but it remains a records problem: the `mw` view
cannot borrow another locale's. The translation pipeline localises terms the original
deliberately left in English -- the source writes `hydration mismatch` where the `zh` view
writes 水合不匹配, while `tw` keeps the English. Three Chinese views, three different answers
about one term. Titles differ too: `mw` carries the source title, every other view a translated
one. A reader on `mw` searching `hydration mismatch` finds it only in the `mw` records.

## A record is a section of a view, and its address is already computed

One record per (article path, locale, section), with `objectID` those three joined so that
re-pushing overwrites rather than accumulating. Six articles across nine locales currently make
770 records.

Sections rather than whole views, for two reasons that happen to agree. A record has a size
ceiling a long article passes on its own, and a result that lands on the paragraph answering
the query is worth more than one that lands at the top of the page. The site already computes
both halves of what that needs: `blocks` carries every heading with the slug it renders as, and
`text` is the plain-text view with advanced expressions collapsed to empty. Boundaries come
from the first, words from the second, and neither is re-derived -- reading the text off the
blocks would mean reimplementing the plain-text renderer, and reading the slugs off the text
would mean guessing at them.

A section is split further at a byte ceiling well under the vendor's, because a section that
merely fits is one edit away from not fitting, and that failure would land at push time on
whoever was publishing rather than at the moment the paragraph was written. A single paragraph
over the ceiling still gets its own record: cutting mid-sentence would cut a match in half,
which is worse than one record running long.

The URL is `views[code].canonical` with the section's slug appended, taken as-is.
`build/indexing.ts` already decides whether a view keeps its own `?lang=` address or collapses
onto the source's, and deciding it a second time here is how the two come to disagree.

### The fingerprint is over the record, not over its sources

`indexnow.ts` hashes the files a page is built from because it cannot see what it sent. Here the
thing being compared is present in the index, so it is hashed directly: title, subtitle,
heading, text and address.

That is simpler, and it is also stricter in the direction that matters. Hashing sources would
inherit a cost [indexing.md](indexing.md) accepts for its own purposes and this one should not:
the translation sidecar is one file for eight locales, so a source hash moves for all eight the
moment one of them is rewritten, and eight-ninths of the resulting push would be identical
bytes. Hashing the record moves exactly the records whose words moved. Rewriting the French
translation writes French records and nothing else.

## The state lives on the remote, because the remote can be read

[indexing.md](indexing.md) keeps `data/indexnow.json` because IndexNow is write-only: it cannot
be asked what it already knows, so a local record is the only thing that can answer "what
changed". Algolia can be browsed. That difference is worth taking, rather than copying the
shape of the neighbouring task.

**The fingerprint is stored as an attribute on the record itself**, and left out of
`searchableAttributes` so it is written to be compared and never matched. A run browses the
index for `objectID` to fingerprint, computes the same map from the corpus, and pushes the
difference. There is no local file, so there is no way for a local file to be wrong about
production. An index that does not exist yet browses as empty, which is what a first run is.

Two things fall out of it. `--seed` has no equivalent and needs none -- the record cannot
disagree with reality when it _is_ reality. And deletion, which IndexNow defers because a
sitemap that stops listing a page says nothing, is simply the other half of the same diff: an
`objectID` the remote holds and the corpus no longer produces is deleted. A stale record in a
search index does not decay quietly like an unannounced URL; it keeps being returned.

## It runs after the deploy, and it is not part of `sync`

The index describes what the site serves, so it is aligned once the site serves it -- the same
ordering [indexing.md](indexing.md) states for IndexNow, for the same reason. A record pointing
at an address that is not live yet, or holding text the site no longer has, is wrong in a way
nothing reports.

`sync` is the wrong host for it twice over. It runs _before_ a deploy, because assets have to
exist before the page referencing them ships. And its safety is structural: rclone is pointed
at `data/public` and can physically see nothing else, which is what makes it fail closed.
Publishing something with a different scope from inside that command retires that argument,
and its `--dry-run` default would then cover one of the two publications rather than both.

## The write key never leaves this machine

Two credentials, treated oppositely, because they are exposed oppositely.

**The write key is only ever held by a person running `mise run search`.** It goes in
`secrets.json` under sops, decrypted into the task's environment by mise. It is not a wrangler
secret and not a CI variable, because nothing that is deployed ever writes to the index --
adding it to either would grant a write credential to a build that has no use for it.

It is named `ALGOLIA_WRITE_KEY`, and the supplier's name in it is deliberate. The environment is
the binding edge, which is exactly where the workspace's `naming.md` allows a vendor name, and
where `RCLONE_CONFIG_R2_*` and `SENTRY_AUTH_TOKEN` already sit. What that rule protects is
everything above the edge: the task is `search`, the spec is this file, and no module, type or
function inside the site or the push script carries the supplier's name. Changing suppliers
rewrites the binding layer rather than every place that touched it.

It is scoped to the `press_*` indices, the same move as the R2 token scoped to one bucket in
[toolchain.md](toolchain.md): the boundary holds because the credential cannot cross it, not
because whoever ran the command was careful. What it is _not_ scoped to is destruction --
`deleteIndex` is left on deliberately, and the reason is that the two credentials are not
comparable. Losing a bucket loses the only copy of an object; losing this index costs one run
of `mise run search`, because `contents/` is the source and the index is derived from it in
full. A permission whose worst outcome is a command being run again does not need fencing.

Reading settings back is not granted, and that shows up in normal use the way the R2 token's
missing list permission does: `GET /1/indexes/{index}/settings` returns `403` while writing
them succeeds. Nothing needs it -- the task pushes settings and never asks what they were, and
a run that wants to prove a settings write landed observes its effect instead.

**The search-only key and the application id ship in the browser bundle**, so they are public
by construction and are written in plain text under an `algolia` block in `site.config.yaml` --
the same edge, named the same way -- beside the IndexNow key and for the same reason
([analytics.md](analytics.md)). What restricts them is Algolia's own key scoping and rate
limiting, not obscurity. They are identities, not addresses; the host they
resolve to is assembled from the application id at the point of use, against a base in
`libs/urls`.

The cost of keeping the push local is that a deploy does not update the index -- someone has to
run the command. `indexnow` already behaves this way, so it is consistent rather than a new
kind of surprise, but it is written here because "I pushed and search is stale" is otherwise a
thing to rediscover.

## What the corpus costs, and which limit binds first

Six articles across nine locales are 770 records. The multiplier is the sectioning, not the
locales: 54 views become 711 sections, and the byte ceiling adds the last 59. Fourteen records
per view on average, seven at the least and twenty-seven at the most.

Projected at that shape, a hundred articles are roughly 12,800 records and two hundred are
roughly 25,700, against a free tier that includes 50,000. **The record count is not the limit
that binds.** The 10,000 monthly search requests are, and they are spent by the interface
rather than by the corpus: a search-as-you-type field issues one request per keystroke, so ten
thousand requests is a few dozen searches a day. Whatever protects that budget -- a debounce, a
minimum query length, searching on submit -- is a decision about the search field, and it does
not touch anything in this file.

Coarser records were considered and are not worth it. One record per view is 54 rather than
770, but a whole view exceeds the size ceiling, so it is not a smaller index -- it is a
truncated one, where the tail of a long article stops being searchable. Cutting at `##` and
folding deeper headings into their parent saves about 13%: the corpus has 62 second-level
headings and 11 below them, which is not enough to pay for losing those anchors.

None of this needs deciding early, because the index is derived. `contents/` is the source, one
command rebuilds every record from it, and changing the granularity costs a run rather than a
migration.

## What the measurement does not cover

It was taken over 212 records. Relevance differences that only appear once a corpus is large
enough for many documents to match one query are invisible at this size, and so is anything
below the fifth hit. What is established is narrow and sufficient for the decision it settled:
at this scale, on these four languages, the language settings are inert.

It should be re-run rather than trusted if the corpus grows by an order of magnitude, or when
a source language arrives that is neither CJK nor Latin -- Arabic, Thai and Hebrew are the
cases where a declaration is most likely to start mattering, since none of them separate words
the way the four tested here do. The scripts are throwaway; the queries are the part worth
writing down, and they are in this file.

Untested and deliberately so: synonyms, custom dictionaries and `decompoundedAttributes`. Each
is a feature this site does not use, and measuring a feature before wanting it is how a
settings block accumulates entries nobody can explain.
