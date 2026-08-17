import { Hono } from 'hono';
import { type Bindings, read, toResponse } from '@canmi/store';
import { WEEKLY } from './cache';
import { cardKeys } from './key';

/**
 * Serving OpenGraph cards, one view per language.
 *
 * The address is the page's own path with `?lang=` on it -- `/opengraph/development/x.png?lang=ja`
 * -- and the view is a directory in the bucket. Same separation as everywhere else here: the URL
 * says what the reader wants, the key says where the bytes are, and the second is free to move.
 * See spec/architecture/data.md.
 *
 * A card is named by its slug rather than by a hash of its bytes, so unlike the licence texts
 * these cannot be immutable. The lifetime is a week, which is also how long X holds a card.
 */
const opengraph = new Hono<{ Bindings: Bindings }>();

opengraph.get('/*', async (c) => {
	const url = new URL(c.req.url);
	const keys = cardKeys(url.pathname, url.searchParams.get('lang'));
	if (!keys) {
		return c.json({ error: 'not found' }, 404);
	}

	// The view asked for, then the source view -- in that order and not in parallel. A page
	// whose card has not been rendered in its language should still advertise one, and a card
	// in the wrong language says more about the page than a blank rectangle does; but reading
	// both at once would spend a second bucket round trip on every request that hits the first.
	const [asked, fallback] = keys;
	const found =
		(asked && (await read(c.env, asked))) || (fallback && (await read(c.env, fallback)));
	if (!found) {
		return c.json({ error: 'not found' }, 404);
	}

	// `?lang=` is part of the URL, so caches already key on it; nothing needs a `Vary` here.
	const response = toResponse(found);
	const headers = new Headers(response.headers);
	headers.set('Cache-Control', WEEKLY);
	return new Response(response.body, { status: response.status, headers });
});

export default opengraph;
