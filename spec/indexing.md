# Telling a search engine what changed

The sitemap is the pull side of discovery: it lists every address and waits to be crawled. See
[locale.md](locale.md) for what an address is and [architecture/data.md](architecture/data.md)
for what is allowed into it. This file is the push side -- IndexNow, which notifies the
participating engines that something moved rather than waiting for them to notice. Searching
this site's own corpus is [search.md](search.md), which runs beside this one and shares its
ordering, but answers a reader here rather than announcing an address elsewhere.

## The key is published, because the protocol requires publishing it

Ownership is proved by serving the key as a text file the engines can fetch. That makes it
public by construction: anyone who can read the proof can read the key. It is written in plain
text in `site.config.yaml` for the same reason the analytics client id is -- see
[analytics.md](analytics.md) -- rather than kept somewhere that implies a secrecy the protocol
does not allow.

It lives in the site config rather than in `libs/urls` because it is not an address. It is an
identity this site proves, like the Bluesky handle beside it, and `libs/urls` holds what
resolves.

**There is no verification step that completes.** An engine fetches the key file on _every_
submission, not once when the site is set up. The file therefore has to stay served for as long
as the key is in use; taking it down turns the next submission into a `403` and nothing else
reports it.

### The path is derived from the key, never written twice

The route is a dynamic segment checked against the config, not a directory named after the key.
Naming the directory would make the address a second copy of a value nothing compares, so
rotating the key would leave the file served under the old name -- and that failure appears only
as a `403` on the next submission, with no line anywhere saying which of the two is stale. One
edit has to move both, which means only one of them may be written down.

### It is served from the site, not from the CDN

`keyLocation` must be **on the same host** as the URLs being submitted. The CDN is a different
domain entirely, so a key file there proves nothing about this site, and a redirect from the
site to it is undefined in the protocol rather than allowed -- a bet whose losing outcome is,
again, a silent `403`. The site already serves `robots.txt`, `llms.txt` and `licenses.txt` from
prerendered routes; this is one more of those and costs the same.

## Only what changed is submitted

IndexNow is for URLs that were added, updated or deleted. Replaying the whole sitemap on a
schedule is what its `429 Too Many Requests (potential Spam)` exists to stop, and the cost of
doing it is not a rejected request but a key the engines learn to discount.

So `mise run indexnow` reconciles rather than broadcasts: it reads the live sitemap, compares it
against a record of what was sent before, and submits the difference. An unchanged site sends
nothing.

### What changed is decided by a content hash, not by a date

**The record holds a fingerprint of the files a page is built from.** Remembering only which
URLs have been seen would mean an edited article is never announced again -- the one event the
protocol most wants -- so the record has to carry something that moves when the page does. Three
candidates, and only one of them works.

`lastmod` from the sitemap is the obvious one and is wrong in both directions. It is a display
value the author owns: there are edits where the date shown to a reader should deliberately stay
put, and tying submission to it would make "tell the search engines" a side effect of a decision
about what a page claims about itself. Concretely, a rebuilt but unchanged page carries a fresh
build timestamp and would be announced for nothing, while an article whose translations were
rewritten keeps its old frontmatter date and would never be announced at all -- which is exactly
what happened the day this was written.

The file's mtime decouples the two and does not survive. Neither jj nor git records it, so a
fresh clone dates every file to the checkout and the next run announces the whole site. It also
moves for things that are not changes: a `touch`, a checkout, a reformat.

So the fingerprint is a hash of the bytes, which is the primitive segment ids and asset ids
already use here. It answers the question directly, survives a clone, and stays put when nothing
about the content did.

**Hashed per view, from the files that view reads.** The source view takes the article and its
summary; a translated view takes the sidecar as well, so rewriting translations announces the
translated addresses and leaves the source one alone. The sidecar is hashed whole rather than
per locale -- reading one locale out of it would teach this script the sidecar's shape to buy a
distinction that only matters on the days translations change anyway.

Paths, not URLs: the record describes one site, and storing the origin on every row would repeat
one string a few hundred times. It is machine-written JSON in `data/`, not hand-edited YAML,
because nothing about it is a judgement a person makes.

**The licence directories are not announced.** They are in the sitemap so they can be crawled,
and that is all they need: derived pages nobody is waiting on, whose only timestamp is the
build's. Announcing them would mean announcing thirty URLs on every deploy.

`--seed` records what is live without announcing it, for when the engines already hold these
URLs but this record does not agree -- a submission made by hand, or a change to how the
fingerprint is computed, which makes every page look new while none of them is.

**Written only after the request is accepted.** Recording first would let a failed submission
look sent, and the next run would skip exactly the URLs that never arrived.

Deletions are outside this. The sitemap does not list a page that is gone, so noticing one means
diffing against the previous sitemap rather than reconciling with it -- a separate mechanism,
worth building when a page is actually withdrawn rather than in advance of one.

## The sitemap is read over HTTP, from production

The sitemap route is generated per request so that `changefreq` and `priority` describe
staleness at crawl time. There is no build artifact to read, and what is live is what an engine
would see, which is the thing being reconciled. `mise run indexnow` therefore belongs after
`mise run deploy-site`, never before it.
