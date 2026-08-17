import { read } from '@canmi/store';
import { Hono } from 'hono';
import type { Bindings } from './bindings';

/**
 * `GET /image/{cid}` -- what is known about an asset.
 *
 * The same id that names bytes on the CDN names a record here, so a caller holding one
 * content id can ask either question without a second lookup to translate between them. The
 * record is written by `cms image` and published alongside the variants; this reads it back
 * verbatim rather than assembling anything, so there is one description of an asset and not
 * two that can disagree. See spec/architecture/media.md.
 *
 * Reading goes through the same store as the CDN, so `mise run dev-api` answers from
 * `data/public` instead of needing `--remote` to reach a bucket only production writes.
 */
const CID = /^[0-9a-f]{32}$/;

const image = new Hono<{ Bindings: Bindings }>();

image.get('/:cid', async (c) => {
	const cid = c.req.param('cid').toLowerCase();
	if (!CID.test(cid)) {
		return c.json({ error: 'not a content id' }, 400);
	}

	const found = await read(c.env, `meta/${cid}.json`);
	if (!found) {
		return c.json({ error: 'not found' }, 404);
	}

	const headers = new Headers({
		'Content-Type': 'application/json; charset=utf-8',
		// The record can be rewritten when an asset is re-derived, so unlike the assets
		// themselves this is not immutable and gets a short life instead.
		'Cache-Control': 'public, max-age=300',
	});
	if (found.etag) headers.set('ETag', found.etag);
	return new Response(found.body, { headers });
});

export default image;
