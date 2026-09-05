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

## It lives in the repository, not at the edge

Before this, the header on production was `same-origin`, and it came from nowhere in the
repository: a zone-level setting at Cloudflare added it to every response, alongside
`X-Frame-Options` and `X-Content-Type-Options`. Development served no policy, production served
one nobody could grep for, and the two disagreed. The header is now set in `hooks.server.ts`,
before the markdown handler so that its responses carry it too, and the edge must not overwrite
it: a Cloudflare transform that *sets* rather than *adds* this header would win, so that setting
is the thing to check whenever production and the repository disagree.
