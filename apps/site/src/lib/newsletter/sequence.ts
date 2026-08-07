type Stage = { at: number; for: number };
type Timeline = Record<string, Stage>;

/**
 * Subscribing, in milliseconds from the moment the API answers.
 *
 * It is a sequence rather than one slow fade. Held together, two and a bit seconds of simultaneous
 * motion reads as a page that has stopped responding; spent in order it reads as three things
 * happening because of each other -- the address is taken in, the button acknowledges it, and only
 * then does the page offer to undo it. See spec/engagement.md.
 */
export const SEQUENCE = {
	/** The address as typed, lifting away. */
	typed: { at: 0, for: 400 },
	/**
	 * The mask sweeping across it, left to right. It takes the better part of the budget on
	 * purpose: it is the one stage that is about the address itself, and the beats after it are
	 * acknowledgements, which are cheaper to read.
	 */
	redact: { at: 0, for: 1500 },
	/** The button cooling out of ink and swapping its copy. */
	chip: { at: 1350, for: 450 },
	/** The subscriber count going, to clear the line under the pill. */
	count: { at: 1500, for: 150 },
	/** The confirmation arriving on that line. */
	row: { at: 1650, for: 250 },
	/** The unsubscribe control, last. */
	undo: { at: 1850, for: 250 },
} as const satisfies Timeline;

export const SEQUENCE_TOTAL = 2100;

/**
 * Unsubscribing, which runs the same beats backwards and is deliberately much shorter.
 *
 * Committing is worth dwelling on; leaving is not, and holding somebody inside an animation while
 * they are trying to go is the one place a long transition turns hostile.
 */
export const REVERSE = {
	/** The line under the pill leaving the way it came, and the control that was just used with it. */
	undo: { at: 0, for: 250 },
	/**
	 * The address dissolving. It is the longest stage of the four and it does not sweep: coming in
	 * was an act performed on the address, going out is it ceasing to be held.
	 */
	dissolve: { at: 0, for: 600 },
	/** The button warming back to ink and taking its verb back. */
	chip: { at: 300, for: 300 },
	/** The field returning, and the button springing back to something pressable. */
	form: { at: 600, for: 300 },
} as const satisfies Timeline;

export const REVERSE_TOTAL = 900;

export const TIMELINES = [
	{ name: 'sequence', stages: SEQUENCE as Timeline, total: SEQUENCE_TOTAL },
	{ name: 'reverse', stages: REVERSE as Timeline, total: REVERSE_TOTAL },
] as const;

/** Every duration and offset the stylesheet needs, so no timing is written twice. */
export function sequenceStyle(): string {
	return [stageStyle('', SEQUENCE), stageStyle('back-', REVERSE)].join(';');
}

function stageStyle(prefix: string, timeline: Timeline): string {
	return Object.entries(timeline)
		.flatMap(([name, stage]) => [
			`--${prefix}${name}-at:${stage.at}ms`,
			`--${prefix}${name}-for:${stage.for}ms`,
		])
		.join(';');
}
