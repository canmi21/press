import type { Alternate, Article } from './types.ts';

export type SitemapView = { loc: string; alternates: Alternate[] };

/** Preserve the indexing decision and its exact alternate set for every distinct address. */
export function sitemapViews(
	article: Pick<Article, 'canonicalUrls' | 'alternates'>,
): SitemapView[] {
	return article.canonicalUrls.map((loc) => ({ loc, alternates: article.alternates }));
}
