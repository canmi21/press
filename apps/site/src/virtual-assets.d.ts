declare module 'virtual:assets' {
	/** The merged asset manifest, baked in at build. Written by `cms image`. */
	export const assets: {
		version: number;
		generated: string;
		assets: Record<
			string,
			{
				type: string;
				created: string;
				updated: string;
				blake3: string;
				thumbhash: string;
				/** Written by `cms alt`; absent until an asset has been described. */
				description?: string;
				source: { mime: string; width: number; height: number; ratio: string; bytes: number };
				variants: Record<
					string,
					{ mime: string; width: number; height: number; quality: number; bytes: number }
				>;
			}
		>;
	};
}
