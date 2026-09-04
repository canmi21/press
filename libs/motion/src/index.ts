/**
 * The gestures shared by everything here that moves.
 *
 * Only the shape of a movement lives here, never the thing being moved: a height animation needs
 * rem conversion the site owns, and a sliding indicator needs geometry the CMS owns. What both
 * would otherwise write out separately is the spring.
 *
 * The site's disclosure was first and the CMS's tab indicator is the second, which is the pair
 * that moved this out of `apps/site/src/lib/client/collapse.ts`.
 */

/**
 * How fast a surface travels, in CSS pixels per millisecond.
 *
 * **A press moves at a speed, not for a duration.** A spring with fixed stiffness settles in
 * roughly the same time whatever distance it covers, which means a short move is a slow one.
 * Measured on the article library, both on the same spring: a group of four folded 203px in
 * 512ms, and one article's panel opened 86px in 466ms -- 0.40 against 0.19 pixels a millisecond.
 * The small one was travelling at less than half the speed of the large one, and read as sluggish
 * next to it.
 *
 * So the distance sets the time. The number is the large move's own speed, which is the one that
 * already felt right; scaling from it leaves that gesture where it was and makes every shorter one
 * quicker rather than making the long one faster.
 */
export const TRAVEL_SPEED = 0.6;

/** Under this a move is a flicker rather than a motion, so it plays at all only above it. */
export const NEGLIGIBLE_PIXELS = 0.5;

/**
 * Floors and ceilings on the derived time, in seconds.
 *
 * A few pixels would otherwise finish inside one frame and read as a jump, and a very tall panel
 * would take long enough that the press stops feeling connected to it.
 */
const MIN_SECONDS = 0.12;
const MAX_SECONDS = 0.5;

/**
 * The transition for a surface answering a press, over a known distance.
 *
 * `visualDuration` is the time to visually arrive; the spring's remaining settle happens after it,
 * so this is the number a person actually perceives and the right one to derive from a speed.
 * `bounce` is low on purpose -- the spring this replaces was damped to 0.98 of critical, which is
 * firm and barely overshooting, and that is the character being kept.
 */
export function pressSpring(distancePixels: number): {
	type: 'spring';
	visualDuration: number;
	bounce: number;
} {
	const seconds = Math.abs(distancePixels) / TRAVEL_SPEED / 1000;
	return {
		type: 'spring',
		visualDuration: Math.min(MAX_SECONDS, Math.max(MIN_SECONDS, seconds)),
		bounce: 0.08,
	};
}

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
