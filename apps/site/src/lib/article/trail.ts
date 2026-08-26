/**
 * Where a reader has been in this tab, so the article's Back control knows what "back" means.
 *
 * The site links articles to each other -- a `::article` card, a link in prose -- and every one
 * of them navigates in place. So Back cannot mean "home": a reader three articles deep who is
 * sent to the homepage loses the thread they were following. It has to mean one step up the way
 * they came.
 *
 * The browser's own history is not that. Its previous entry may be an anchor jump inside this
 * article, a locale switch, or a page on somebody else's site, none of which is a step in a
 * reading trail. What is wanted is the sequence of articles, and nothing but the trail itself
 * records it.
 *
 * `sessionStorage` is the right shelf for it because its lifetime is the question's lifetime: one
 * tab, surviving reloads, gone when the tab closes. Two tabs on the same site are two readers as
 * far as this is concerned, and they get two trails.
 */

export const TRAIL_KEY = 'trail';

/**
 * A trail, and the page it was recorded for.
 *
 * `at` is what makes it self-validating. A reload arrives looking exactly like a fresh visit --
 * no `from` -- so the record cannot be trusted on its own; it is trusted when it says it belongs
 * to the page now being shown, and discarded otherwise. Without that, a tab that walked a trail
 * and then opened an unrelated article would offer a way back to somewhere the reader never
 * came from.
 */
export type Trail = {
	at: string;
	paths: string[];
};

/**
 * How many steps back are kept.
 *
 * Not a memory concern -- it is that a trail is a way back, and one nobody will walk twelve
 * steps of is a record rather than a control. Cycles are truncated by `advance` before they can
 * reach this, so it bounds the honest case only.
 */
const MAX_DEPTH = 8;

const HOME = '/';

export function isTrail(value: unknown): value is Trail {
	if (typeof value !== 'object' || value === null) return false;
	const { at, paths } = value as Partial<Trail>;
	return (
		typeof at === 'string' &&
		Array.isArray(paths) &&
		paths.every((path) => typeof path === 'string')
	);
}

/**
 * The trail after a navigation, given whatever was stored for the page being left.
 *
 * `stored` is only believed when it belongs to `from`; anything else is a trail for a page this
 * navigation did not start on. Passing `from` as undefined is a full page load, where the only
 * trail that survives is the one already claiming this page -- that is the reload case.
 */
export function advance(stored: Trail | undefined, to: string, from?: string): Trail {
	if (from === undefined) {
		return stored?.at === to ? stored : { at: to, paths: [] };
	}
	if (from === to) return { at: to, paths: stored?.at === from ? stored.paths : [] };

	const paths = stored?.at === from ? stored.paths : [];
	// Arriving somewhere already on the trail is a step back, however it was taken -- the Back
	// control, the browser's button, or a link that happens to point there. Cutting to that
	// point rather than appending is what stops A -> B -> A -> B from growing without end, and
	// it costs nothing to state once instead of special-casing each way of going backwards.
	const seen = paths.lastIndexOf(to);
	if (seen >= 0) return { at: to, paths: paths.slice(0, seen) };

	return { at: to, paths: [...paths, from].slice(-MAX_DEPTH) };
}

/** Where Back goes from here: one step up the trail, or home when there is none. */
export function backTarget(trail: Trail | undefined): string {
	return trail?.paths.at(-1) ?? HOME;
}

export function readTrail(storage: Pick<Storage, 'getItem'>): Trail | undefined {
	const raw = storage.getItem(TRAIL_KEY);
	if (!raw) return undefined;
	try {
		const parsed: unknown = JSON.parse(raw);
		return isTrail(parsed) ? parsed : undefined;
	} catch {
		// Another tab, an older shape, or somebody's devtools. A trail is a convenience and its
		// absence is already handled, so a bad one is dropped rather than reported.
		return undefined;
	}
}

export function writeTrail(storage: Pick<Storage, 'setItem'>, trail: Trail): void {
	storage.setItem(TRAIL_KEY, JSON.stringify(trail));
}
