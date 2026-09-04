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

import {
	NEGLIGIBLE_PIXELS,
	contentMotion,
	pressMotion,
	prefersReducedMotion,
	travelMotion,
} from '@canmi/motion';
import { remFromMeasuredPixels } from '@canmi/units';
import { animate } from 'motion';

type Control = { stop: () => void };

const running = new WeakMap<HTMLElement, Control>();
// The indicator runs two at once, one per edge.
const travelling = new WeakMap<HTMLElement, Control[]>();

/**
 * Put `indicator` under `active`, travelling from wherever it currently is.
 *
 * The bar is driven by its centre, not its left edge. Its centre crosses the strip on one curve
 * while its width adapts on another, over one duration, so the two start and land together --
 * the bar moves and resizes at once rather than sliding and then correcting its length. Left and
 * width are derived from that pair on each frame, never animated directly.
 *
 * The first placement is silent: there is no previous tab to travel from, and a bar sweeping in
 * from the left edge on load reads as a loading animation rather than as a position.
 */
export function slideIndicator(indicator: HTMLElement, active: HTMLElement): void {
	if (indicator.parentElement === null) return;

	const toWidth = active.offsetWidth;
	// A hidden page has no geometry, and the CMS opens on the Overview -- so the library's first
	// draw happens while its tabs measure zero. Placing the bar there would pin it to nothing and,
	// worse, mark it placed, so the real placement would then animate in from the left edge.
	if (toWidth === 0) return;
	const toCentre = active.offsetLeft + toWidth / 2;

	for (const control of travelling.get(indicator) ?? []) control.stop();
	travelling.delete(indicator);

	const paint = (centre: number, width: number) => {
		indicator.style.transform = `translateX(${remFromMeasuredPixels(centre - width / 2)})`;
		indicator.style.width = remFromMeasuredPixels(Math.max(0, width));
	};

	if (indicator.dataset.placed === undefined || prefersReducedMotion()) {
		indicator.dataset.placed = '';
		paint(toCentre, toWidth);
		return;
	}

	const fromWidth = indicator.offsetWidth;
	const fromCentre = indicator.offsetLeft + getTranslateX(indicator) + fromWidth / 2;
	if (
		Math.abs(fromCentre - toCentre) < NEGLIGIBLE_PIXELS &&
		Math.abs(fromWidth - toWidth) < NEGLIGIBLE_PIXELS
	) {
		paint(toCentre, toWidth);
		return;
	}

	// The distance the bar covers is its centre's, which is what a person follows.
	const { duration, centre, width } = travelMotion(toCentre - fromCentre);
	let liveCentre = fromCentre;
	let liveWidth = fromWidth;

	travelling.set(indicator, [
		animate(fromCentre, toCentre, {
			duration,
			ease: centre as never,
			onUpdate: (value: number) => {
				liveCentre = value;
				paint(liveCentre, liveWidth);
			},
		}),
		animate(fromWidth, toWidth, {
			duration,
			ease: width as never,
			onUpdate: (value: number) => {
				liveWidth = value;
				paint(liveCentre, liveWidth);
			},
		}),
	]);
}

function getTranslateX(element: HTMLElement): number {
	const transform = getComputedStyle(element).transform;
	if (transform === 'none') return 0;
	const matrix = new DOMMatrixReadOnly(transform);
	return matrix.m41;
}

/**
 * Open or close `panel`, animating between two measured heights.
 *
 * `height: auto` cannot be animated, so the natural height is measured by briefly setting it and
 * reading back, then handed to `auto` once the panel has arrived -- a panel pinned to a measured
 * number would stop following its own content when the window resizes. The same reasoning, and
 * the same spring, as the site's disclosures.
 *
 * Interruptible by construction: the running animation for this panel is stopped before another
 * starts, and the new one departs from wherever the old one had reached rather than from the
 * state it was travelling to. Clicking a header twice quickly reverses the motion instead of
 * queueing a second one.
 */
export function animateHeight(panel: HTMLElement, expanded: boolean): void {
	running.get(panel)?.stop();
	running.delete(panel);

	const from = panel.getBoundingClientRect().height;
	panel.style.height = 'auto';
	const to = expanded ? panel.getBoundingClientRect().height : 0;

	const settle = () => {
		panel.style.height = expanded ? 'auto' : '0rem';
		running.delete(panel);
	};

	if (prefersReducedMotion() || Math.abs(from - to) < NEGLIGIBLE_PIXELS) {
		settle();
		return;
	}

	panel.style.height = remFromMeasuredPixels(from);
	const control: Control = animate(from, to, {
		...pressMotion(to - from),
		onUpdate: (height: number) => {
			panel.style.height = remFromMeasuredPixels(Math.max(0, height));
		},
		onComplete: settle,
	});
	running.set(panel, control);
}


/**
 * Let a control resize itself, rather than jump, when what it says changes.
 *
 * The width is measured before and after `change` runs, so the caller only has to describe the new
 * content and never has to know how wide it will be. The element is pinned to where it was, driven
 * to where it is going, and then released back to `auto` -- pinning it permanently would stop it
 * following its own text at a different zoom or font size.
 *
 * Lifted from the site's support actions, which have done this since before there was anywhere to
 * share it from. What they add on top -- revealing masked copy in step with the width -- stays
 * theirs; what is common is measure, pin, travel, release.
 */
export function animateWidth(element: HTMLElement, change: () => void): void {
	const from = element.getBoundingClientRect().width;
	element.style.width = '';
	change();
	const to = element.getBoundingClientRect().width;

	running.get(element)?.stop();
	running.delete(element);

	if (prefersReducedMotion() || Math.abs(to - from) < NEGLIGIBLE_PIXELS) return;

	element.style.width = remFromMeasuredPixels(from);
	const control: Control = animate(from, to, {
		...contentMotion(),
		onUpdate: (width: number) => {
			element.style.width = remFromMeasuredPixels(Math.max(0, width));
		},
		onComplete: () => {
			// Only the animation still in charge may release the width: an interrupted one settling
			// late would hand the element back to `auto` in the middle of the move that replaced it.
			if (running.get(element) !== control) return;
			running.delete(element);
			element.style.width = '';
		},
	});
	running.set(element, control);
}
