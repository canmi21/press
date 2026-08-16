import { dev } from '$app/environment';
import { OpenPanel } from '@openpanel/web';

/**
 * The OpenPanel client this site reports to.
 *
 * The id is compiled into the browser bundle and readable from devtools by anyone who loads the
 * page, so it is public by construction and is written here rather than encrypted -- storing it
 * as a secret would hide from a reader of this repository that it is already published, not
 * hide it from anybody else. See spec/toolchain.md. The client *secret* is the half that would
 * need protecting, and it is only required for server-side events, which this site does not
 * send; it is therefore absent from the repository entirely rather than stored unused.
 *
 * What actually restricts use of this id is the CORS origin list on the client in OpenPanel's
 * dashboard, not its being hard to find.
 */
const CLIENT_ID = 'fb80587a-c39c-4171-9e1f-c14f73d31bc1';

/**
 * Start reporting page views, outgoing link clicks, and `data-track` elements.
 *
 * Constructing the client is what arms it, so development is excluded by not constructing one
 * rather than by an option. The two options that look like an off switch are not: `disabled`
 * queues events until `ready()` and then sends the backlog, and `filter` drops payloads only
 * after the constructor has already patched `history.pushState` and bound a document-level
 * click listener. Neither leaves a dev session as it found it.
 *
 * `trackScreenViews` works on SvelteKit's client-side navigation because it wraps
 * `history.pushState`, which is what the router calls; there is no route hook to register.
 *
 * Session replay stays off. It records what real visitors do rather than counting that they
 * came, which is a different bargain with the reader and belongs to a decision of its own.
 */
export function registerAnalytics(): void {
	if (dev) return;

	// Constructing is the entire API here: the SDK arms its listeners in the constructor and
	// registers itself nowhere, so there is no instance to keep until something needs to report
	// a custom event by hand.
	// eslint-disable-next-line no-new
	new OpenPanel({
		clientId: CLIENT_ID,
		trackScreenViews: true,
		trackOutgoingLinks: true,
		trackAttributes: true,
	});
}
