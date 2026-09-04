/**
 * The sliding indicator under a row of tabs.
 *
 * One element is moved and resized between the tabs rather than each tab drawing its own
 * underline, because two underlines cross-fading is a change of state and one bar travelling is
 * a change of place -- and a tab strip is about place. It also means the rail underneath can be
 * invisible: the moving bar is what says where you are, so nothing has to draw the track it
 * rides on.
 *
 * Shared here rather than written into the library page, because every page that grows a tab
 * strip wants the same gesture, and a second copy of the spring is two numbers to keep in step.
 * The spring itself lives in `@canmi/motion` with the site's disclosures, which is the pair that
 * put it there. See spec/architecture/cms.md.
 */

import { NEGLIGIBLE_PIXELS, PRESS_SPRING, prefersReducedMotion } from '@canmi/motion';
import { animate } from 'motion';

type Control = { stop: () => void };

const running = new WeakMap<HTMLElement, Control>();

/**
 * Put `indicator` under `active`, animating from wherever it currently is.
 *
 * Both offset and width are driven, because tabs are not equal widths and a bar that slid without
 * resizing would arrive at the right place the wrong length. The first placement is silent: there
 * is no previous tab to travel from, and a bar sweeping in from the left edge on load reads as a
 * loading animation rather than as a position.
 */
export function slideIndicator(indicator: HTMLElement, active: HTMLElement): void {
	const strip = indicator.parentElement;
	if (strip === null) return;

	const to = active.offsetLeft;
	const width = active.offsetWidth;
	const placed = indicator.dataset.placed !== undefined;

	running.get(indicator)?.stop();
	running.delete(indicator);

	const apply = (offset: number, size: number) => {
		indicator.style.transform = `translateX(${offset}px)`;
		indicator.style.width = `${size}px`;
	};

	if (!placed || prefersReducedMotion()) {
		indicator.dataset.placed = '';
		apply(to, width);
		return;
	}

	const from = indicator.offsetLeft + (getTranslateX(indicator) || 0);
	const fromWidth = indicator.offsetWidth;
	if (Math.abs(from - to) < NEGLIGIBLE_PIXELS && Math.abs(fromWidth - width) < NEGLIGIBLE_PIXELS) {
		apply(to, width);
		return;
	}

	// One animation drives both values off a single 0..1 progress, so the bar cannot arrive at its
	// destination before it has finished resizing.
	const control: Control = animate(0, 1, {
		...PRESS_SPRING,
		onUpdate: (progress: number) => {
			apply(from + (to - from) * progress, fromWidth + (width - fromWidth) * progress);
		},
	});
	running.set(indicator, control);
}

function getTranslateX(element: HTMLElement): number {
	const transform = getComputedStyle(element).transform;
	if (transform === 'none') return 0;
	const matrix = new DOMMatrixReadOnly(transform);
	return matrix.m41;
}
