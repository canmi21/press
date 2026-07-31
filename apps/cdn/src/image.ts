import { Hono } from 'hono';
import { type Bindings, read, toResponse } from '@canmi/store';

/**
 * Serving content-addressed assets.
 *
 * `/image/{cid}.avif` is a direct key lookup. Any other extension asks for the same object in
 * another format, which Cloudflare's image transformations produce from the stored AVIF.
 * The extension is the whole request: there is no size parameter, so only sizes that were
 * actually derived exist, and nobody can burn the monthly conversion quota by inventing
 * dimensions. See spec/architecture.md.
 */
const image = new Hono<{ Bindings: Bindings }>();

/** The stored format. Everything else is derived from it on demand. */
const STORED = 'avif';

/** What a request may ask to be converted into, and the type each is served as. */
const CONVERTIBLE: Record<string, string> = {
	webp: 'image/webp',
	jpeg: 'image/jpeg',
	jpg: 'image/jpeg',
	png: 'image/png',
};

/** Content ids are BLAKE3 truncated to 128 bits, hex encoded. */
const CID = /^[0-9a-f]{32}$/;

/** `image/{ab}/{cd}/{cid}.{ext}`, matching the layout apps/cms writes. */
export function keyFor(cid: string, extension: string): string {
	return `image/${cid.slice(0, 2)}/${cid.slice(2, 4)}/${cid}.${extension}`;
}

/** Split `{cid}.{ext}`, or null if it is not that shape. */
export function parseName(name: string): { cid: string; extension: string } | null {
	const dot = name.lastIndexOf('.');
	if (dot <= 0) return null;
	const cid = name.slice(0, dot).toLowerCase();
	const extension = name.slice(dot + 1).toLowerCase();
	return CID.test(cid) ? { cid, extension } : null;
}

image.get('/:name', async (c) => {
	const parsed = parseName(c.req.param('name'));
	if (!parsed) {
		return c.json({ error: 'not a content id' }, 400);
	}
	const { cid, extension } = parsed;

	// A flat-colour original is stored as PNG rather than AVIF, so both are direct lookups
	// before anything is converted.
	const stored = await read(c.env, keyFor(cid, extension));
	if (stored) {
		return toResponse(stored);
	}

	const target = CONVERTIBLE[extension];
	if (!target) {
		return c.json({ error: 'not found' }, 404);
	}

	// Checked before converting so a missing asset is a 404 rather than a failed
	// transformation, which would otherwise be reported as a quota problem.
	const source = await read(c.env, keyFor(cid, STORED));
	if (!source) {
		return c.json({ error: 'not found' }, 404);
	}

	return convert(new URL(c.req.url), cid, target);
});

/**
 * Re-encode a stored object through Cloudflare's image transformations.
 *
 * The source is named by URL rather than passed as bytes: the transformation runs in front of
 * a fetch, so it needs somewhere to fetch from, and the only public address of that object is
 * this worker's own AVIF route. That subrequest is served from cache after the first hit.
 *
 * Only the format changes -- no width, no quality, nothing a caller can vary.
 *
 * Exceeding the monthly quota does not degrade: new transformations fail while cached ones
 * keep serving. That is reported as 503 rather than dressed up as a missing image, because a
 * client told 404 will not come back and the image is not actually gone.
 */
async function convert(requested: URL, cid: string, target: string): Promise<Response> {
	const source = new URL(`/image/${cid}.${STORED}`, requested.origin);
	const response = await fetch(source, {
		cf: { image: { format: target.replace('image/', '') } },
	} as RequestInit);

	if (!response.ok) {
		return new Response(JSON.stringify({ error: 'conversion unavailable' }), {
			status: 503,
			headers: { 'Content-Type': 'application/json' },
		});
	}
	return new Response(response.body, { headers: { 'Content-Type': target } });
}

export default image;
