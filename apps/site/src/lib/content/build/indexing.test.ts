import { readFileSync } from 'node:fs';
import { parse as parseYaml } from 'yaml';
import { describe, expect, it } from 'vitest';
import { assemble, similarity, type SegmentLayout, type TranslationSidecar } from './assemble';
import { CANONICAL_SIMILARITY_THRESHOLD, indexingMetadata } from './indexing';
import { LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../../locale/index';

// Real articles, measured rather than invented: the threshold is a claim about actual
// translations and a fixture written to sit on one side of it would prove nothing.
//
// Nothing in the corpus sits above the threshold any more. Same-language views used to be
// verbatim copies of the source and scored 0.947 to 1.000; since they became genuine
// localisations (spec/i18n.md, "Same-language views localise the article too") the zh views
// of these mixed-language articles measure 0.570 to 0.682, so the fold rule stops applying --
// the outcome spec/locale.md anticipates for mixed originals. The above-threshold side is
// unmeasured until some view lands there again, the same way the English-source side went
// unmeasured when `homepage` stopped being translated.
const ARTICLES: { path: string; source: Exclude<LocaleCode, 'mw'> }[] = [
	{ path: 'architecture/compile-time-rendering', source: 'zh' },
	{ path: 'development/rust-cargo-cranelift-tuning', source: 'zh' },
	{ path: 'milestone/less-is-more', source: 'zh' },
	{ path: 'mirror/less-than-an-hour', source: 'zh' },
	{ path: 'mirror/friends-come-in-phases', source: 'zh' },
];
const layout = JSON.parse(
	readFileSync(new URL('../../../../../../data/build/segments.json', import.meta.url), 'utf8'),
) as SegmentLayout;

function articleContent(path: string): Record<LocaleCode, string> {
	const article = new URL(`../../../../../../contents/${path}.md`, import.meta.url);
	const sidecar = new URL(`../../../../../../contents/${path}.i18n.yaml`, import.meta.url);
	const raw = readFileSync(article, 'utf8');
	const sidecarData = parseYaml(readFileSync(sidecar, 'utf8')) as TranslationSidecar;
	const spans = layout.articles[`${path}.md`];
	if (!spans) throw new Error(`${path}: missing from data/build/segments.json`);
	let source = '';
	const translated = Object.entries(PUBLIC_LANGUAGE).map(([code, locale]) => {
		const assembled = assemble(raw, spans, sidecarData, locale, `${path}.md`);
		if (assembled.missing.length > 0) {
			throw new Error(`${path}: ${locale} is missing ${assembled.missing.length} segments`);
		}
		source = assembled.translatable.source;
		return [code, assembled.translatable.translated];
	});
	return Object.fromEntries([['mw', source], ...translated]) as Record<LocaleCode, string>;
}

describe('indexing metadata', () => {
	it('keeps the measured article fixtures on the intended sides of the threshold', () => {
		for (const { path, source } of ARTICLES) {
			const content = articleContent(path);
			const indexing = indexingMetadata('/article', content);
			// The source-language view still scores far above every real translation -- the gap
			// between a localisation and a translation is what the metric measures -- just no
			// longer high enough to fold into the Original.
			for (const code of Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]) {
				expect(similarity(content.mw, content[code]), `${path}:${code}`).toBeLessThan(
					CANONICAL_SIMILARITY_THRESHOLD,
				);
				expect(indexing.canonical[code], `${path}:${code}`).toBe(`/article?lang=${code}`);
				if (code !== source) {
					expect(
						similarity(content.mw, content[source]),
						`${path}: ${source} should outscore ${code}`,
					).toBeGreaterThan(similarity(content.mw, content[code]));
				}
			}
		}
	});

	it('makes every hreflang name a canonical without reading frontmatter language', () => {
		const first = ARTICLES[0];
		if (first === undefined) throw new Error('the tracked corpus has no articles to probe');
		const content = articleContent(first.path);
		const indexing = indexingMetadata('/article', content);

		expect(indexing.canonical.mw).toBe('/article');
		expect(indexing.canonical.zh).toBe('/article?lang=zh');
		expect(indexing.canonical.en).toBe('/article?lang=en');
		expect(indexing.canonicalUrls).toEqual([
			...new Set(LOCALE_CODES.map((code) => indexing.canonical[code])),
		]);
		expect(indexing.alternates).toHaveLength(Object.keys(PUBLIC_LANGUAGE).length + 1);
		expect(indexing.alternates.map(({ code }) => code)).not.toContain('mw');
		for (const code of Object.keys(PUBLIC_LANGUAGE) as Exclude<LocaleCode, 'mw'>[]) {
			const alternate = indexing.alternates.find((entry) => entry.code === code);
			expect(alternate?.href).toBe(indexing.canonical[code]);
			expect(alternate?.languageTag).toBe(PUBLIC_LANGUAGE[code]);
		}
		expect(indexing.alternates.at(-1)).toEqual({
			code: 'x-default',
			languageTag: 'x-default',
			href: '/article',
		});
	});
});
