import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import encode, { init } from '@jsquash/webp/encode.js';
import { thumbHashToRGBA } from 'thumbhash';
import type { AssetManifest } from '$lib/assets';

/**
 * The inline placeholders, all of them, encoded once at build time.
 *
 * Lives under `server/` because it reaches for `node:fs` to load its codec, which no browser
 * bundle may contain. What it produces is inlined into prerendered HTML, so a reader receives
 * the result and never this.
 *
 * Every hash in the manifest is encoded in one build pass rather than on demand. The codec is
 * asynchronous and `resolve` in $lib/assets is called from the middle of a markdown walk;
 * doing this before compilation lets that stay synchronous, instead of turning every caller
 * async to await a lookup that was going to be needed anyway.
 *
 * The manifest carries only the thumbhash. A decoded copy used to sit beside it -- the same
 * picture written twice -- and deriving it here keeps one source of truth, but only because
 * the derivation turned out to be as good as what it replaced. thumbhash's own
 * `thumbHashToDataURL` writes an uncompressed PNG at 3.3KB, twenty times the 167-byte WebP it
 * stood in for; the same pixels through libwebp come to 144 bytes. A derivation being
 * possible is not the same as it being free, and this one had to be measured.
 */
/**
 * Quality for an image roughly 32 pixels on its long edge.
 *
 * Higher is wasted: the source is a thumbhash, which has already discarded everything but an
 * impression of colour and shape. This only has to avoid adding artefacts of its own to a
 * picture about to be covered by the real one.
 */
const QUALITY = 70;

export async function buildPreviews(assets: AssetManifest): Promise<Map<string, string>> {
	const require = createRequire(import.meta.url);
	await init(
		await WebAssembly.compile(
			await readFile(require.resolve('@jsquash/webp/codec/enc/webp_enc.wasm')),
		),
	);

	const previews = new Map<string, string>();
	for (const asset of Object.values(assets.media)) {
		if (!asset.thumbhash || previews.has(asset.thumbhash)) continue;
		const bytes = Uint8Array.from(atob(asset.thumbhash), (c) => c.charCodeAt(0));
		const { w, h, rgba } = thumbHashToRGBA(bytes);
		const webp = await encode(
			{ data: new Uint8ClampedArray(rgba), width: w, height: h, colorSpace: 'srgb' },
			{ quality: QUALITY },
		);
		const base64 = Buffer.from(webp).toString('base64');
		previews.set(asset.thumbhash, `data:image/webp;base64,${base64}`);
	}
	return previews;
}
