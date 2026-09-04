/**
 * The gestures shared by everything here that moves.
 *
 * Only the shape of a movement lives here, never the thing being moved: a height animation needs
 * rem conversion the site owns, and a sliding indicator needs geometry the CMS owns. What both
 * would otherwise write out separately is the timing.
 *
 * The site's disclosure was first and the CMS's tab indicator is the second, which is the pair
 * that moved this out of `apps/site/src/lib/client/collapse.ts`.
 */

/**
 * The move this scale is anchored to: a group of four articles folding, measured.
 *
 * Everything else is derived from it, so the one gesture that was already right stays exactly
 * where it was and the others move toward it.
 */
const REFERENCE_PIXELS = 178;
const REFERENCE_SECONDS = 0.222;

/**
 * Floors and ceilings on the derived time, in seconds.
 *
 * A few pixels would otherwise finish inside one frame and read as a jump, and a very tall panel
 * would take long enough that the press stops feeling connected to it.
 */
const MIN_SECONDS = 0.1;
const MAX_SECONDS = 0.45;

/** Under this a move is a flicker rather than a motion, so it plays at all only above it. */
export const NEGLIGIBLE_PIXELS = 0.5;

/**
 * CSS's own `ease`, which is the flattest of the curves that still starts promptly.
 *
 * The curve decides how much of a move lands in its worst frame, and that is what "smooth" means
 * at 60fps. Over the reference fold, measured as peak movement in a single frame against the
 * 13.7px a linear ramp would use: the ease-out quintic this replaces put 56.8px in its first frame
 * -- a third of the whole distance in 16ms, which is the graininess it was reporting rather than
 * any dropped frame. This curve peaks at 30.6px and opens with 11.1, so it is 46% flatter at the
 * top and still moves visibly on the first frame after the press.
 *
 * Curves that start at rest are flatter still and were rejected on that: two frames of stillness
 * after a click reads as the control not having heard it.
 */
const EASE = [0.25, 0.1, 0.25, 1] as const;

/**
 * How long a surface takes to travel a distance.
 *
 * **Not a constant speed, and that is the correction.** A fixed spring was worse -- it settles in
 * about the same time whatever it covers, so a short move was a slow one, measured at 0.19 pixels
 * a millisecond against 0.40 for a long one. But dividing by a constant speed overshoots the other
 * way: it makes a small panel finish in a tenth of a second, which reads as a flash rather than a
 * movement. Neither is how the eye reads travel.
 *
 * So the time grows with the square root of the distance. A move four times as long takes twice as
 * long, not four times: short ones get proportionally more time than their size, long ones less,
 * and the anchor above is untouched. Against the constant speed it replaces, at the same anchor:
 * 86px goes from 108ms to 154ms, and 600px from 750ms to 408ms.
 */
export function pressMotion(distancePixels: number): {
	duration: number;
	ease: readonly [number, number, number, number];
} {
	const scaled = REFERENCE_SECONDS * Math.sqrt(Math.abs(distancePixels) / REFERENCE_PIXELS);
	return { duration: Math.min(MAX_SECONDS, Math.max(MIN_SECONDS, scaled)), ease: EASE };
}

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
