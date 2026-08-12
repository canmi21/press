import { runInNewContext } from 'node:vm';
import { describe, expect, it, vi } from 'vitest';
import { articleHashScript, scheduleInitialHashJump } from './toc';

function runHashScript(type: NavigationTimingType, hash = '#target') {
	const replaceState = vi.fn();
	const window: { canmiArticleInitialHash?: string } = {};
	runInNewContext(articleHashScript, {
		history: { replaceState, state: { index: 1 } },
		location: { hash, pathname: '/article', search: '?lang=en' },
		performance: { getEntriesByType: () => [{ type }] },
		window,
	});
	return { replaceState, window };
}

describe('the article hash before hydration', () => {
	it('holds a fresh navigation at the article start', () => {
		const { replaceState, window } = runHashScript('navigate');

		expect(window.canmiArticleInitialHash).toBe('target');
		expect(replaceState).toHaveBeenCalledWith({ index: 1 }, '', '/article?lang=en');
	});

	it('leaves reload restoration to the browser', () => {
		const { replaceState, window } = runHashScript('reload');

		expect(window.canmiArticleInitialHash).toBeUndefined();
		expect(replaceState).not.toHaveBeenCalled();
	});
});

describe('the initial article hash jump', () => {
	it('starts after Motion restores the scroll position used for keyframe measurement', () => {
		let postRender: (() => void) | undefined;
		let nextFrame: (() => void) | undefined;
		const events: string[] = [];

		scheduleInitialHashJump(
			(callback) => (postRender = callback),
			(callback) => {
				nextFrame = callback;
				return 1;
			},
			vi.fn(),
			() => events.push('reset'),
			() => events.push('jump'),
		);

		// Starting while Motion still holds its measurement snapshot lets its restore cancel the jump.
		expect(events).toEqual([]);
		postRender?.();
		expect(events).toEqual(['reset']);
		nextFrame?.();
		expect(events).toEqual(['reset', 'jump']);
	});

	it('cancels a pending jump when its article is removed', () => {
		let postRender: (() => void) | undefined;
		const jump = vi.fn();
		const cancel = scheduleInitialHashJump(
			(callback) => (postRender = callback),
			vi.fn(() => 1),
			vi.fn(),
			vi.fn(),
			jump,
		);

		cancel();
		postRender?.();
		expect(jump).not.toHaveBeenCalled();
	});
});
