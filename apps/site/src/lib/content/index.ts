import { articles as builtArticles, pages as builtPages } from 'virtual:articles';
import type { Article, Page } from './types';

const articlesByPath = new Map(builtArticles.map((article) => [article.path, article]));
const pagesByPath = new Map(builtPages.map((page) => [page.path, page]));

// Article list (publish-date desc), shared by the sitemap, /llms.txt and
// per-article markdown.
export function getArticles(): Article[] {
	return builtArticles;
}

export function getArticle(path: string): Article | undefined {
	return articlesByPath.get(path);
}

// Compiled standalone page by path (e.g. 'homepage'). Every locale view is baked
// into the virtual module alongside articles, so request handling is only a lookup.
export function getPage(path: string): Page | undefined {
	return pagesByPath.get(path);
}
