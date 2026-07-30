import type { MiddlewareHandler } from 'hono';

/** One year. The longest value browsers honour, and what `immutable` implies. */
const IMMUTABLE = 31_536_000;
const WEEK = 604_800;
const ERROR = 300;

/**
 * Paths whose bytes never change under a given name, so a client may keep them forever.
 *
 * The font filenames carry no content hash, which makes this a promise rather than an
 * observation: re-subsetting a font has to produce a new filename, or everyone holding a
 * cached copy keeps the old one for a year. Inherited from the `_headers` file the old
 * static-assets deployment used. See spec/architecture.md.
 */
const IMMUTABLE_PREFIXES = ['/fonts/'];

export const cacheControl: MiddlewareHandler = async (c, next) => {
	await next();
	if (c.res.headers.has('Cache-Control')) return;

	const path = new URL(c.req.url).pathname;
	const ok = c.res.status >= 200 && c.res.status < 300;

	let value: string;
	if (ok && IMMUTABLE_PREFIXES.some((prefix) => path.startsWith(prefix))) {
		value = `public, max-age=${IMMUTABLE}, immutable`;
	} else if (ok) {
		value = `public, max-age=${WEEK}`;
	} else {
		// Errors are cached briefly rather than not at all: a missing favicon is requested on
		// every page view, and without this each one is a full trip to the origin.
		value = `public, max-age=${ERROR}`;
	}

	const headers = new Headers(c.res.headers);
	headers.set('Cache-Control', value);
	c.res = new Response(c.res.body, {
		status: c.res.status,
		statusText: c.res.statusText,
		headers,
	});
};
