/**
 * The brief selection-coloured light a note jump lands with, in either direction.
 *
 * Arriving in the right scroll band is not the same as knowing what was arrived at: the noted
 * words may sit anywhere in their line, and a note is one small line among its siblings. The
 * URL never carries the move (spec/styling.md), so `:target` cannot say -- this does.
 *
 * The light is painted as its own translucent layer above the text, not as a background on
 * it. A background sits under the element's children, and any child that brings its own --
 * an inline code span is the ordinary case -- swallows the light exactly where it lands.
 * Painting on top cannot be covered by anything the content grows later, and the selection
 * colour already carries alpha in both themes, so the words stay readable through it. One
 * box per rendered line fragment, read off `getClientRects()` at the moment of arrival, so
 * the geometry is the text's own; boxes sit in document coordinates and stay glued to the
 * words under any further scrolling. A reflow mid-fade would strand them, but the fade is
 * two seconds long and a resize inside it is nobody's reading flow.
 *
 * One flash at a time, module-wide: the two directions share the reader's attention, so a
 * new jump takes the light with it.
 */

/** As long as the fade, plus a beat: the overlay must outlive what it plays. */
const FLASH_MS = 1900;
/** Give up waiting for the scroll and light up anyway: arrival detection is best effort. */
const ARRIVAL_MS = 3000;
/** The classed target still drives the marker-number tint in prose; see article.svelte. */
const CLASS = 'note-return';

/**
 * How a light that wraps closes its corners.
 *
 * `each` rounds every fragment into a finished box: the prose words, where every fragment is
 * wholly "the marked words". `ends` rounds only where the sentence starts and stops, leaving
 * square edges at the breaks: the note's line, one sentence whose break says it continues --
 * the same way a real drag-selection breaks, and this is its ink.
 */
export type FlashCorners = 'each' | 'ends';

let flashed: HTMLElement | undefined;
let overlay: HTMLDivElement | undefined;
let flashTimer: ReturnType<typeof setTimeout> | undefined;

function clear() {
	if (flashTimer !== undefined) clearTimeout(flashTimer);
	flashTimer = undefined;
	flashed?.classList.remove(CLASS);
	flashed = undefined;
	overlay?.remove();
	overlay = undefined;
}

function paint(target: HTMLElement, corners: FlashCorners): HTMLDivElement {
	const layer = document.createElement('div');
	// Absolute at the document root, so page coordinates are its coordinates. Inert to the
	// pointer, and above every block's own stacking (code blocks top out at 10) while staying
	// under the modal layer at 50 -- a landing light must never sit on a dialog.
	layer.style.cssText = 'position:absolute;left:0;top:0;z-index:30;pointer-events:none;';
	const em = Number.parseFloat(getComputedStyle(target).fontSize);
	const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
	// The tailoring the background version wore: snug over the glyphs, rounded at the corners
	// the semantics leave closed.
	const padX = 0.2 * em;
	const padY = 0.08 * em;
	const radius = 0.25 * rem;
	const rects = [...target.getClientRects()];
	for (const [index, rect] of rects.entries()) {
		const first = index === 0;
		const last = index === rects.length - 1;
		const startPad = corners === 'each' || first ? padX : 0;
		const endPad = corners === 'each' || last ? padX : 0;
		const rounded =
			corners === 'each'
				? `${radius}px`
				: `${first ? radius : 0}px ${last ? radius : 0}px ${last ? radius : 0}px ${first ? radius : 0}px`;
		const box = document.createElement('div');
		box.style.cssText =
			`position:absolute;background:var(--color-selection);border-radius:${rounded};` +
			`left:${rect.left + window.scrollX - startPad}px;top:${rect.top + window.scrollY - padY}px;` +
			`width:${rect.width + startPad + endPad}px;height:${rect.height + 2 * padY}px;`;
		layer.appendChild(box);
	}
	document.body.appendChild(layer);
	return layer;
}

/**
 * Light `target` once the scroll that is carrying it settles.
 *
 * On arrival, not on departure: a long article's smooth scroll outlasts the fade, which
 * would play to an empty viewport. The browser's smooth-scroll duration is internal and
 * per-engine, so it cannot be computed up front, and Safari has no `scrollend` -- arrival is
 * read off the geometry instead: the target in the viewport and its position unchanged for a
 * frame, which is the scroll settling on it. Per-frame, so the light starts the frame the
 * move ends, with a cap so a scroll interrupted mid-flight still ends the wait. The timer
 * owns removal because reduced motion runs no fade and fires no finish event; one mechanism
 * for both -- under reduced motion the light appears whole and leaves whole.
 */
export function flashOnArrival(target: HTMLElement, corners: FlashCorners): void {
	clear();
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
		overlay = paint(target, corners);
		// Whole on the first frame -- the instant of arrival is the message -- held, then
		// letting go slowly.
		if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			overlay.animate([{ opacity: 1 }, { opacity: 1, offset: 0.3 }, { opacity: 0 }], {
				duration: FLASH_MS - 100,
				easing: 'ease-out',
				fill: 'forwards',
			});
		}
		flashTimer = setTimeout(clear, FLASH_MS);
	};
	requestAnimationFrame(waitThenFlash);
}
