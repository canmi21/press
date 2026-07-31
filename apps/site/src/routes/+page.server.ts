import { getArticles, getPage } from '$lib/content';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const prerender = true;

// Landing page lists every article newest-first; getArticles() is already sorted
// by publish date. The bio prose is content-driven (contents/homepage.md, also
// reachable at /homepage.md). Only the fields the cards render are forwarded.
export const load: PageServerLoad = async () => {
	const page = getPage('homepage');
	if (!page) error(500, 'Missing contents/homepage.md');
	const articles = (await getArticles()).map(({ meta, path, text }) => ({
		title: meta.title,
		subtitle: meta.subtitle,
		created: meta.created,
		path,
		// First few prose-ish blocks, capped — the article icon maps their wrapped
		// line shape into its body bars on the client.
		paragraphs: text
			.split('\n\n')
			.map((p) => p.trim())
			.filter((p) => p.length > 16)
			.slice(0, 3)
			.map((p) => p.slice(0, 140)),
	}));
	return {
		articles,
		title: page.meta.title ?? 'Canmi',
		description: page.meta.description ?? '',
		bio: page.blocks,
	};
};
