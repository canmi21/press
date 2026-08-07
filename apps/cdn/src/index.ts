import { robotsTxt } from '@canmi/robots';
import { isDevHost, pickUrls } from '@canmi/urls';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { BRIEFLY, cacheControl } from './cache';
import favicon from './favicon';
import image from './image';
import github from './github';
import { type Bindings, read, toResponse } from '@canmi/store';

const app = new Hono<{ Bindings: Bindings }>();

// Any origin may read from the CDN. Everything it serves is already public -- an allowlist
// only decided which page could draw a picture anyone can fetch by URL, so it protected
// nothing and broke embedding, which is what a CDN is for. The methods stay read-only, so
// "any origin" grants exactly what a GET already grants.
//
// `allowHeaders` is absent by the same argument rather than by oversight. Without it a
// preflight reflects whatever was asked for, which grants nothing where there are no
// credentials to reach -- `*` and credentials cannot be combined at all. Naming headers
// instead means guessing which ones an embedder sends, `Range` among them, and breaking the
// ones guessed wrong. A scanner flagging the reflection has found a real pattern on a
// credentialed origin and nothing here.
//
// The API is the opposite and stays restricted: it answers about state, and there the origin
// is the difference between a reader and a caller.
app.use('*', cors({ origin: '*', allowMethods: ['GET', 'HEAD', 'OPTIONS'] }));
app.use('*', cacheControl);

app.get('/', (c) => {
	const urls = pickUrls(isDevHost(new URL(c.req.url).hostname));
	return c.redirect(`${urls.site}/?ref=cdn`, 302);
});

// Nothing here is disallowed, and the previous two attempts explain why.
//
// `Disallow: /` was right in principle -- a CDN has nothing worth indexing, and its URLs in
// search results compete with the pages that embed them. But OpenGraph cards have to be
// fetched by crawlers, so an exception was added as `Allow: /opengraph/`, and X still refused
// it. Twitterbot implements the original 1994 robots.txt draft, which has no `Allow` at all:
// it reads the disallow, never sees the exception, and skips the image.
//
// A per-agent block would work, but it would mean guessing which crawlers parse which decade
// of the format and revisiting that list forever. The thing being protected was mild -- some
// image URLs ranking on their own -- and the bandwidth is not ours to ration. So the policy
// stops being clever: everything is fetchable, and the file exists to say so rather than to
// leave crawlers guessing.
//
// Served with a short life of its own. The default for an unhashed path here is a week, which
// is far too long for the file that says what a crawler may do -- a policy correction should
// circulate in minutes.
app.get('/robots.txt', (c) => {
	c.header('Cache-Control', BRIEFLY);
	return c.text(robotsTxt({ disallow: [''] }));
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
