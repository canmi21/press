import { describe, expect, it } from 'vitest';
import { URLS } from '@canmi/urls';
import { packArticles, packPages, unpackArticles, unpackPages } from './packed';
import type { Article, ArticleView, PageView } from './types';
import type { LocaleCode } from '../locale';

const codes: LocaleCode[] = ['mw', 'en', 'de', 'es', 'fr', 'ja', 'ko', 'tw', 'zh'];

describe('packed virtual content', () => {
	it('stores one source payload for every locale that falls back to it', () => {
		const source = {
			meta: {
				title: 'Title',
				subtitle: 'Subtitle',
				description: 'Description',
				lang: 'en',
				created: '2026-01-01T00:00:00Z',
				lastmod: '2026-01-01T00:00:00Z',
			},
			toc: [{ slug: 'one', text: 'One', depth: 2 }],
			blocks: [{ type: 'prose' as const, html: '<p>Body</p>' }],
			feed: '<p>Body</p>',
			markdown: 'Body',
			text: 'Body',
		};
		const views = {} as Article['views'];
		const url = `${URLS.apps.production.site}/example`;
		for (const code of codes) {
			views[code] = {
				...source,
				code,
				languageTag: 'en-US',
				canonical: url,
				translationAvailable: code === 'mw',
			} satisfies ArticleView;
		}
		const article: Article = {
			...source,
			path: 'example',
			url,
			views,
			canonicalUrls: [],
			alternates: [],
		};

		const packed = packArticles([article]);
		const unpacked = unpackArticles(packed);
		expect(packed[0]?.contents).toHaveLength(0);
		expect(JSON.stringify(packed).length).toBeLessThan(JSON.stringify([article]).length / 2);
		expect(unpacked).toEqual([article]);
		expect(unpacked[0]?.views.de.blocks).toBe(unpacked[0]?.blocks);
	});

	it('deduplicates the shared view of a standalone page', () => {
		const view: PageView = { meta: { title: 'Home' }, blocks: [] };
		const views = Object.fromEntries(codes.map((code) => [code, view])) as Record<
			LocaleCode,
			PageView
		>;
		const page = { path: 'homepage', markdown: '# Homepage', views };

		const packed = packPages([page]);
		const unpacked = unpackPages(packed);
		expect(packed[0]?.contents).toHaveLength(1);
		expect(unpacked).toEqual([page]);
		expect(unpacked[0]?.views.de).toBe(unpacked[0]?.views.mw);
	});
});
