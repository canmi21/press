import { URLS } from '@canmi/urls';

/**
 * Resolving an image reference into everything the markup needs, at build time.
 *
 * Articles reference an image by the content id of its original. The manifest holds the
 * variants derived from it, so the page can carry an exact `srcset` and its own placeholder
 * without the images being present in the repository or a single request being made to
 * discover their dimensions.
 */

const EXTENSION: Record<string, string> = {
	'image/avif': 'avif',
	'image/webp': 'webp',
	'image/png': 'png',
	'image/jpeg': 'jpg',
};

export type Resolved = {
	src: string;
	srcset: string;
	width: number;
	height: number;
	ratio: string;
	preview: string;
	/**
	 * What the image shows, from the manifest.
	 *
	 * Baked in at build time for the same reason the placeholder is: it belongs to the picture,
	 * so every article referencing it inherits the same words without repeating them, and a
	 * description written after the article still reaches it on the next build.
	 *
	 * Absent for an asset nobody has described yet. That is a gap `cms check` reports, not
	 * something to paper over with the filename.
	 */
	description?: string;
};

export type AssetManifest = {
	media: Record<
		string,
		{
			thumbhash: string;
			source: { width: number; height: number; ratio: string };
			variants: Record<string, { mime: string; width: number }>;
		}
	>;
};

export type MediaManifest = {
	media: Record<string, { description?: Record<string, { text: string }> }>;
};

/** Strip any extension an article wrote, leaving the content id. */
function idOf(reference: string): string {
	return (
		reference
			.split('/')
			.pop()
			?.replace(/\.[a-z0-9]+$/i, '') ?? reference
	);
}

function url(cid: string, mime: string): string {
	return `${URLS.apps.production.cdn}/image/${cid}.${EXTENSION[mime] ?? 'avif'}`;
}

/**
 * The variants of an image, ordered by width, as a `srcset` plus the largest as `src`.
 *
 * Returns null for a reference the manifest does not know, which is what happens to an
 * article written before its image was imported. The caller falls back to a plain `img` so
 * the page still renders rather than failing the build.
 */
export function createAssetResolver(
	assets: AssetManifest,
	media: MediaManifest,
	previews: ReadonlyMap<string, string>,
	descriptionLocale = 'en-US',
): (reference: string) => Resolved | null {
	return (reference) => {
		const id = idOf(reference);
		const asset = assets.media[id];
		if (!asset) return null;

		const variants = Object.entries(asset.variants).toSorted(([, a], [, b]) => a.width - b.width);
		const largest = variants.at(-1);
		if (!largest) return null;

		return {
			src: url(largest[0], largest[1].mime),
			srcset: variants.map(([cid, v]) => `${url(cid, v.mime)} ${v.width}w`).join(', '),
			// The original's dimensions, not the largest variant's: they share a ratio, and this is
			// what the browser needs to reserve the right box before anything loads.
			width: asset.source.width,
			height: asset.source.height,
			ratio: asset.source.ratio,
			preview: previews.get(asset.thumbhash) ?? '',
			// media.yaml owns these translations independently from article segments. Selecting the
			// matching value here makes each compiled view carry its own accessible fallback text.
			description: media.media[id]?.description?.[descriptionLocale]?.text,
		};
	};
}
