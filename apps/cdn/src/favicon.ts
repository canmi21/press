import { Hono } from 'hono';
import { type Bindings, findOne, read, toResponse } from '@canmi/store';

/**
 * Serving favicons that the local CMS already fetched and synced.
 *
 * This worker never fetches from another site and never writes to the bucket. Both used to
 * happen here, and moving them into the CMS is what makes the mirror one-directional -- a
 * worker writing into `data/public`'s bucket would make the cloud authoritative for those
 * bytes. See spec/architecture.md.
 *
 * A miss is a 404, not a placeholder. The caller knows what it wants to draw when a site has
 * no icon, and an image response would deny it the chance to decide.
 */
const favicon = new Hono<{ Bindings: Bindings }>();

export function isValidHostname(value: string): boolean {
	if (value.length > 253 || value === 'localhost') return false;
	const labels = value.split('.');
	if (labels.length < 2) return false;
	// Four all-numeric labels is an address, not a site. `1.example.com` is fine.
	if (labels.length === 4 && labels.every((label) => /^\d{1,3}$/.test(label))) return false;
	return labels.every(
		(label) =>
			label.length > 0 &&
			label.length <= 63 &&
			!label.startsWith('-') &&
			!label.endsWith('-') &&
			/^[a-z0-9-]+$/.test(label),
	);
}

/**
 * Which stored variants to try, in order.
 *
 * Naming a tone means that tone or nothing. No silent substitution: a caller that asked for
 * dark and received light has no way to know it happened, and would draw a light icon on a
 * dark surface believing it had the right one. A 404 hands the choice back.
 *
 * With no tone named, either variant will do, so whichever exists is returned.
 */
export function candidates(tone: string | undefined): readonly string[] {
	if (tone === 'dark') return ['dark'];
	if (tone === 'light') return ['light'];
	return ['light', 'dark'];
}

favicon.get('/:domain', async (c) => {
	const domain = c.req.param('domain').toLowerCase();
	if (!isValidHostname(domain)) {
		return c.json({ error: 'invalid hostname' }, 400);
	}

	const [first, second] = candidates(c.req.query('tone'));

	// `??` short-circuits, so the second variant is only looked up when the first is absent.
	// That ordering is the whole point -- fetching both in parallel would cost two bucket
	// requests on every hit to save latency on the rarer miss.
	const found = (await lookup(c.env, domain, first)) ?? (await lookup(c.env, domain, second));

	if (!found) {
		return c.json({ error: 'not found' }, 404);
	}
	return toResponse(found);
});

async function lookup(env: Bindings, domain: string, variant: string | undefined) {
	if (!variant) return null;
	const key = await findOne(env, `favicon/${domain}/${variant}.`);
	return key ? read(env, key) : null;
}

export default favicon;
