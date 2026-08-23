import { describe, expect, it } from 'vitest';
import { contentRefreshQueue } from './content-refresh.ts';

const noop = () => undefined;

function deferred(): { promise: Promise<void>; resolve: () => void } {
	let resolve = noop;
	const promise = new Promise<void>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

describe('content refresh queue', () => {
	it('collapses a save burst into one final refresh and one HMR leader', async () => {
		const runs = [deferred(), deferred()];
		const segmentFlags: boolean[] = [];
		const queue = contentRefreshQueue((segments) => {
			segmentFlags.push(segments);
			return runs[segmentFlags.length - 1]?.promise ?? Promise.resolve();
		});

		const first = queue.request(true);
		const second = queue.request(false);
		expect(first.leader).toBe(true);
		expect(second.leader).toBe(false);
		expect(segmentFlags).toEqual([true]);

		runs[0].resolve();
		await Promise.resolve();
		await Promise.resolve();
		expect(segmentFlags).toEqual([true, false]);

		runs[1].resolve();
		await Promise.all([first.settled, second.settled]);
		expect(queue.active()).toBeUndefined();
	});

	it('carries a later markdown event into the final refresh', async () => {
		const runs = [deferred(), deferred()];
		const segmentFlags: boolean[] = [];
		const queue = contentRefreshQueue((segments) => {
			segmentFlags.push(segments);
			return runs[segmentFlags.length - 1]?.promise ?? Promise.resolve();
		});

		const first = queue.request(false);
		const second = queue.request(true);
		runs[0].resolve();
		await Promise.resolve();
		await Promise.resolve();
		expect(segmentFlags).toEqual([false, true]);

		runs[1].resolve();
		await Promise.all([first.settled, second.settled]);
	});
});
