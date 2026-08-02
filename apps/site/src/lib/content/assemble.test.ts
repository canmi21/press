import { describe, expect, it } from 'vitest';
import { assemble, similarity, sourceFingerprint, type SegmentSpan } from './assemble';
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
