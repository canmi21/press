import { describe, expect, it } from 'vitest';
import { ARTICLE_TITLE_BUDGET, fits, pixels } from './width.ts';

/**
 * Measured in the rendered page, and the same ten strings the CMS holds `width::pixels` to.
 *
 * This corpus is what keeps two implementations of one rule from drifting. A constant changed on
 * either side turns these red, which is the whole reason the numbers are written out here rather
 * than derived from the code under test.
 */
const MEASURED: [string, number][] = [
	['朋友只是同行一程', 128],
	['将渲染视为协议', 112],
	['プロトコルとしての描画', 176],
	['친구는 한 시절의 인연', 137],
	['Freunde auf Zeit', 125],
	['Se acabó holgazanear', 170],
	['Cargo stops slacking', 160],
	['El renderizado como protocolo', 234],
	['Le rendu, un protocole', 172],
	['Friends are only there for a season', 265],
];

describe('estimating how wide a line will draw', () => {
	it('never comes in under the rendered width', () => {
		for (const [text, measured] of MEASURED) {
			expect(pixels(text), text).toBeGreaterThanOrEqual(measured);
		}
	});

	it('stays within a fiftieth of the rendered width', () => {
		// A per-character table rather than an average, so the tolerance is what kerning and
		// rounding leave rather than what a blunt constant needed. It was a sixth.
		for (const [text, measured] of MEASURED) {
			expect((pixels(text) - measured) / measured, text).toBeLessThanOrEqual(0.02);
		}
	});

	it('charges a Han character the full square it draws', () => {
		expect(pixels('朋友只是同行一程')).toBe(16 * 8);
		expect(pixels('プロトコル')).toBe(16 * 5);
	});

	it('charges Hangul less than its width class would say', () => {
		// Two columns each by East Asian Width, but 13.84px on the page.
		expect(pixels('인연')).toBeLessThan(16 * 2);
		expect(pixels('인연')).toBe(14 * 2);
	});

	it('lets a short title through the article budget and keeps the longest title out', () => {
		expect(fits('Freunde auf Zeit', ARTICLE_TITLE_BUDGET)).toBe(true);
		expect(
			fits('Freundschaften gehören immer nur zu bestimmten Lebensphasen', ARTICLE_TITLE_BUDGET),
		).toBe(false);
	});
});
