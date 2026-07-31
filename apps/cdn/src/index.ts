import { robotsTxt } from '@canmi/robots';
import { URLS, isDevHost, pickUrls } from '@canmi/urls';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { BRIEFLY, cacheControl } from './cache';
import favicon from './favicon';
import image from './image';
import github from './github';
import { type Bindings, read, toResponse } from '@canmi/store';

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
// pages that embed them. OpenGraph cards are the exception: X and Slack read robots.txt
// before fetching an og:image, so a blanket disallow hides the one thing a page advertises.
//
// Served with a short life of its own. The default for an unhashed path here is a week, which
// is far too long for the file that says what a crawler may do -- a policy correction should
// circulate in minutes.
app.get('/robots.txt', (c) => {
	c.header('Cache-Control', BRIEFLY);
	return c.text(robotsTxt({ allow: ['/opengraph/'], disallow: ['/'] }));
});

app.route('/favicon', favicon);
app.route('/image', image);
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
