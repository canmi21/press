import { getArticle, getArticles } from '$lib/content';
import { error, redirect } from '@sveltejs/kit';
import { redirects } from 'virtual:redirects';
import type { EntryGenerator, PageServerLoad } from './$types';

export const prerender = true;

// Enumerate every article path so each is prerendered — no per-article route
// file. Redirect sources go through the same catch-all: their load() returns a
// redirect() that SvelteKit records and the active adapter emits in its own
// format.
export const entries: EntryGenerator = async () => {
	const articles = (await getArticles()).map((article) => ({ path: article.path }));
	const moved = Object.keys(redirects).map((from) => ({ path: from.replace(/^\//, '') }));
	return [...articles, ...moved];
};

export const load: PageServerLoad = async ({ params }) => {
	const target = redirects[`/${params.path}`];
	if (target) redirect(301, target);
	const article = await getArticle(params.path);
	if (!article) error(404, 'Not found');
	// Non-whitespace character count of the readable text (prose + headings, no
	// code/diagrams), computed at build time since this route is prerendered.
	const chars = article.text.replace(/\s/g, '').length;
	return { meta: article.meta, blocks: article.blocks, chars };
};
