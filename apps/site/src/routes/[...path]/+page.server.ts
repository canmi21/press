import { getArticle } from '$lib/content';
import { error, redirect } from '@sveltejs/kit';
import { redirects } from 'virtual:redirects';
import type { PageServerLoad } from './$types';

export const prerender = false;

export const load: PageServerLoad = async ({ params, locals }) => {
	const target = redirects[`/${params.path}`];
	if (target) redirect(301, target);
	const article = getArticle(params.path);
	if (!article) error(404, 'Not found');
	const view = article.views[locals.locale?.code ?? 'mw'];
	const chars = view.text.replace(/\s/g, '').length;
	return {
		// The article's own path, not the requested one: it is what the read counter is keyed
		// by on the API side, and the two lists have to name the same thing.
		slug: article.path,
		meta: view.meta,
		blocks: view.blocks,
		summary: view.summary,
		chars,
		locale: {
			code: view.code,
			languageTag: view.languageTag,
			canonical: view.canonical,
			alternates: article.alternates,
		},
	};
};
