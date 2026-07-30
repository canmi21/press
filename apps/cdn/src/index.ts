import { robotsTxt } from '@canmi/robots';
import { URLS, isDevHost, pickUrls } from '@canmi/urls';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { cacheControl } from './cache';
import favicon from './favicon';
import github from './github';
import { type Bindings, read, toResponse } from './store';

const app = new Hono<{ Bindings: Bindings }>();

app.use(
	'*',
	cors({
		origin: [
			URLS.apps.production.site,
			URLS.apps.development.site,
			URLS.internal.app,
			URLS.internal.infra,
			URLS.internal.link,
		],
		allowMethods: ['GET', 'HEAD', 'OPTIONS'],
	}),
);
app.use('*', cacheControl);

app.get('/', (c) => {
	const urls = pickUrls(isDevHost(new URL(c.req.url).hostname));
	return c.redirect(`${urls.site}/?ref=cdn`, 302);
});

// A CDN has nothing worth indexing, and its URLs appearing in results would compete with the
// pages that embed them.
app.get('/robots.txt', (c) => c.text(robotsTxt({ disallow: ['/'] })));

app.route('/favicon', favicon);
app.route('/github', github);

// Everything else is a direct key lookup: fonts, the site's own icons, whatever else lands in
// data/public. The path is the key, because the bucket mirrors that directory exactly.
app.get('/*', async (c) => {
	const key = new URL(c.req.url).pathname.replace(/^\/+/, '');
	if (!key || key.includes('..')) {
		return c.json({ error: 'not found' }, 404);
	}
	const found = await read(c.env, key);
	if (!found) {
		return c.json({ error: 'not found' }, 404);
	}
	return toResponse(found);
});

export default app;
