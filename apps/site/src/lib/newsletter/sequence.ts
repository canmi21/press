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
	typed: { at: 0, for: 350 },
	/** The mask sweeping across it, left to right. */
	redact: { at: 0, for: 1200 },
	/** The button cooling out of ink and swapping its copy. */
	chip: { at: 1050, for: 600 },
	/** The subscriber count going, to clear the line under the pill. */
	count: { at: 1500, for: 150 },
	/** The confirmation arriving on that line. */
	row: { at: 1650, for: 300 },
	/** The unsubscribe control, last. */
	undo: { at: 1800, for: 300 },
} as const satisfies Timeline;

export const SEQUENCE_TOTAL = 2100;

/**
 * Unsubscribing, which runs the same beats backwards and is deliberately much shorter.
 *
 * Committing is worth dwelling on; leaving is not, and holding somebody inside an animation while
 * they are trying to go is the one place a long transition turns hostile.
 */
export const REVERSE = {
	/** The control that was just used, going with the state it undid. */
	undo: { at: 0, for: 200 },
	/** The mask retreating the way it arrived, right to left. */
	unredact: { at: 100, for: 450 },
	/** The button warming back to ink and taking its verb back. */
	chip: { at: 200, for: 400 },
	/** The field returning, ready to be typed in again. */
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
