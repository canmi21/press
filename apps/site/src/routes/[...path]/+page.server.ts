import { getArticle } from '$lib/content';
import { error, redirect } from '@sveltejs/kit';
import { redirects } from 'virtual:redirects';
import type { Block } from '$lib/content/types';
import type { LocaleCode } from '$lib/locale';
import type { PageServerLoad } from './$types';

/** Characters in an article, ignoring whitespace. The one measure the header and a card share. */
function characters(text: string): number {
	return text.replace(/\s/g, '').length;
}

/**
 * Measure the articles an `::article` card points at.
 *
 * Done here rather than in the compiler because a card names a sibling, and the compiler sees
 * one article at a time -- by the time a request arrives every view is compiled and the number
 * is a lookup. A target that has since been renamed away simply gets no figure, which is the
 * card minus one line rather than a broken page.
 */
function measureArticleCards(blocks: Block[], code: LocaleCode): Block[] {
	return blocks.map((block) => {
		if (block.type !== 'article') return block;
		const target = getArticle(block.path);
		if (!target) return block;
		return { ...block, chars: characters(target.views[code].text) };
	});
}

export const prerender = false;

export const load: PageServerLoad = async ({ params, locals }) => {
	const target = redirects[`/${params.path}`];
	if (target) redirect(301, target);
	const article = getArticle(params.path);
	if (!article) error(404, 'Not found');
	const code = locals.locale?.code ?? 'mw';
	const view = article.views[code];
	const chars = characters(view.text);
	return {
		// The article's own path, not the requested one: it is what the read counter is keyed
		// by on the API side, and the two lists have to name the same thing.
		slug: article.path,
		meta: view.meta,
		toc: view.toc,
		blocks: measureArticleCards(view.blocks, code),
		summary: view.summary,
		chars,
		locale: {
			code: view.code,
			languageTag: view.languageTag,
			canonical: view.canonical,
			alternates: article.alternates,
			translationAvailable: view.translationAvailable,
		},
	};
};
