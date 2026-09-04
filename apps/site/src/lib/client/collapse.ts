/**
 * The one way a disclosure opens and closes on this site.
 *
 * Animating to `height: auto` is not possible, so the height is animated between two measured
 * numbers and handed back to `auto` at rest -- a panel that stayed pinned to a measured height
 * would stop following its own content when the window resizes or a font finishes loading.
 *
 * Shared rather than copied. A second disclosure with the same spring written out again is two
 * numbers to keep in step and no way to tell, later, whether they were meant to be equal or
 * merely happen to be. The code block was first; the collected notes are the second, and the
 * pair is what moved this out here.
 *
 * The search panel is the third and the one that is not a disclosure: nothing is pressed, the
 * results simply become other results and the box carries itself between the two heights. Its
 * target is measured differently for that reason -- see spec/search.md -- but the gesture is the
 * same one, so the spring is too.
 */

import { NEGLIGIBLE_PIXELS, PRESS_SPRING, prefersReducedMotion } from '@canmi/motion';
import { animate } from 'motion';
import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';

/**
 * Firm and barely overshooting: a panel answering a press, not a thing being thrown.
 *
 * Re-exported rather than declared: the CMS's tab indicator rides the same spring, so the numbers
 * moved to `@canmi/motion` when it became the second consumer. The name stays because this is
 * what the site's disclosures call it.
 */
export const COLLAPSE_SPRING = PRESS_SPRING;

export type AnimationControl = { stop: () => void };

/** Where a disclosure is, including the two states it is only passing through. */
export type CollapsePhase = 'collapsed' | 'collapsing' | 'expanded' | 'expanding';

export { prefersReducedMotion };

/**
 * Drive `element`'s height from where it is to `targetPixels`, calling `onSettle` when it lands.
 *
 * Returns the control to stop it, or nothing when no animation was needed -- reduced motion, or
 * a distance too small to see. In both of those cases `onSettle` has already run, so a caller
 * never has to ask which of the two happened.
 *
 * `onSettle` is handed the control that finished, and nothing when the landing was immediate.
 * A stopped animation is not guaranteed to stay silent, so a caller that has since started
 * another compares before acting: settling the wrong one would pin the panel to a height the
 * move it interrupted was travelling to.
 *
 * `onFrame` sees each height on its way, for anything that has to move in step with the panel
 * rather than merely after it. It is called with the same value the element is given, so a
 * caller can derive its own progress from the distance already covered and stay on the spring's
 * curve instead of guessing one.
 */
export function animateHeight(
	element: HTMLElement,
	targetPixels: number,
	onSettle: (finished?: AnimationControl) => void,
	onFrame?: (heightPixels: number) => void,
): AnimationControl | undefined {
	const currentPixels = element.getBoundingClientRect().height;
	element.style.height = remFromMeasuredPixels(currentPixels);

	if (prefersReducedMotion() || Math.abs(currentPixels - targetPixels) < NEGLIGIBLE_PIXELS) {
		onFrame?.(targetPixels);
		onSettle();
		return undefined;
	}

	const rootPixels =
		Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
		DEFAULT_PIXELS_PER_REM;
	let control: AnimationControl;
	control = animate(currentPixels, targetPixels, {
		...COLLAPSE_SPRING,
		onUpdate: (height) => {
			element.style.setProperty('height', remFromMeasuredPixels(Math.max(0, height), rootPixels));
			onFrame?.(height);
		},
		onComplete: () => onSettle(control),
	});
	return control;
}
