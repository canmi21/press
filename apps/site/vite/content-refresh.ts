export type RefreshRequest = {
	leader: boolean;
	settled: Promise<void>;
};

/** Coalesce source events while preserving whether the final snapshot needs fresh segments. */
export function contentRefreshQueue(refresh: (segments: boolean) => Promise<void>): {
	request: (segments: boolean) => RefreshRequest;
	active: () => Promise<void> | undefined;
} {
	let active: Promise<void> | undefined;
	let dirty = false;
	let needsSegments = false;

	return {
		request(segments) {
			needsSegments ||= segments;
			if (active) {
				dirty = true;
				return { leader: false, settled: active };
			}

			const run = async (): Promise<void> => {
				do {
					dirty = false;
					const refreshSegments = needsSegments;
					needsSegments = false;
					// oxlint-disable-next-line no-await-in-loop -- an event during a refresh needs a later pass
					await refresh(refreshSegments);
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
