import type { R2Bucket } from '@cloudflare/workers-types';
import { Hono } from 'hono';

/**
 * `GET /image/{cid}` -- what is known about an asset.
 *
 * The same id that names bytes on the CDN names a record here, so a caller holding one
 * content id can ask either question without a second lookup to translate between them. The
 * record is written by `cms image` and published alongside the variants; this reads it back
 * verbatim rather than assembling anything, so there is one description of an asset and not
 * two that can disagree. See spec/architecture.md.
 */
type Bindings = { PUBLIC?: R2Bucket };

const CID = /^[0-9a-f]{32}$/;

const image = new Hono<{ Bindings: Bindings }>();

image.get('/:cid', async (c) => {
	const cid = c.req.param('cid').toLowerCase();
	if (!CID.test(cid)) {
		return c.json({ error: 'not a content id' }, 400);
	}
	if (!c.env.PUBLIC) {
		return c.json({ error: 'metadata store not bound' }, 503);
	}

	const object = await c.env.PUBLIC.get(`meta/${cid}.json`);
	if (!object) {
		return c.json({ error: 'not found' }, 404);
	}

	return new Response(object.body as unknown as ReadableStream, {
		headers: {
			'Content-Type': 'application/json; charset=utf-8',
			// The record can be rewritten when an asset is re-derived, so unlike the assets
			// themselves this is not immutable and gets a short life instead.
			'Cache-Control': 'public, max-age=300',
			ETag: object.httpEtag,
		},
	});
});

export default image;
