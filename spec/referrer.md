# What a request from here tells the site it goes to

The site's referrer policy is `origin-when-cross-origin`, set as a response header on everything
the site worker serves. A request inside the site carries the full URL. One to another site --
a link followed out of an article, an image the CDN serves, a font -- carries the origin alone.

## The origin is sent on purpose

A site linked from an article learns that the link came from `canmi.net`. That is the author's
choice: a site being cited should be able to see it was cited, and by whom. What it does not
learn is which article, because the path is the part that says what somebody was reading, and
that stays here. `same-origin` would have sent nothing at all, and `strict-origin-when-cross-origin`,
the browsers' own default, differs from this only in dropping the origin on a downgrade to plain
HTTP -- a case this site does not link into, and one the author chose not to special-case.

## `noreferrer` is left off every link out

Every link that opens a new tab is `rel="noopener"` and nothing more. `noopener` keeps the opened
page from reaching back into this one, which is a protection. `noreferrer` would also strip the
referrer from that navigation, overriding the policy above for exactly the links it was set for;
twenty-five links carried it because that is the pair every guide recommends together, and every
one of them was silencing a policy set to speak. The pair is not one thing, and only one half of
it is wanted here. Article links compiled by `compile.ts` follow the same rule.

## The security headers live in the repository, not at the edge

Before this, the header on production was `same-origin`, and it came from nowhere in the
repository: Cloudflare's "Add security headers" managed transform put it on every response,
alongside `X-Frame-Options: SAMEORIGIN` and `X-Content-Type-Options: nosniff`. Development served
none of the three, production served values nobody could grep for, and the two disagreed. When
the worker started sending its own policy the transform overwrote it -- it sets, it does not add
-- which was measured on the deployed build before the setting was turned off.

That transform is off, and all three headers are set in `hooks.server.ts`, before the markdown
handler so that its responses carry them too. `X-Frame-Options` and `X-Content-Type-Options` are
kept because they were doing something worth keeping: no other site may frame these pages, and
no browser may guess a type a response did not state. Whenever production and the repository
disagree on a header, the zone's transform rules are the thing to check first.

Static files the platform serves without running the worker -- `robots.txt`, the favicons, the
fonts -- do not pass through the hook and carry none of these. None of them is a document a
browser could frame or a page that sends a referrer, so nothing is lost there.
