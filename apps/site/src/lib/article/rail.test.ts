import { describe, expect, it } from 'vitest';
import { homeCenter, homeRestingCenter, railEndOffset, railTop } from './rail';

// Half a one-line control plus the 1rem gap it keeps, which is what home-link.svelte measures.
const CLEARANCE = 26;

describe('the article side rail at the end of the article', () => {
	it('keeps its resting layout while the article end is below it', () => {
		expect(railEndOffset(600, 120, 500)).toBe(0);
		expect(railTop(600, 120, 500)).toBe(240);
		expect(homeCenter(108, 600, 120, 500, CLEARANCE)).toBe(108);
	});

	it('pins the rail bottom to the article end', () => {
		expect(railEndOffset(600, 120, 350)).toBe(-10);
		expect(railTop(600, 120, 350)).toBe(230);
		expect(homeCenter(108, 600, 120, 350, CLEARANCE)).toBe(98);
	});

	it('grows an expanded rail upward from the same pinned bottom', () => {
		expect(railTop(600, 120, 350)).toBe(230);
		expect(railTop(600, 286, 350)).toBe(64);
		expect(homeCenter(108, 600, 120, 350, CLEARANCE)).toBe(98);
		// The rail's top is at 157 here, well clear of a control resting at 108, so only the
		// article end moves it -- the same -93 the rail itself takes.
		expect(homeCenter(108, 600, 286, 350, CLEARANCE)).toBe(15);
		expect(railTop(600, 120, 350) - homeCenter(108, 600, 120, 350, CLEARANCE)).toBe(132);
		expect(railTop(600, 286, 350) - homeCenter(108, 600, 286, 350, CLEARANCE)).toBe(49);
	});

	it('lets both controls leave the viewport with the article', () => {
		expect(railTop(600, 120, -20)).toBe(-140);
		expect(homeCenter(108, 600, 120, -20, CLEARANCE)).toBe(-272);
	});
});

describe('where the return control rests', () => {
	// The fault this rule came from: an ordinary article left room to spare above its entries and
	// the control still sat 12px above the title, because the old rule halved the band whether or
	// not it needed to.
	it('rests level with the title while the entries are clear of it', () => {
		expect(homeRestingCenter(108, 612, 230, CLEARANCE)).toBe(108);
		expect(homeRestingCenter(108, 612, 344, CLEARANCE)).toBe(108);
	});

	it('rises once the entries reach it, and no sooner', () => {
		expect(homeRestingCenter(108, 612, 346, CLEARANCE)).toBe(107);
		expect(homeRestingCenter(108, 612, 400, CLEARANCE)).toBe(80);
		// An expanded rail is simply a taller one; nothing here knows which state it is in.
		expect(homeRestingCenter(108, 612, 549, CLEARANCE)).toBe(15.75);
	});

	it('takes the middle of the band once there is no clearance left to keep', () => {
		expect(homeRestingCenter(108, 612, 600, CLEARANCE)).toBe(3);
		expect(homeRestingCenter(108, 612, 700, CLEARANCE)).toBe(0);
	});

	// The two expressions meet where the band is twice the clearance, so the control slides
	// between them rather than jumping as a window is resized.
	it('is continuous where the two rules meet', () => {
		const meeting = 612 - 2 * (2 * CLEARANCE);
		expect(homeRestingCenter(108, 612, meeting, CLEARANCE)).toBe(CLEARANCE);
		expect(homeRestingCenter(108, 612, meeting - 2, CLEARANCE)).toBe(CLEARANCE + 1);
		expect(homeRestingCenter(108, 612, meeting + 2, CLEARANCE)).toBe(CLEARANCE - 0.5);
	});
});
