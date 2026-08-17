import type { MiddlewareHandler } from 'hono';

/** One year. The longest value browsers honour, and what `immutable` implies. */
const IMMUTABLE = 31_536_000;
const WEEK = 604_800;
const ERROR = 300;

/**
 * What a hashed name earns. Exported because a route that caches its own response has to
 * stamp this before the response is stored, which is earlier than this middleware runs.
 */
export const FOREVER = `public, max-age=${IMMUTABLE}, immutable`;

/** Cached briefly rather than not at all; see the reasoning at the use below. */
export const BRIEFLY = `public, max-age=${ERROR}`;

/**
 * What a name without a hash in it earns.
 *
 * An OpenGraph card is addressed by the slug of the page it belongs to, so editing a title
 * rewrites the bytes under an unchanged URL. A week is the accepted staleness and is also how
 * long X holds a card, so a shorter value would only cost fetches without shortening the wait.
 */
export const WEEKLY = `public, max-age=${WEEK}`;

/**
 * A path whose last segment is a content hash, so its bytes cannot change under that name.
 *
 * This is the whole basis for the year: an observation about the URL rather than a promise
 * anyone has to keep. Changing the bytes changes the hash and therefore the URL.
 */
const HASHED = /\/[0-9a-f]{32}\.[a-z0-9]+$/;

/**
 * Paths kept forever without a hash to justify it.
 *
 * Font filenames carry no content hash, which makes this a promise rather than an
 * observation: re-subsetting a font has to produce a new filename, or everyone holding a
 * cached copy keeps the old one for a year. Inherited from the `_headers` file the old
 * static-assets deployment used. See spec/architecture/delivery.md.
 */
const IMMUTABLE_PREFIXES = ['/fonts/'];

export const cacheControl: MiddlewareHandler = async (c, next) => {
	await next();
	if (c.res.headers.has('Cache-Control')) return;

	const path = new URL(c.req.url).pathname;
	const ok = c.res.status >= 200 && c.res.status < 300;

	let value: string;
	if (ok && (HASHED.test(path) || IMMUTABLE_PREFIXES.some((prefix) => path.startsWith(prefix)))) {
		value = FOREVER;
	} else if (ok) {
		value = `public, max-age=${WEEK}`;
	} else {
		// Errors are cached briefly rather than not at all: a missing favicon is requested on
		// every page view, and without this each one is a full trip to the origin. Briefly,
		// because unlike a hashed hit an error is a statement about right now -- the asset it
		// refers to may be published a minute later.
		value = BRIEFLY;
	}

	const headers = new Headers(c.res.headers);
	headers.set('Cache-Control', value);
	c.res = new Response(c.res.body, {
		status: c.res.status,
		statusText: c.res.statusText,
		headers,
	});
};
