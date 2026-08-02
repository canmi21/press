import { getArticles, getPage } from '$lib/content';
import { homepageContent } from '$lib/server/homepage';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// HTML passes through the Worker so the site-wide private/no-store rule also applies here.
export const prerender = false;

// Landing page lists every article newest-first; getArticles() is already sorted
// by publish date. The bio prose is content-driven (contents/homepage.md, also
// reachable at /homepage.md). Only the fields the cards render are forwarded.
export const load: PageServerLoad = async ({ locals }) => {
	const page = getPage('homepage');
	if (!page) error(500, 'Missing contents/homepage.md');
	const code = locals.locale?.code ?? 'mw';
	const articles = (await getArticles()).map((article) => {
		const { meta, text } = article.views[code];
		return {
			title: meta.title,
			subtitle: meta.subtitle,
			created: meta.created,
			path: article.path,
			// First few prose-ish blocks, capped — the article icon maps their wrapped
			// line shape into its body bars on the client.
			paragraphs: text
				.split('\n\n')
				.map((p) => p.trim())
				.filter((p) => p.length > 16)
				.slice(0, 3)
				.map((p) => p.slice(0, 140)),
		};
	});
	return {
		articles,
		locale: { code },
		...homepageContent(page, code),
	};
};
