declare module 'virtual:assets' {
	/** The merged asset manifest, baked in at build. Written by `cms image`. */
	export const assets: {
		version: number;
		/** When the manifest first appeared, and when anything in it last moved. */
		created: string;
		updated: string;
		media: Record<
			string,
			{
				type: string;
				created: string;
				updated: string;
				blake3: string;
				thumbhash: string;
				source: { mime: string; width: number; height: number; ratio: string; bytes: number };
				variants: Record<
					string,
					{ mime: string; width: number; height: number; quality: number; bytes: number }
				>;
			}
		>;
	};
}
