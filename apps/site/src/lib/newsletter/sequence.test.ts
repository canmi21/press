import { describe, expect, it } from 'vitest';
import { SEQUENCE, SEQUENCE_TOTAL, sequenceStyle } from './sequence';

const stages = Object.values(SEQUENCE);

describe('the subscribe sequence', () => {
	it('ends exactly on its stated total', () => {
		// The number is quoted in spec/engagement.md and read by anyone timing the interaction, so a
		// stage extended without moving the ones after it must fail here rather than silently run long.
		expect(Math.max(...stages.map((stage) => stage.at + stage.for))).toBe(SEQUENCE_TOTAL);
	});

	it('never leaves the reader watching nothing', () => {
		// Every stage has to begin before the one before it has finished, or the transition reads as
		// having stalled rather than as one thing following another.
		const ordered = [...stages].sort((a, b) => a.at - b.at);
		for (const [index, stage] of ordered.entries()) {
			if (index === 0) continue;
			const previous = ordered[index - 1];
			if (!previous) continue;
			expect(stage.at).toBeLessThanOrEqual(previous.at + previous.for);
		}
	});

	it('publishes every stage to the stylesheet', () => {
		const style = sequenceStyle();
		for (const name of Object.keys(SEQUENCE)) {
			expect(style).toContain(`--${name}-at:`);
			expect(style).toContain(`--${name}-for:`);
		}
	});
});
