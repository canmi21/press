# Reader engagement

Newsletter subscriptions and likes are mutable reader state. They belong to the standalone API
Worker at `apps/api`, never to the site Worker. The site has no embedded `/api/*` routes, and SSR
does not fetch engagement data. Server rendering remains limited to locale selection and content
that was available at build time.

## One D1 database owns API state

The API has one Cloudflare D1 database for all of its relational state. Both the database name and
the Worker name are `api`; its binding is the complete word `DATABASE`. The production database is
in WNAM and has ID `959379fe-f528-4304-9f40-3e1c957208fe`.

Drizzle owns the TypeScript schema and generates committed SQL migrations. Migration filenames use
Drizzle's indexed random-word form, such as `0000_word_word.sql`; they are not named by hand.
Wrangler, not Drizzle, applies those migrations so D1 has one migration ledger. CI and deploys apply
committed SQL but never generate it.

Every API `build` applies remote D1 migrations before producing the dry-run Worker bundle. This
keeps Cloudflare Workers Builds from deploying code before its schema and deliberately means that
both local and non-production API builds target the production database. The build credential must
therefore have D1 edit access. The fallback API `deploy` command independently applies remote
migrations immediately before uploading the Worker. Local development applies local migrations
before starting Wrangler, whose default local D1 implementation is Miniflare.

## Newsletter identity is the email address

The API trims and lowercases an address, then removes the first `+` and everything after it in the
local part. It validates the resulting bare ASCII address before writing it. For example,
`Alice+notes@Example.com` becomes `alice@example.com`.

The canonical email is the subscription identity and the database primary key. Duplicate requests
do not create another subscriber. The raw `CF-Connecting-IP` value is stored with a new subscription
for future analysis only; it is not used to identify or deduplicate a subscriber.

A newly created subscription returns its canonical `email` and a `cancel_token`. API payload keys
use lowercase snake case. The token is 16 cryptographically random bytes encoded as exactly 32
lowercase hexadecimal characters. The browser stores the canonical email and raw token as JSON in
`localStorage["email"]`. The API stores only the token's SHA-256 hash. A duplicate request without
the original token still succeeds, but cannot receive a replacement token because that would let
another visitor cancel the subscription.

## The browser recognises its own subscription

That record is the only thing a returning reader is known by. HTML is rendered by a Worker that
cannot see `localStorage`, so the subscription form is what the server sends and the confirmed
state replaces it after mount.

**The pill itself is one element across both states.** Its box, its border and the place its
button occupies do not move; only what sits in them is replaced. The submit button keeps its own
copy after subscribing and becomes inert -- there is nothing left to submit, and it is hidden
from assistive technology, which has the state from the pill's label and would otherwise be
offered a control that does not exist. A check mark was there first and was the wrong shape for
the moment: it re-announced what the sentence below already says, at the cost of the one landmark
the reader had just aimed at.

A record that does not parse, or whose token is not the 32 hexadecimal characters the API will
accept, is deleted rather than shown. The alternative is an unsubscribe control whose every use
fails, which is worse than not offering one.

**The confirmed state and the ability to cancel are separate facts.** Subscribing again from a
device that never held the token confirms the address and offers no cancellation, because that
device genuinely cannot cancel. A cancellation the API answers with `404` clears the local record
and reports success: the subscription is already gone -- most likely cancelled from another
browser -- and an error would leave the reader looking at a subscription they cannot get rid of.

Cancelling takes one click and no confirmation step. Resubscribing is the same form that is
already on the page and issues a fresh token, so the mistake costs a click to undo.

### The address is redacted in place, and only when somebody did it

Subscribing sweeps a clip left to right across the masked address while a plain copy of what was
typed fades out beneath it, both in one grid cell so the masked form arrives exactly where the
field's text was. The reading is that the address was redacted rather than swapped, which is the
truthful account of what happened to it.

**A record read at mount animates nothing.** Someone returning to the page did not just do
anything, and replaying the confirmation would claim they had. The animation belongs to the
interaction, not to the state.

Reduced motion goes straight to the confirmed pill. The typed copy exists only to be animated
away, so without the animation it is not rendered at all rather than left behind for a timer to
remove.

This is a clip rather than the measured masks the Support rail uses. That geometry depends on the
rendered width of two labels and cannot be written down in advance; this one is always the whole
box, which puts it on the CSS side of the rule in [architecture.md](architecture.md).

### One row under the pill, in every state

Below the pill sits a single line: status text on the left, the unsubscribe control on the
right. It is present whether or not anyone is subscribed, so nothing further down the page moves
as the section changes state.

The left slot carries whatever the reader most recently needs to know -- an error, a
cancellation, a fresh confirmation -- and otherwise falls back to the subscriber count. The
confirmation therefore lasts one visit rather than persisting: by the time the page is reloaded
the pill already says the reader is on the list, and the sentence has been read. Stacking these
as separate lines was the first attempt and it made the section grow a row at a time as it
changed state.

The right slot is the only place a destructive action appears, and it is a text link rather than
a button surface. Reserving one edge for it keeps it from ever being the thing under a cursor
aimed at the subscribe button.

### The address is shown masked, and the domain is the identifying half

A confirmed subscription shows the address it is for, because the reader needs to tell their
address from a typo of it, and shows it masked, because a page may be read over somebody's
shoulder or on a shared screen. The first character of the local part survives and the rest
becomes bullets.

The domain is treated separately, since the two carry different amounts of identity. A mail
provider names nobody: thousands of readers share `gmail.com`, so it stays legible and is what
makes the masked address recognisable at all. A domain the reader controls is the opposite --
`canmi.net` identifies one person as surely as the full address -- so everything but the final
label is masked. A short allowlist of providers separates the two; anything absent is treated as
the reader's own.

The public suffix list is deliberately not used to find the registrable domain. Its payload
cannot be justified for this, and approximating it would leave `example.co.uk` exposed while
hiding `example.com` -- a rule that is wrong only for some readers is harder to trust than one
that always keeps a single label.

## A like is one active row per IP

The raw `CF-Connecting-IP` value is the like identity and database primary key. An IP can contribute
at most one active like. Liking inserts the row if absent; unliking deletes it. A state query returns
the current IP's `liked` boolean together with the global like and subscriber counts.

The stored IP values are not D1 rate-limit counters. The state query and mutation endpoints use
separate Cloudflare Workers Rate Limiting bindings keyed by the raw IP, with a wider allowance for
reads. This is deliberately approximate, inexpensive abuse resistance rather than a globally
strict quota.

## Engagement data is a persisted client query

The site fetches engagement state in the browser from the standalone API origin. TanStack Query
deduplicates the shared request used by Newsletter and Support. Its sync-storage persister writes
all successful TanStack Query query data into the single global `localStorage["cache"]` container
across page reloads. All queries remain fresh for five minutes; once stale, normal TanStack Query
refresh triggers update them. Unused in-memory data and cross-reload persistence may remain for up
to three days as a fallback while a stale query refreshes in the background.

Mutations and errors are never persisted. Email addresses and cancellation tokens never enter the
query cache. The dedicated `localStorage["email"]` capability record is independent application
logic and is not part of TanStack Query cache eviction or hydration.
