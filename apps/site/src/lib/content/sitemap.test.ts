import { describe, expect, it } from 'vitest';
import { LOCALE_CODES, type LocaleCode } from '../locale';
import { indexingMetadata } from './indexing';
import { sitemapViews } from './sitemap';
import type { Alternate } from './types';

function alternateBytes(alternates: readonly Alternate[]): string {
	return alternates.map(({ languageTag, href }) => `${languageTag}\0${href}`).join('\n');
}

describe('sitemap language views', () => {
	it('uses the page-head hreflang set byte-for-byte for every distinct canonical URL', () => {
		const raws = Object.fromEntries(
			LOCALE_CODES.map((code) => [code, code === 'mw' || code === 'zh' ? 'source' : code]),
		) as Record<LocaleCode, string>;
		const indexing = indexingMetadata('https://example.com/article', raws);
		const pageHeadAlternates = indexing.alternates;
		const entries = sitemapViews({
			canonicalUrls: indexing.canonicalUrls,
			alternates: pageHeadAlternates,
		});

		expect(entries).toHaveLength(LOCALE_CODES.length - 1);
		for (const entry of entries) {
			expect(entry.alternates).toBe(pageHeadAlternates);
			expect(alternateBytes(entry.alternates)).toBe(alternateBytes(pageHeadAlternates));
			expect(entry.alternates.some(({ href }) => href === entry.loc)).toBe(true);
		}
	});
});
