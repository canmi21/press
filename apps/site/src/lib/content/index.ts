import { content } from 'virtual:articles';
import type { Article, Page } from './types';

// Article list (publish-date desc), shared by the sitemap, /llms.txt and
// per-article markdown.
export function getArticles(): Article[] {
	return content.articles;
}

export function getArticle(path: string): Article | undefined {
	return content.articlesByPath.get(path);
}

// Compiled standalone page by path (e.g. 'homepage'). Every locale view is baked
// into the virtual module alongside articles, so request handling is only a lookup.
export function getPage(path: string): Page | undefined {
	return content.pagesByPath.get(path);
}
