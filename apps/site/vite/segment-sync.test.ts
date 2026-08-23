import { describe, expect, it } from 'vitest';
import { segmentSyncQueue } from './segment-sync';

const noop = () => undefined;

function deferred(): { promise: Promise<void>; resolve: () => void } {
	let resolve = noop;
	const promise = new Promise<void>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

describe('segment sync queue', () => {
	it('collapses a save burst into one final sync and one HMR leader', async () => {
		const runs = [deferred(), deferred()];
		let index = 0;
		const queue = segmentSyncQueue(() => runs[index++]?.promise ?? Promise.resolve());

		const first = queue.request();
		const second = queue.request();
		expect(first.leader).toBe(true);
		expect(second.leader).toBe(false);
		expect(index).toBe(1);

		runs[0].resolve();
		await Promise.resolve();
		await Promise.resolve();
		expect(index).toBe(2);

		runs[1].resolve();
		await Promise.all([first.settled, second.settled]);
		expect(queue.active()).toBeUndefined();
	});

	it('starts a new leader after the previous burst settles', async () => {
		const queue = segmentSyncQueue(() => Promise.resolve());
		const first = queue.request();
		await first.settled;
		const second = queue.request();

		expect(first.leader).toBe(true);
		expect(second.leader).toBe(true);
		await second.settled;
	});
});
