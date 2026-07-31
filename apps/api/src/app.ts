import { robotsTxt } from '@canmi/robots';
import type { Bindings } from '@canmi/store';
import { URLS, isDevHost, pickUrls } from '@canmi/urls';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import image from './image';

/**
 * The JSON API.
 *
 * Separate from `index.ts` so tests exercise the routes without going through the error
 * reporter, which needs a runtime environment none of them have.
 *
 * Alongside the asset metadata endpoint it carries the redirects and robots policy that any
 * host answering on a domain has to have.
 */
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
		allowMethods: ['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
	}),
);

// Browsers ask any origin they touch for /favicon.ico whether or not it serves pages. Sending
// them to the CDN answers it once rather than logging a 404 on every visit.
app.get('/favicon.ico', (c) => {
	const urls = pickUrls(isDevHost(new URL(c.req.url).hostname));
	return c.redirect(`${urls.cdn}/favicon.ico`, 301);
});

// The API root is not a page. `ref` marks where the visitor came from, so the site can tell
// this apart from someone typing the address.
app.get('/', (c) => {
	const urls = pickUrls(isDevHost(new URL(c.req.url).hostname));
	return c.redirect(`${urls.site}/?ref=api`, 302);
});

app.route('/image', image);

// An API has nothing to index, and its URLs surfacing in search results would compete with
// the pages that call it.
app.get('/robots.txt', (c) => c.text(robotsTxt({ disallow: ['/'] })));

export default app;
