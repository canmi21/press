import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import { buildArticles, summaryFor, translatedRaws } from './articles';
import { sourceFingerprint, type SegmentSpan } from './assemble';

const ROOT = new URL('../../../../../../', import.meta.url);

describe('article widget build inputs', () => {
	it('watches embed records and compiles every widget in the real article', async () => {
		const crates = fileURLToPath(new URL('data/build/crates.json', ROOT));
		const repos = fileURLToPath(new URL('data/build/repos.json', ROOT));
		const tweets = fileURLToPath(new URL('data/build/twitter.json', ROOT));
		const { articles, files } = await buildArticles({
			contents: fileURLToPath(new URL('contents', ROOT)),
			messages: fileURLToPath(new URL('apps/site/messages', ROOT)),
			cdnUrl: URLS.apps.production.cdn,
			assets: fileURLToPath(new URL('data/metadata.json', ROOT)),
			media: fileURLToPath(new URL('data/media.yaml', ROOT)),
			segments: fileURLToPath(new URL('data/build/segments.json', ROOT)),
			crates,
			repos,
			tweets,
		});

		expect(files).toEqual(expect.arrayContaining([crates, repos, tweets]));
		const article = articles.find(
			(candidate) => candidate.path === 'development/rust-cargo-cranelift-tuning',
		);
		expect(article).toBeDefined();
		if (!article) throw new Error('missing rust-cargo-cranelift-tuning');
		for (const view of Object.values(article.views)) {
			// The initial document needs the ToC before browser-side heading measurement can run.
			expect(view.toc).toEqual(
				view.blocks.flatMap((block) =>
					block.type === 'heading'
						? [{ slug: block.slug, text: block.text, depth: block.depth }]
						: [],
				),
			);
			expect(view.blocks.filter(({ type }) => type === 'tokei')).toHaveLength(1);
			expect(view.blocks.filter(({ type }) => type === 'github')).toHaveLength(1);
			expect(view.blocks.filter(({ type }) => type === 'cargo')).toHaveLength(2);
			// Which provider answered is a fact about the last run, not about the build: regenerating
			// the summaries with another runner must not fail this test. That a view carries a
			// summary with provenance at all is the invariant.
			expect(view.summary?.provider).toEqual(expect.any(String));
			expect(view.summary?.provider).not.toEqual('');
		}
		expect(files).toContain(
			fileURLToPath(new URL('contents/development/rust-cargo-cranelift-tuning.summary.yaml', ROOT)),
		);

		const friends = articles.find(
			(candidate) => candidate.path === 'mirror/friends-come-in-phases',
		);
		expect(friends?.blocks).toContainEqual(
			expect.objectContaining({
				type: 'twitter',
				tweet: expect.objectContaining({ id: '2088060180290302397' }),
			}),
		);
	});
});

it('falls back the whole view when any live body translation is missing', () => {
	const raw = '---\ntitle: Source\nlang: en\n---\n\nFirst paragraph.\n\nSecond paragraph.\n';
	const encoder = new TextEncoder();
	const span = (id: string, source: string): SegmentSpan => {
		const start = raw.indexOf(source);
		return {
			id,
			start: encoder.encode(raw.slice(0, start)).length,
			end: encoder.encode(raw.slice(0, start + source.length)).length,
			fingerprint: sourceFingerprint(encoder.encode(source)),
			region: 'body',
		};
	};
	const spans = [span('first', 'First paragraph.'), span('second', 'Second paragraph.')];
	const result = translatedRaws(
		'contents/example.md',
		'example.md',
		raw,
		{
			segments: {
				first: {
					'de-DE': { text: 'Erster Absatz.' },
					'en-US': { text: 'First translated.' },
				},
				second: { 'en-US': { text: 'Second translated.' } },
			},
		},
		{ version: 3, articles: { 'example.md': spans } },
	);

	expect(result.translationAvailable.de).toBe(false);
	expect(result.raws.de).toBe(raw);
	expect(result.translatable.de).toBe(result.translatable.mw);
	expect(result.translationAvailable.en).toBe(true);
	expect(result.raws.en).toContain('First translated.\n\nSecond translated.');
});

it('falls back a missing localized summary to English and then to no summary', () => {
	const english = { text: 'English summary', provider: 'openai' };
	const german = { text: 'Deutsche Zusammenfassung', provider: 'openai' };

	expect(summaryFor({ 'en-US': english, 'de-DE': german }, 'de-DE')).toBe(german);
	expect(summaryFor({ 'en-US': english }, 'de-DE')).toBe(english);
	expect(summaryFor({}, 'de-DE')).toBeUndefined();
});
