declare module 'virtual:media' {
	/**
	 * What each picture is about, baked in at build. Written by `cms alt` and by hand.
	 *
	 * Separate from `virtual:assets` for the same reason the files are separate: one side is
	 * rebuilt from bytes whenever the encoder changes, and the other cannot be rebuilt at all.
	 */
	export const media: {
		version: number;
		media: Record<
			string,
			{
				category?: string;
				description?: Record<string, { text: string; review: boolean }>;
				tags?: string[];
			}
		>;
	};
}
