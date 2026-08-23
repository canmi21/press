import { describe, expect, it } from 'vitest';
import { contentSnapshot } from './snapshot.ts';
import type { Article, Page } from './types.ts';

describe('content snapshot', () => {
	it('builds path indexes for one atomic article and page view', () => {
		const article = { path: 'one' } as Article;
		const page = { path: 'homepage' } as Page;
		const snapshot = contentSnapshot([article], [page]);

		expect(snapshot.articles).toEqual([article]);
		expect(snapshot.pages).toEqual([page]);
		expect(snapshot.articlesByPath.get('one')).toBe(article);
		expect(snapshot.pagesByPath.get('homepage')).toBe(page);
	});
});
