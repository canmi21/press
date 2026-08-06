import { isDevHost } from '@canmi/urls';
import * as Sentry from '@sentry/cloudflare';
import app from './app';
import type { Bindings as WorkerBindings } from './bindings';

type Bindings = WorkerBindings & {
	/**
	 * Set with `wrangler secret put SENTRY_DSN`, never committed.
	 *
	 * A DSN is a URL that accepts events for one project. It is not a credential in the sense
	 * that reading it grants access to anything, but publishing it lets anyone fill the
	 * project with noise, and it differs per environment. Absent, error reporting is simply
	 * off -- which is what should happen in local development, and is why this is optional
	 * rather than a startup requirement.
	 */
	SENTRY_DSN?: string;
};

/**
 * Drop events that came from a development host.
 *
 * `wrangler dev` runs the same code as production, so without this the local server reports
 * every experiment into the same project as real traffic and buries it.
 */
const dropDev = <T extends { request?: { url?: string } }>(event: T) => {
	if (event.request?.url && isDevHost(new URL(event.request.url).hostname)) return null;
	return event;
};

export default Sentry.withSentry(
	(env: Bindings) => ({
		dsn: env.SENTRY_DSN,
		environment: 'production',
		beforeSend: dropDev,
		beforeSendTransaction: dropDev,
	}),
	// Hono exposes `fetch`, which is the whole of the handler contract a Worker needs.
	{ fetch: app.fetch },
);
