import type { Article, Page } from './types.ts';

export type ContentSnapshot = {
	articles: Article[];
	pages: Page[];
	articlesByPath: Map<string, Article>;
	pagesByPath: Map<string, Page>;
};

export function contentSnapshot(articles: Article[], pages: Page[]): ContentSnapshot {
	return {
		articles,
		pages,
		articlesByPath: new Map(articles.map((article) => [article.path, article])),
		pagesByPath: new Map(pages.map((page) => [page.path, page])),
	};
}
