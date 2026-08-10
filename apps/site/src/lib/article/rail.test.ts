import { describe, expect, it } from 'vitest';
import { homeCenter, railEndOffset, railTop } from './rail';

describe('the article side rail at the end of the article', () => {
	it('keeps its resting layout while the article end is below it', () => {
		expect(railEndOffset(600, 120, 500)).toBe(0);
		expect(railTop(600, 120, 500)).toBe(240);
		expect(homeCenter(108, 600, 120, 500)).toBe(108);
	});

	it('pins the rail bottom to the article end', () => {
		expect(railEndOffset(600, 120, 350)).toBe(-10);
		expect(railTop(600, 120, 350)).toBe(230);
		expect(homeCenter(108, 600, 120, 350)).toBe(98);
	});

	it('grows an expanded rail upward from the same pinned bottom', () => {
		expect(railTop(600, 120, 350)).toBe(230);
		expect(railTop(600, 286, 350)).toBe(64);
		expect(homeCenter(108, 600, 120, 350)).toBe(98);
		expect(homeCenter(108, 600, 286, 350)).toBe(-14.5);
		expect(railTop(600, 120, 350) - homeCenter(108, 600, 120, 350)).toBe(132);
		expect(railTop(600, 286, 350) - homeCenter(108, 600, 286, 350)).toBe(78.5);
	});

	it('lets both controls leave the viewport with the article', () => {
		expect(railTop(600, 120, -20)).toBe(-140);
		expect(homeCenter(108, 600, 120, -20)).toBe(-272);
	});
});
