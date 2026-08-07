import { describe, expect, it } from 'vitest';
import { REVERSE_TOTAL, SEQUENCE_TOTAL, sequenceStyle, TIMELINES } from './sequence';

describe.each(TIMELINES)('the $name timeline', ({ stages, total }) => {
	const all = Object.values(stages);

	it('ends exactly on its stated total', () => {
		// Both totals are quoted in spec/engagement.md and are what anyone timing the interaction
		// measures, so a stage extended without moving the ones after it must fail here rather than
		// silently run long.
		expect(Math.max(...all.map((stage) => stage.at + stage.for))).toBe(total);
	});

	it('never leaves the reader watching nothing', () => {
		// Every stage has to begin before the one before it has finished, or the transition reads as
		// having stalled rather than as one thing following another.
		const ordered = [...all].sort((a, b) => a.at - b.at);
		for (const [index, stage] of ordered.entries()) {
			const previous = ordered[index - 1];
			if (!previous) continue;
			expect(stage.at).toBeLessThanOrEqual(previous.at + previous.for);
		}
	});
});

describe('the two timelines together', () => {
	it('leaves faster than it arrives', () => {
		// Committing is worth dwelling on; leaving is not. If these ever meet, the undo has become
		// something the reader has to sit through.
		expect(REVERSE_TOTAL).toBeLessThan(SEQUENCE_TOTAL / 2);
	});

	it('publishes every stage of both to the stylesheet, without collision', () => {
		const style = sequenceStyle();
		for (const { stages, name } of TIMELINES) {
			const prefix = name === 'reverse' ? 'back-' : '';
			for (const stage of Object.keys(stages)) {
				expect(style).toContain(`--${prefix}${stage}-at:`);
				expect(style).toContain(`--${prefix}${stage}-for:`);
			}
		}
	});
});
