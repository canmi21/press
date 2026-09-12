/**
 * Resolving an image reference into everything the markup needs, at build time.
 *
 * Articles reference an image by the content id of its original. The manifest holds the
 * variants derived from it, so the page can carry an exact `srcset` and its own placeholder
 * without the images being present in the repository or a single request being made to
 * discover their dimensions.
 *
 * A diagram is resolved here too. It is not an asset -- an article carries its source inline --
 * but it is the same shape of question, asked of the same kind of record: what does this picture
 * say, in this view's language.
 */

import { sourceFingerprint } from './assemble.ts';

/**
 * What a published variant's file is called, keyed by what it holds.
 *
 * Exported so a test can hold it to the Rust side that names the files. The two are one fact in
 * two languages: apps/cms writes the name, this rebuilds it, and a disagreement shows up only as
 * a redirect nobody notices.
 */
export const EXTENSION: Record<string, string> = {
	'image/avif': 'avif',
	'image/webp': 'webp',
	'image/png': 'png',
	// `jpeg`, matching what apps/cms names the file. The CDN redirects `.jpg` away, and a link
	// built here should not be the thing taking that hop.
	'image/jpeg': 'jpeg',
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

function url(cdnUrl: string, cid: string, mime: string): string {
	return `${cdnUrl}/image/${cid}.${EXTENSION[mime] ?? 'avif'}`;
}

/**
 * The variants of an image, ordered by width, as a `srcset` plus the largest as `src`.
 *
 * Returns null for a reference the manifest does not know, which is what happens to an
 * article written before its image was imported. The caller falls back to a plain `img` so
 * the page still renders rather than failing the build.
 */
/**
 * Every diagram the CMS has described, keyed by the checksum of the source that draws it.
 *
 * Not by the record's own key, which is a BLAKE3 content id: computing one here would put a
 * second implementation of the article hash back into TypeScript, which is the duplication the
 * segment layout exists to remove. The record carries the same cheap FNV-1a the layout uses, over
 * the fence's payload, and this side recomputes that in the four lines it already has.
 */
export type DiagramStore = {
	diagrams: Record<
		string,
		{ fingerprint?: string; description?: Record<string, { text: string }> }
	>;
};

/**
 * What a diagram says, in the view being compiled, by the source that draws it.
 *
 * The locale is the same choice the asset resolver makes and for the same reason: a diagram on
 * the original view is described beside prose in the article's own language.
 */
export function createDiagramResolver(
	store: DiagramStore,
	descriptionLocale = 'en-US',
): (source: string) => string | undefined {
	const byFingerprint = new Map<string, string>();
	for (const entry of Object.values(store.diagrams ?? {})) {
		const text = entry.description?.[descriptionLocale]?.text?.trim();
		if (entry.fingerprint && text) byFingerprint.set(entry.fingerprint, text);
	}
	const encoder = new TextEncoder();
	return (source) => byFingerprint.get(sourceFingerprint(encoder.encode(source)));
}

export function createAssetResolver(
	assets: AssetManifest,
	media: MediaManifest,
	previews: ReadonlyMap<string, string>,
	/**
	 * Which CDN the markup should name.
	 *
	 * Passed rather than picked. This runs at build time, and which CDN answers depends on the
	 * mode -- a development build that named production would send a reader to bytes the local
	 * tree has not published, and hide the ones it has. The same reason the font stylesheets
	 * carry `__CDN_URL__` instead of a host. See spec/architecture/workspace.md.
	 */
	cdnUrl: string,
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
			src: url(cdnUrl, largest[0], largest[1].mime),
			srcset: variants.map(([cid, v]) => `${url(cdnUrl, cid, v.mime)} ${v.width}w`).join(', '),
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
