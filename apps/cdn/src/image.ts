import { Hono } from 'hono';
import { type Bindings, read, toResponse } from '@canmi/store';
import { FOREVER } from './cache';
import { keyFor, parseName, validatorFor } from './key';
import { DECODABLE, type Decodable, isEncodable, transcode } from './transcode';

/**
 * Serving content-addressed assets.
 *
 * A direct key lookup when the stored object is already the format asked for; otherwise the
 * stored object is decoded and re-encoded in this worker. The extension is the whole request:
 * there is no size parameter, so only sizes that were actually derived exist, and no caller
 * can invent dimensions to burn CPU on. See spec/architecture.md.
 */
const image = new Hono<{ Bindings: Bindings }>();

/** What a re-encoded response is served as. */
const TYPES: Record<string, string> = {
	avif: 'image/avif',
	webp: 'image/webp',
	jpeg: 'image/jpeg',
	jpg: 'image/jpeg',
	png: 'image/png',
};

image.get('/:name', async (c) => {
	const parsed = parseName(c.req.param('name'));
	if (!parsed) {
		return c.json({ error: 'not a content id' }, 400);
	}
	const { cid, extension } = parsed;

	// Answered before the bucket is touched. The id is a hash of the bytes, so a client
	// holding this tag holds these bytes; nothing on the far side could change that, and
	// reading the object to confirm it would only prove what the URL already stated.
	const tag = validatorFor(cid, extension);
	if (c.req.header('If-None-Match') === tag) {
		return new Response(null, { status: 304, headers: { ETag: tag } });
	}

	// A flat-colour original is stored as PNG rather than AVIF, so either may be a direct hit
	// and neither can be assumed to be the stored one.
	const stored = await read(c.env, keyFor(cid, extension));
	if (stored) {
		return finish(toResponse(stored), cid, extension);
	}

	if (!isEncodable(extension)) {
		return c.json({ error: 'not found' }, 404);
	}

	// `caches.default` is a Workers addition the DOM CacheStorage has no member for. Named
	// structurally rather than imported from @cloudflare/workers-types, because that module
	// declares its own Response, and pulling it in makes every handler here disagree with the
	// DOM Response Hono is typed against.
	const cache = (caches as unknown as { default: Cache }).default;
	const cached = await cache.match(c.req.raw);
	if (cached) {
		return cached;
	}

	const source = await findStored(c.env, cid);
	if (!source) {
		return c.json({ error: 'not found' }, 404);
	}

	const bytes = await transcode(await source.bytes, source.format, extension);
	const response = finish(
		new Response(bytes, { headers: { 'Content-Type': TYPES[extension] ?? 'image/jpeg' } }),
		cid,
		extension,
	);
	// Held at the edge so the decode is paid once per colo rather than once per reader. The
	// response is immutable, so there is nothing for a stale entry to be wrong about.
	c.executionCtx.waitUntil(cache.put(c.req.raw, response.clone()));
	return response;
});

/**
 * The stored object for an id, in whichever format it was published as.
 *
 * Probed rather than assumed. Most assets are AVIF, but a flat-colour screenshot is stored as
 * PNG because lossy coding is the wrong tool for it, and asking for the AVIF that was never
 * written is how those became a 404 instead of a conversion.
 */
async function findStored(
	env: Bindings,
	cid: string,
): Promise<{ bytes: Promise<ArrayBuffer>; format: Decodable } | null> {
	for (const format of DECODABLE) {
		const found = await read(env, keyFor(cid, format));
		if (found) {
			return { bytes: new Response(found.body).arrayBuffer(), format };
		}
	}
	return null;
}

/** Give a response the validator and lifetime that content addressing earns it. */
function finish(response: Response, cid: string, extension: string): Response {
	if (!response.ok) return response;
	const headers = new Headers(response.headers);
	// Overwritten rather than deferred to: R2 supplies its own ETag for the stored object,
	// which would answer a `.webp` request with the AVIF object's tag and disagree with what
	// the 304 path compares against.
	headers.set('ETag', validatorFor(cid, extension));
	headers.set('Cache-Control', FOREVER);
	return new Response(response.body, { status: response.status, headers });
}

export default image;
