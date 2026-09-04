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

/**
 * The two curves an indicator travels on, and they describe different things.
 *
 * A bar crossing a strip is not a box growing, and one curve cannot say what it does. What it has
 * is a **centre** that moves and a **width** that adapts, and those are separate facts: the centre
 * is where the bar is, the width is how much of the label under it is covered. Driving offset and
 * width together conflates them into a rectangle redrawn at successive positions -- correct, and
 * inert.
 *
 * So the centre is animated on `CENTRE` and the width on `WIDTH`, over one duration, starting and
 * landing together. The bar reads as an object that moves and resizes at once rather than one that
 * is being retyped.
 *
 * `CENTRE` leaves decisively and settles, because the movement is the gesture. `WIDTH` is the
 * flatter of the two: a resize that raced the movement would look like the bar snapping to its new
 * size before it arrived, and one that lagged would leave it the wrong length at rest for a frame.
 */
const CENTRE = [0.32, 0.72, 0.24, 1] as const;
const WIDTH = [0.4, 0, 0.2, 1] as const;

/** A bar travelling between two tabs, rather than a surface opening. */
const TRAVEL_REFERENCE_PIXELS = 60;
const TRAVEL_REFERENCE_SECONDS = 0.24;
const TRAVEL_MIN_SECONDS = 0.18;
const TRAVEL_MAX_SECONDS = 0.38;

/**
 * How an indicator crosses to its new tab.
 *
 * Slower for its distance than a panel opening, and deliberately: a tab strip's hops are short
 * enough that the panel curve would be over before the movement could be read as one. Scaled by
 * the same square root for the same reason, with its own anchor because it is a different gesture.
 */
export function travelMotion(distancePixels: number): {
	duration: number;
	centre: readonly [number, number, number, number];
	width: readonly [number, number, number, number];
} {
	const scaled =
		TRAVEL_REFERENCE_SECONDS * Math.sqrt(Math.abs(distancePixels) / TRAVEL_REFERENCE_PIXELS);
	return {
		duration: Math.min(TRAVEL_MAX_SECONDS, Math.max(TRAVEL_MIN_SECONDS, scaled)),
		centre: CENTRE,
		width: WIDTH,
	};
}

/**
 * A control resizing because its content changed.
 *
 * A spring here where a panel gets a tween, and the overshoot is the reason rather than an
 * oversight: a button that swaps its label is a thing being pushed out or pulled in, and a little
 * give at the end is what makes it read as elastic instead of as a box being retyped. The site's
 * support actions have used this shape since before there was a module to keep it in -- stiffness
 * 420 against damping 28 is a damping ratio near 0.74, so it passes its target and comes back.
 *
 * The rest thresholds are stated in pixels because the value being animated is a width. Left at
 * the library's defaults of 0.01 they hold a spring open while it covers a hundredth of a pixel,
 * which is what made a fold feel slow enough to be worth measuring.
 */
export function contentMotion(): {
	type: 'spring';
	stiffness: number;
	damping: number;
	mass: number;
	restDelta: number;
	restSpeed: number;
} {
	return { type: 'spring', stiffness: 420, damping: 28, mass: 0.85, restDelta: 0.5, restSpeed: 10 };
}

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
