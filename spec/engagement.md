# Reader engagement

Newsletter subscriptions and likes are mutable reader state. They belong to the standalone API
Worker at `apps/api`, never to the site Worker. The site has no embedded `/api/*` routes, and SSR
does not fetch engagement data. Server rendering remains limited to locale selection and content
that was available at build time.

## One D1 database owns API state

The API has one Cloudflare D1 database for all of its relational state. Both the database name and
the Worker name are `api`; its binding is the complete word `DATABASE`. The production database is
in WNAM, and its id lives in [wrangler.jsonc](../apps/api/wrangler.jsonc) -- the only file that
consumes it. Copying the id here would make a third home for it, and the only one nothing checks.

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
button occupies do not move; only what sits in them is replaced. A check mark was there first and
was the wrong shape for the moment: it re-announced what the sentence below already says, at the
cost of the one landmark the reader had just aimed at.

The button's place instead holds the outcome, so the shape somebody just used becomes the label
for what it did. It is inert -- there is nothing left to submit -- and it is the pill's whole
accessible content, the masked address being of no use read aloud.

Two things follow from it no longer being an action. It gives up the ink surface for the quiet
raised one, since ink is reserved for the thing worth pressing. And its copy states the state
rather than repeating the verb: `You're in` beside an `Unsubscribe` link, never `Subscribe`
beside it.

**Both labels are laid out in every state and the unused one is only made invisible**, so the
button is as wide as the wider of the two and its edge does not move when the copy changes. The
alternative is animating a width no stylesheet can know, which would drag a measurement into a
component that otherwise needs none. Every locale pays for this in the same place: the subscribe
button is occasionally a little wider than its own text.

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

### Subscribing takes 2.1 seconds, spent in three beats

That is a long time for an interface, and it only works as a **sequence**. Two seconds of
simultaneous motion reads as a page that has stopped responding; the same two seconds spent in
order reads as three things happening because of each other. Stretching one animation to fill the
budget was the failure mode to avoid — the fix for a transition that feels rushed is more stages,
not slower ones.

The timeline is one object in [sequence.ts](../apps/site/src/lib/newsletter/sequence.ts), handed
to the stylesheet as custom properties so no duration is written twice. Its tests hold two
invariants: the stages end exactly on the stated total, and none begins after the previous one has
finished — a gap is the reader watching nothing, which is the thing the sequence exists to avoid.

1. **The address is taken in.** A clip sweeps left to right across the masked form while a plain
   copy of what was typed lifts away beneath it, both in one grid cell so the mask arrives exactly
   where the field's text was. The reading is that the address was redacted rather than swapped,
   which is the truthful account of what happened to it.
2. **The button acknowledges it**, cooling out of ink and crossfading its copy, so one control is
   seen settling rather than a second appearing in its place.
3. **The line below settles**, the count clearing before the confirmation arrives in its place.

The sweep does not use the shared spring. A spring is a settle — it covers 97% of the distance in
the first half and leaves the rest of its stage with nothing visibly happening, which is exactly
what a stage this long cannot afford. It eases in and out instead, and the spring stays where
something is landing rather than travelling.

The unsubscribe control is last, and is absent rather than merely invisible until its beat. A
control that undoes what the reader is still watching happen has nothing to undo yet, and it
arrives directly below the button they just pressed, where a second click would otherwise land on
it.

### Unsubscribing runs the same beats backwards, in 900 milliseconds

Leaving is animated too -- a state that arrives deliberately and then vanishes was never one
thing changing -- but it is deliberately much shorter, and a test holds it under half the
forward total. Committing is worth dwelling on; leaving is not, and holding somebody inside an
animation while they are trying to go is the one place a long transition turns hostile.

The line under the pill leaves along the path it arrived by -- the confirmation and the
unsubscribe control both sink back down and fade, the exact reverse of their entrance -- so the
two states read as one thing changing rather than two that happen to share a row.

**The address does not reverse its sweep, though.** Redacting is something done to it, edge and
all; letting it go is not, and a mirrored wipe would say the site was busy taking the address back
rather than simply no longer holding it. It goes soft instead of directional: blurred out of
focus, drifting slightly, gone. It is also the longest stage of the four, which is what makes it
read as dissolving rather than being cleared away.

**The pill keeps showing the address it is undoing until that finishes**, so the record outlives
the request that cancelled it by exactly that long -- which is also why a second click is refused
for the duration rather than spent on a subscription that is already gone.

The submit button is the one thing that does not fade in at the end. It is the shape the chip has
just finished warming back into and arrives at the same ink it was handed; fading it would blink
the one element that was continuous across the swap. It springs up to size instead, because the
moment it becomes pressable again is worth marking and scale carries that without touching the
colour that made it continuous.

**A record read at mount animates nothing.** Someone returning to the page did not just do
anything, and replaying the confirmation would claim they had. The animation belongs to the
interaction, not to the state.

Reduced motion goes straight to the confirmed pill. The typed copy exists only to be animated
away, so without the animation it is not rendered at all rather than left behind for a timer to
remove.

This is a clip rather than the measured masks the Support rail uses. That geometry depends on the
rendered width of two labels and cannot be written down in advance; this one is always the whole
box, which puts it on the CSS side of the rule in [styling.md](styling.md).

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

## A read is counted by the browser that performed it

An article's read count follows its slug -- the site's own article path, such as
`development/rust-cargo-cranelift-tuning` -- and not any one of its nine language views. The
same article read in Japanese and in the original is the same article being read.

`POST /read` takes the slug in the body and answers with the count it now has. The slug is not
a path parameter because an article path contains a slash. The count is incremented and read in
one `INSERT ... ON CONFLICT DO UPDATE ... RETURNING` statement, so concurrent readers cannot be
handed the same number and an article's first read is the row's creation rather than a case of
its own.

**Which slugs exist is compiled into the Worker, not learned from requests.** The API is built
from the repository the markdown lives in, so `scripts/slugs.ts` walks `contents/` at build time
and emits the list; an unrecognised slug is a `404` and never reaches the database. The generated
module is committed, because the workspace type check, lint and test tasks run from the root and
never pass through this package's build -- a test regenerates the list and fails if it has gone
stale. Site and API are separate Workers Builds off the same commit, so a newly published article
is briefly unknown to the API. It is a few minutes, and it heals itself.

Deduplication is one Cloudflare rate limit of one count per IP per article per minute, with the
wider per-IP engagement allowance above it to bound somebody walking every slug in turn. **Being
deduplicated is answered with the current count, not with `429`.** The page still needs the number
to display, and a second look inside the minute is the same read rather than a failure.

The counts the four original articles carried in their frontmatter came from the old site and are
not reconstructible; they were seeded once as a data migration and the `views:` key was then
removed from the markdown. That edit did not touch `lastmod`, because nothing about the articles
changed.

## Engagement data is a persisted client query

The site fetches engagement state in the browser from the standalone API origin. TanStack Query
deduplicates the shared request used by Newsletter and Support. Its sync-storage persister writes
all successful TanStack Query query data into the single global `localStorage["cache"]` container
across page reloads. All queries remain fresh for five minutes; once stale, normal TanStack Query
refresh triggers update them. Unused in-memory data and cross-reload persistence may remain for up
to three days as a fallback while a stale query refreshes in the background.

The read count is a query rather than a mutation despite having an effect, because what the page
wants back is the number and the number is what has to survive a reload. That makes the count and
the request that produces it the same thing, so its refetch triggers are off: a read is somebody
opening the article, not somebody returning to the tab.

Mutations and errors are never persisted. Email addresses and cancellation tokens never enter the
query cache. The dedicated `localStorage["email"]` capability record is independent application
logic and is not part of TanStack Query cache eviction or hydration.
