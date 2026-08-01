import { dev } from '$app/environment';
import { URLS } from '@canmi/urls';
import { handleErrorWithSentry, initCloudflareSentryHandle, sentryHandle } from '@sentry/sveltekit';
import type { Handle } from '@sveltejs/kit';
import { sequence } from '@sveltejs/kit/hooks';
import { getArticle, getPage } from '$lib/content';
import { app } from '$lib/server/api';
import { themeScript } from '$lib/theme';

// Serve clean markdown at <url>.md (llms.txt convention) generically, without a
// per-target route — for articles and standalone pages (e.g. /homepage.md).
const markdownHandle: Handle = async ({ event, resolve }) => {
	const { pathname } = event.url;
	if (pathname.endsWith('.md')) {
		const slug = pathname.slice(1, -3);
		const markdown = (await getArticle(slug))?.markdown ?? getPage(slug)?.markdown;
		if (markdown) {
			return new Response(markdown, {
				headers: {
					'Content-Type': 'text/markdown; charset=utf-8',
					'Cache-Control': 'public, max-age=300, s-maxage=300',
				},
			});
		}
	}
	return resolve(event);
};

const pageHandle: Handle = ({ event, resolve }) => {
	const { pathname } = event.url;
	if (pathname.startsWith('/api')) {
		return app.fetch(event.request, event.platform?.env, event.platform?.context);
	}
	// The homepage renders at /, but its source is contents/homepage.md, so the
	// markdown stays reachable at /homepage.md; bounce the bare page path to /.
	if (pathname === '/homepage') {
		return new Response(null, { status: 302, headers: { location: '/' } });
	}
	const theme = event.cookies.get('theme');
	return resolve(event, {
		transformPageChunk: ({ html }) =>
			hoistCharset(
				html
					.replace('%theme.class%', theme === 'dark' ? 'dark' : '')
					.replace('%theme.script%', themeScript),
			),
	});
};

/**
 * Move the encoding declaration to the front of `<head>`.
 *
 * It lands second, not first: `sequence` nests handlers, so the one listed earliest transforms
 * last, and Sentry has to be listed first. Its trace tag therefore ends up ahead of this no
 * matter where the hoist runs. Reordering the sequence to win that would put error capture
 * inside the page handler to satisfy a lint.
 *
 * Second is enough. The standard asks for the declaration inside the first 1024 bytes; this
 * brings it from 629 to 386, and the one tag preceding it is ASCII hex that no decoder can
 * read two ways. In development Sentry is off entirely, so there it does land first.
 *
 * Nothing about this is explained in app.html: a comment there would be copied into every page
 * ever served, which is a strange place to keep notes for whoever edits the handler.
 */
function hoistCharset(html: string): string {
	const charset = /\s*<meta charset="[^"]*"\s*\/?>/i.exec(html);
	if (!charset || !html.includes('<head>')) return html;
	return html.replace(charset[0], '').replace('<head>', `<head>${charset[0].trim()}`);
}

export const handle = sequence(
	initCloudflareSentryHandle({
		dsn: URLS.external.sentry.site,
		enabled: !dev,
		environment: dev ? 'development' : 'production',
	}),
	sentryHandle(),
	markdownHandle,
	pageHandle,
);

export const handleError = handleErrorWithSentry();
