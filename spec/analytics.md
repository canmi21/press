# Analytics

Three services currently count visits to the site: Cloudflare Insights, umami, and OpenPanel.
Nothing here argues for that number. The rules below are what they must agree on while it holds.

## Development loads the client and reports nothing

A dev session runs the same wiring production does, and sends no hits. Not loading the client
in development would be simpler and is wrong: the parts that break are the ones only exercised
by loading it -- a `data-track` attribute that names an event nobody registered, an outgoing
link handler that never binds. Those surface on a page you are looking at, or they surface in
production.

How each one is held to it, since no two offer the same switch:

| Service             | Mechanism                                    |
| ------------------- | -------------------------------------------- |
| umami               | `data-domains="canmi.net"` on the script tag |
| OpenPanel           | `filter: () => !dev`                         |
| Cloudflare Insights | `{#if !dev}` around the script               |

Cloudflare Insights is the odd one out and stays that way: its beacon takes no filter hook, so
the only control is whether the tag renders.

**For OpenPanel, `filter` is the only option that does this.** `send()` consults it before the
queue check and resolves immediately when it returns false, so the payload is dropped rather
than held. `disabled` looks like the same thing and is not -- it queues events and flushes the
backlog on `ready()`, which turns a dev session into a delayed batch of real-looking traffic.
Verified against SDK 1.3.1 by probing both branches: `dev` gives zero network calls and zero
queued events, production gives one call per event.

Every reporting path in that SDK -- `track`, `screenView`, `identify`, the group calls -- funnels
through the one `send()`, so the filter is a complete boundary and not a list to keep current.

## The client id is public, the client secret is not in the repo

An analytics client id ships in the browser bundle and is readable from devtools by anyone who
loads the page. It goes in plain source next to a note saying so, per the rule in
[toolchain.md](toolchain.md). What restricts its use is the CORS origin list configured on the
service, not obscurity.

The OpenPanel **client secret is absent from the repository entirely** rather than stored
encrypted and unused. It is only required for server-side events, and this site sends none. A
credential stored before anything needs it is one more thing to rotate and one more way to be
wrong about what is in use.

The derived `MCP_TOKEN` some services hand out is not stored either -- it is
`base64(client_id:client_secret)`, so keeping it is keeping a second copy of a credential that
will not be updated when the first one rotates. Recompute it when it is needed.

## Session replay stays off

It records what a visitor did rather than counting that they came. That is a different bargain
with the reader than page counting, and turning it on is a decision that belongs here in
writing, not a commented-out block in a config.
