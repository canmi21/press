/**
 * One canary, and the whole of core-js behind it.
 *
 * The list this replaced was hand-written, and nothing could say how long it should be: a
 * missing entry is a crash in somebody's browser, found the way the first one was, in Sentry.
 * Loading every stable polyfill removes the question rather than answering it -- there is no
 * longer a list to be wrong about.
 *
 * It costs nothing to be generous here because the import is dynamic. Measured on the built
 * client: the check compiles to 174 bytes in the eager entry, and core-js
 * lands in chunks that are not in the entry's static closure. A current browser reads two
 * `typeof`s and requests nothing.
 *
 * `toSorted` is the canary because it is the one that actually broke -- Chrome 110, Firefox 115
 * and Safari 16.0 shipped it, and a reader below that line reached production and threw. One
 * check rather than several because browser support is strongly ordered: a browser with this
 * has the decade of features before it, and a browser without it needs everything anyway.
 *
 * `stable` rather than `es`, which would leave out `URL` and `structuredClone`, or `actual`,
 * which adds proposals nothing here writes. See spec/compat.md.
 */
export async function prepareBrowserRuntime(): Promise<void> {
	if (typeof Array.prototype.toSorted === 'function') return;
	await import('core-js/stable');
}
