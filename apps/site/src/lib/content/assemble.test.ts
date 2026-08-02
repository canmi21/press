import { describe, expect, it } from 'vitest';
import { parse as parseYaml } from 'yaml';
import { assemble, similarity, sourceFingerprint, type SegmentSpan } from './assemble';
import { compile } from './compile';
import { CANONICAL_SIMILARITY_THRESHOLD } from './indexing';

it('assembles translations in article order while leaving code untouched', () => {
	const raw =
		'---\ntitle: Test\n---\n\n中文\n\nfirst line\n\n```ts\nconst x = 1;\n```\n\nlast line\n';
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
	const spans = [span('first', '中文'), span('second', 'first line'), span('last', 'last line')];
	const result = assemble(
		raw,
		spans,
		{
			segments: {
				first: { 'de-DE': { text: 'Chinesisch' } },
				second: { 'de-DE': { text: 'erste Zeile' } },
				last: { 'de-DE': { text: 'letzte Zeile' } },
			},
		},
		'de-DE',
		'contents/example.md',
	);
	expect(result.missing).toEqual([]);
	expect(result.raw).toContain('erste Zeile\n\n```ts\nconst x = 1;\n```\n\nletzte Zeile');
	expect(result.translatable).toEqual({
		source: '中文\n\nfirst line\n\nlast line',
		translated: 'Chinesisch\n\nerste Zeile\n\nletzte Zeile',
	});
});

it('rejects spans shifted by an insertion even when every translation still resolves', () => {
	const original = '---\ntitle: Test\n---\n\nfirst\n\nsecond\n';
	const encoder = new TextEncoder();
	const span = (id: string, source: string): SegmentSpan => {
		const start = original.indexOf(source);
		return {
			id,
			start: encoder.encode(original.slice(0, start)).length,
			end: encoder.encode(original.slice(0, start + source.length)).length,
			fingerprint: sourceFingerprint(encoder.encode(source)),
			region: 'body',
		};
	};
	const spans = [span('first-id', 'first'), span('second-id', 'second')];
	const sidecar = {
		segments: {
			'first-id': { 'de-DE': { text: 'erste' } },
			'second-id': { 'de-DE': { text: 'zweite' } },
		},
	};
	const edited = original.replace('\nfirst', '\ninserted\n\nfirst');

	expect(() => assemble(edited, spans, sidecar, 'de-DE', 'contents/shifted.md')).toThrow(
		'contents/shifted.md: stale segment layout',
	);
});

it('compiles a view with the source title when that locale has no title translation', async () => {
	const raw =
		'---\ntitle: Source title\nsubtitle: Source subtitle\ndescription: Source description\nlang: en\ncreated: 2026-08-02\nlastmod: 2026-08-02\n---\n\nBody\n';
	const encoder = new TextEncoder();
	const start = raw.indexOf(': Source title') + 1;
	const end = start + ' Source title'.length;
	const span: SegmentSpan = {
		id: 'title-id',
		start,
		end,
		fingerprint: sourceFingerprint(encoder.encode(raw.slice(start, end))),
		region: 'frontmatter',
	};
	const result = assemble(raw, [span], { segments: {} }, 'de-DE', 'contents/fallback.md');
	const yaml = result.raw.slice(4, result.raw.indexOf('\n---', 4));

	expect(result.missing).toEqual([]);
	expect(parseYaml(yaml)).toMatchObject({ title: 'Source title', lang: 'en' });
	const view = await compile(result.raw, '/fallback', {
		resolveAsset: () => null,
		highlight: async () => '',
	});
	expect(view.meta.title).toBe('Source title');
});

it('quotes translated frontmatter before splicing it into yaml', () => {
	const raw = '---\ntitle: Source title\nlang: en\n---\n\nBody\n';
	const encoder = new TextEncoder();
	const start = raw.indexOf(': Source title') + 1;
	const end = start + ' Source title'.length;
	const span: SegmentSpan = {
		id: 'title-id',
		start,
		end,
		fingerprint: sourceFingerprint(encoder.encode(raw.slice(start, end))),
		region: 'frontmatter',
	};
	const result = assemble(
		raw,
		[span],
		{ segments: { 'title-id': { 'de-DE': { text: 'Titel: "übersetzt"' } } } },
		'de-DE',
		'contents/translated.md',
	);
	const yaml = result.raw.slice(4, result.raw.indexOf('\n---', 4));

	expect(parseYaml(yaml)).toMatchObject({ title: 'Titel: "übersetzt"', lang: 'en' });
});

describe('similarity', () => {
	it('treats punctuation width and whitespace drift as the same prose', () => {
		expect(similarity('这是 "test" ?\n', '这是 “test” ？')).toBeGreaterThan(
			CANONICAL_SIMILARITY_THRESHOLD,
		);
	});

	it('keeps a genuine translation well below the canonical threshold', () => {
		expect(similarity('这是完全不同的一段文章。', 'This is a translated article.')).toBeLessThan(
			CANONICAL_SIMILARITY_THRESHOLD,
		);
	});
});
