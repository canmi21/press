import { readFileSync } from 'node:fs';
import { parse as parseYaml } from 'yaml';
import { describe, expect, it } from 'vitest';
import { assemble, similarity, type TranslationSidecar } from './assemble';
import { CANONICAL_SIMILARITY_THRESHOLD, indexingMetadata } from './indexing';
import { LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../locale';

const ARTICLES = [
	'architecture/compile-time-rendering',
	'development/rust-cargo-cranelift-tuning',
	'milestone/less-is-more',
	'mirror/less-than-an-hour',
];

function articleViews(path: string): Record<LocaleCode, string> {
	const article = new URL(`../../../../../contents/${path}.md`, import.meta.url);
	const sidecar = new URL(`../../../../../contents/${path}.i18n.yaml`, import.meta.url);
	const raw = readFileSync(article, 'utf8');
	const translations = parseYaml(readFileSync(sidecar, 'utf8')) as TranslationSidecar;
	return Object.fromEntries([
		['mw', raw],
		...Object.entries(PUBLIC_LANGUAGE).map(([code, locale]) => {
			const assembled = assemble(raw, translations, locale);
			if (assembled.missing.length > 0) {
				throw new Error(`${path}: ${locale} is missing ${assembled.missing.length} segments`);
			}
			return [code, assembled.raw];
		}),
	]) as Record<LocaleCode, string>;
}

describe('indexing metadata', () => {
	it('keeps the measured article fixtures on the intended sides of the threshold', () => {
		for (const path of ARTICLES) {
			const views = articleViews(path);
			expect(similarity(views.mw, views.zh), path).toBeGreaterThanOrEqual(
				CANONICAL_SIMILARITY_THRESHOLD,
			);
			expect(similarity(views.mw, views.en), path).toBeLessThan(CANONICAL_SIMILARITY_THRESHOLD);
		}
	});

	it('makes every hreflang name the canonical of the view it links to', () => {
		const views = articleViews(ARTICLES[0]);
		const indexing = indexingMetadata('/article', 'zh', views);

		expect(indexing.canonical.mw).toBe('/article');
		expect(indexing.canonical.zh).toBe('/article');
		expect(indexing.canonical.en).toBe('/article?lang=en');
		expect(indexing.alternates).toHaveLength(LOCALE_CODES.length + 1);
		for (const code of LOCALE_CODES) {
			const alternate = indexing.alternates.find((entry) => entry.code === code);
			expect(alternate?.href).toBe(indexing.canonical[code]);
			expect(alternate?.languageTag).toBe(code === 'mw' ? 'zh' : PUBLIC_LANGUAGE[code]);
		}
		expect(indexing.alternates.at(-1)).toEqual({
			code: 'x-default',
			languageTag: 'x-default',
			href: '/article',
		});
	});
});
