import { URLS } from '@canmi/urls';
import { Hono } from 'hono';

/**
 * Proxying GitHub avatars.
 *
 * Unlike favicons this stays a live fetch, because an avatar is per-user, changes whenever
 * its owner changes it, and there is no bounded set to prepare in advance. It writes nothing,
 * so the mirror stays one-directional: the rule is that the cloud never authors bytes in
 * `data/public`, not that it never makes a request. See spec/architecture.md.
 */
const github = new Hono();

github.get('/avatar/:idOrName', async (c) => {
	const value = c.req.param('idOrName');
	const width = c.req.query('width');
	const isNumeric = /^\d+$/.test(value);

	// Numeric ids resolve against the avatar host and take `s`; usernames go through
	// github.com and take `size`. Two spellings for one idea, both upstream's.
	const upstream = isNumeric
		? new URL(`${URLS.external.github.avatars}/u/${value}`)
		: new URL(`${URLS.external.github.web}/${value}.png`);
	if (width) upstream.searchParams.set(isNumeric ? 's' : 'size', width);

	const response = await fetch(upstream);
	return new Response(response.body, {
		status: response.status,
		headers: { 'Content-Type': response.headers.get('content-type') ?? 'image/png' },
	});
});

export default github;
