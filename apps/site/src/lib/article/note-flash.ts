/**
 * The brief selection-coloured fill a note jump lands with, in either direction.
 *
 * Arriving in the right scroll band is not the same as knowing what was arrived at: the noted
 * words may sit anywhere in their line, and a note is one small line among its siblings. The
 * URL never carries the move (spec/styling.md), so `:target` cannot say -- a class set here
 * does, and CSS decides what the light looks like and for how long.
 *
 * One flash at a time, module-wide: the two directions share the reader's attention, so a new
 * jump takes the light with it.
 */

/** As long as the CSS animation, plus a beat: the class must outlive what it plays. */
const FLASH_MS = 1900;
/** Give up waiting for the scroll and light up anyway: arrival detection is best effort. */
const ARRIVAL_MS = 3000;

const CLASS = 'note-return';

let flashed: HTMLElement | undefined;
let flashTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Add the flash class to `target` once the scroll that is carrying it settles.
 *
 * On arrival, not on departure: a long article's smooth scroll outlasts the animation, which
 * would play to an empty viewport. The browser's smooth-scroll duration is internal and
 * per-engine, so it cannot be computed up front, and Safari has no `scrollend` -- arrival is
 * read off the geometry instead: the target in the viewport and its position unchanged for a
 * frame, which is the scroll settling on it. Per-frame, so the light starts the frame the
 * move ends, with a cap so a scroll interrupted mid-flight still ends the wait. The timer
 * owns removal because reduced motion runs no animation and fires no `animationend`; one
 * mechanism for both.
 */
export function flashOnArrival(target: HTMLElement): void {
	if (flashTimer !== undefined) clearTimeout(flashTimer);
	flashed?.classList.remove(CLASS);
	flashed = target;
	const started = performance.now();
	let restingTop: number | undefined;
	const settled = () => {
		const { top, bottom } = target.getBoundingClientRect();
		const inView = top >= 0 && bottom <= window.innerHeight;
		const still = restingTop !== undefined && Math.abs(top - restingTop) < 1;
		restingTop = top;
		return inView && still;
	};
	const waitThenFlash = () => {
		if (flashed !== target) return;
		if (!settled() && performance.now() - started < ARRIVAL_MS) {
			requestAnimationFrame(waitThenFlash);
			return;
		}
		target.classList.add(CLASS);
		flashTimer = setTimeout(() => {
			target.classList.remove(CLASS);
			if (flashed === target) flashed = undefined;
			flashTimer = undefined;
		}, FLASH_MS);
	};
	requestAnimationFrame(waitThenFlash);
}
