/**
 * The gestures shared by everything here that moves.
 *
 * Only the shape of a movement lives here, never the thing being moved: a height animation needs
 * rem conversion the site owns, and a sliding indicator needs geometry the CMS owns. What both
 * would otherwise write out separately is the spring, and two copies of four numbers are two
 * things to keep in step with no way to tell later whether they were meant to be equal.
 *
 * The site's disclosure was first and the CMS's tab indicator is the second, which is the pair
 * that moved this out of `apps/site/src/lib/client/collapse.ts`.
 */

/** Firm and barely overshooting: a surface answering a press, not a thing being thrown. */
export const PRESS_SPRING = {
	type: 'spring' as const,
	stiffness: 420,
	damping: 38,
	mass: 0.9,
};

/** Below this a move reads as a flicker rather than a motion, so it is not worth playing. */
export const NEGLIGIBLE_PIXELS = 0.5;

export function prefersReducedMotion(): boolean {
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
