export type SyncRequest = {
	leader: boolean;
	settled: Promise<void>;
};

/** Coalesce a burst of source events while still syncing once after the last event. */
export function segmentSyncQueue(sync: () => Promise<void>): {
	request: () => SyncRequest;
	active: () => Promise<void> | undefined;
} {
	let active: Promise<void> | undefined;
	let dirty = false;

	return {
		request() {
			if (active) {
				dirty = true;
				return { leader: false, settled: active };
			}

			const run = async (): Promise<void> => {
				do {
					dirty = false;
					// oxlint-disable-next-line no-await-in-loop -- an event during a sync needs a later pass
					await sync();
				} while (dirty);
			};
			active = run().finally(() => {
				active = undefined;
			});
			return { leader: true, settled: active };
		},
		active: () => active,
	};
}
