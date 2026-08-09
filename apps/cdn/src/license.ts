import { Hono } from 'hono';
import { type Bindings, read, toResponse } from '@canmi/store';
import { FOREVER } from './cache';
import { licenseKeyFor, parseName, validatorFor } from './key';

/**
 * Serving the licence texts `cms licenses` publishes.
 *
 * The same shape as the image route and for the same reason: a caller names the object by its
 * content id alone, and the fanned-out key it is stored under is put together here. Before
 * this route existed the site linked the storage key verbatim, which made the bucket's layout
 * a public interface. See spec/architecture.md.
 *
 * Nothing is transcoded -- a licence is bytes that must be served exactly as the package
 * shipped them -- so this is a lookup and a validator, and no more.
 */
const license = new Hono<{ Bindings: Bindings }>();

/** The one object here that is named rather than addressed by its content. */
const FULL = 'full.txt';

license.get('/:name', async (c) => {
	const name = c.req.param('name');

	if (name === FULL) {
		const found = await read(c.env, `license/${FULL}`);
		// Left to the cache middleware rather than stamped: the aggregate is rewritten whenever
		// the dependency tree moves, so its name promises nothing about its bytes.
		return found ? toResponse(found) : c.json({ error: 'not found' }, 404);
	}

	const parsed = parseName(name);
	if (!parsed || parsed.extension !== 'txt') {
		return c.json({ error: 'not a content id' }, 400);
	}
	const { cid } = parsed;

	// Answered before the bucket is touched, exactly as the image route does: the id is a hash
	// of the bytes, so a client holding this tag holds these bytes.
	const tag = validatorFor(cid, 'txt');
	if (c.req.header('If-None-Match') === tag) {
		return new Response(null, { status: 304, headers: { ETag: tag } });
	}

	const stored = await read(c.env, licenseKeyFor(cid));
	if (!stored) {
		return c.json({ error: 'not found' }, 404);
	}

	const response = toResponse(stored);
	const headers = new Headers(response.headers);
	// Overwritten rather than deferred to, so the tag agrees with what the 304 above compares
	// against instead of with whatever R2 supplies for the stored object.
	headers.set('ETag', tag);
	headers.set('Cache-Control', FOREVER);
	return new Response(response.body, { status: response.status, headers });
});

export default license;
