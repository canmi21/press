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

An ordinary API `build` is a dry run and must not mutate a remote database. The API `deploy` command
applies remote D1 migrations immediately before uploading the Worker. Local development applies
local migrations before starting Wrangler, whose default local D1 implementation is Miniflare.

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
`localStorage["email"]` for a future cancellation UI. The API stores only the token's SHA-256 hash.
A duplicate request without the original token still succeeds, but cannot receive a replacement
token because that would let another visitor cancel the subscription.

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
across page reloads. Engagement data is fresh for 15 minutes and may remain available as a
seven-day fallback while a background refetch refreshes stale data.

Mutations and errors are never persisted. Email addresses and cancellation tokens never enter the
query cache. The dedicated `localStorage["email"]` capability record is independent application
logic and is not part of TanStack Query cache eviction or hydration.
