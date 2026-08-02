import { describe, expect, it } from 'vitest';
import { assemble, segmentId, similarity, splitSegments } from './assemble';
import { CANONICAL_SIMILARITY_THRESHOLD } from './indexing';

it('uses the same normalised BLAKE3 address as the CMS', () => {
	expect(segmentId('one two\nthree')).toBe('b6434919a1cc9750bde65e1a4f81e056');
});

it('assembles translations in article order while leaving code untouched', () => {
	const raw = '---\ntitle: Test\n---\n\nfirst line\n\n```ts\nconst x = 1;\n```\n\nlast line\n';
	const [first, code, last] = splitSegments(raw);
	if (!first || !code || !last) throw new Error('fixture did not split');
	const result = assemble(
		raw,
		{
			segments: {
				[first.id]: { 'de-DE': { text: 'erste Zeile' } },
				[last.id]: { 'de-DE': { text: 'letzte Zeile' } },
			},
		},
		'de-DE',
	);
	expect(result.missing).toEqual([]);
	expect(result.raw).toContain('erste Zeile\n\n```ts\nconst x = 1;\n```\n\nletzte Zeile');
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
