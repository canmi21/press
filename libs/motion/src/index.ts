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
 * The small one was travelling at less than half the speed of the large one and read as sluggish.
 *
 * So the distance sets the time.
 */
export const TRAVEL_SPEED = 0.8;

/** Under this a move is a flicker rather than a motion, so it plays at all only above it. */
export const NEGLIGIBLE_PIXELS = 0.5;

/**
 * Floors and ceilings on the derived time, in seconds.
 *
 * A few pixels would otherwise finish inside one frame and read as a jump, and a very tall panel
 * would take long enough that the press stops feeling connected to it.
 */
const MIN_SECONDS = 0.12;
const MAX_SECONDS = 0.45;

/**
 * A strong ease-out: most of the distance early, decelerating into the target.
 *
 * This is the spring's character without its arithmetic -- it leans out hard and settles, but it
 * arrives, which a spring does not.
 */
const EASE = [0.22, 1, 0.36, 1] as const;

/**
 * The transition for a surface answering a press, over a known distance.
 *
 * A tween rather than a spring, and the reason is the whole point of this module. A spring has no
 * end: it approaches its target asymptotically and stops when the library decides it is close
 * enough, which for `motion` is a `restDelta` of 0.01 -- a hundredth of a pixel. Measured folding
 * a group, that tail ran 182ms of a 468ms move, 39% of the time spent covering the last three and
 * a half pixels. Nothing on screen changes during it and it reads as the panel being slow to let
 * go. Correcting the rest thresholds to half a pixel recovered only 74ms of it, because the rest
 * of the tail is the spring curve itself.
 *
 * With a duration the move ends when it says it will, which is what makes "a speed" a promise
 * rather than an average.
 */
export function pressMotion(distancePixels: number): {
	duration: number;
	ease: readonly [number, number, number, number];
} {
	const seconds = Math.abs(distancePixels) / TRAVEL_SPEED / 1000;
	return { duration: Math.min(MAX_SECONDS, Math.max(MIN_SECONDS, seconds)), ease: EASE };
}

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
