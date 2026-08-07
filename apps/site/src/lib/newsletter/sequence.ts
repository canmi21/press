type Stage = { at: number; for: number };

/**
 * The subscribe transition, in milliseconds from the moment the API answers.
 *
 * It is a sequence rather than one slow fade. Held together, two and a bit seconds of simultaneous
 * motion reads as a page that has stopped responding; spent in order it reads as three things
 * happening because of each other -- the address is taken in, the button acknowledges it, and only
 * then does the page offer to undo it. See spec/engagement.md.
 */
export const SEQUENCE = {
	/** The address as typed, lifting away. */
	typed: { at: 0, for: 300 },
	/** The mask sweeping across it, left to right. */
	redact: { at: 0, for: 900 },
	/** The button cooling out of ink and swapping its copy. */
	chip: { at: 800, for: 700 },
	/** The subscriber count going, to clear the line under the pill. */
	count: { at: 1450, for: 150 },
	/** The confirmation arriving on that line. */
	row: { at: 1600, for: 350 },
	/** The unsubscribe control, last. */
	undo: { at: 1750, for: 350 },
} as const satisfies Record<string, Stage>;

export const SEQUENCE_TOTAL = 2100;

/** Every duration and offset the stylesheet needs, so the timeline has one home. */
export function sequenceStyle(): string {
	return Object.entries(SEQUENCE)
		.flatMap(([name, stage]) => [`--${name}-at:${stage.at}ms`, `--${name}-for:${stage.for}ms`])
		.join(';');
}
