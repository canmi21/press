import { building, dev } from '$app/environment';
import { URLS } from '@canmi/urls';
import { handleErrorWithSentry, initCloudflareSentryHandle, sentryHandle } from '@sentry/sveltekit';
import type { Handle } from '@sveltejs/kit';
import { sequence } from '@sveltejs/kit/hooks';
import { getArticle, getPage } from '$lib/content';
import { themeScript } from '$lib/theme';
import { LANGUAGE_COOKIE_MAX_AGE, languageTag, privateHtml, resolveLocale } from '$lib/locale';
import { registerServerStrategy } from '$lib/locale/paraglide';

registerServerStrategy();

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

/**
 * A request for a document rather than a page.
 *
 * Every server endpoint here is named with an extension -- `/atom.xml`, `/sitemap.xml`,
 * `/robots.txt`, `/llms.txt`, `/licenses.txt`, `/licenses/full.txt` and the per-package texts
 * under it -- and no page is. That convention is what this reads, so the rule can be stated
 * the way it is actually meant: pages negotiate, documents resolve their own language or have
 * none.
 *
 * Written as an exception list rather than a list of pages because being multilingual is what
 * a page here *is*. A page missing from a list of pages serves the original to everybody and
 * says nothing about it, which is the failure that goes unnoticed; a document missing from
 * this one merely negotiates when it did not need to.
 */
const DOCUMENT_PATH = /\.[^./]+$/;

const pageHandle: Handle = async ({ event, resolve }) => {
	const { pathname } = event.url;
	// The homepage renders at /, but its source is contents/homepage.md, so the
	// markdown stays reachable at /homepage.md; bounce the bare page path to /.
	if (pathname === '/homepage') {
		return new Response(null, { status: 302, headers: { location: '/' } });
	}
	const path = pathname.replace(/^\//, '').replace(/\/$/, '');
	const article = getArticle(path);
	// Browser-facing HTML negotiates from every reader preference, which is every page. The
	// article lookup comes first so a slug that happens to carry a dot is still a page.
	const localeAware = article != null || !DOCUMENT_PATH.test(pathname);
	if (localeAware && !building) {
		const cookie = event.cookies.get('language');
		const code = resolveLocale({
			query: event.url.searchParams.get('lang'),
			cookie,
			acceptLanguage: event.request.headers.get('accept-language'),
		});
		event.locals.locale = {
			code,
			languageTag: languageTag(code, article?.meta.lang ?? 'en-US'),
		};
		// Rewrite even an unchanged value so cookies created before client-side switching was
		// introduced lose HttpOnly and become writable by the language controls.
		event.cookies.set('language', code, {
			path: '/',
			maxAge: LANGUAGE_COOKIE_MAX_AGE,
			sameSite: 'lax',
			httpOnly: false,
		});
	}
	const theme = event.cookies.get('theme');
	const response = await resolve(event, {
		transformPageChunk: ({ html }) =>
			hoistCharset(
				html
					.replace('%language.tag%', event.locals.locale?.languageTag ?? 'en-US')
					// The internal code for the client-side Paraglide strategy. The rendered
					// document is the authoritative result of the worker's full negotiation.
					.replace('%language.code%', event.locals.locale?.code ?? 'mw')
					.replace('%theme.class%', theme === 'dark' ? 'dark' : '')
					.replace('%theme.script%', themeScript),
			),
	});
	return privateHtml(response);
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
