import decodeAvif, { init as initAvifDecode } from '@jsquash/avif/decode';
import AVIF_DEC_WASM from '@jsquash/avif/codec/dec/avif_dec.wasm';
import encodeJpeg, { init as initJpegEncode } from '@jsquash/jpeg/encode';
import JPEG_ENC_WASM from '@jsquash/jpeg/codec/enc/mozjpeg_enc.wasm';
import decodePng, { init as initPngDecode } from '@jsquash/png/decode';
import encodePng, { init as initPngEncode } from '@jsquash/png/encode';
// Unlike the other three codecs, this one ships a wasm-bindgen declaration describing the
// module's own exports. What arrives here is whatever the bundler substitutes for the
// import, and wrangler substitutes a compiled WebAssembly.Module.
// @ts-expect-error -- the shipped .d.ts describes the wasm's exports, not the bundler's.
import PNG_WASM from '@jsquash/png/codec/pkg/squoosh_png_bg.wasm';
import encodeWebp, { init as initWebpEncode } from '@jsquash/webp/encode';
import WEBP_ENC_WASM from '@jsquash/webp/codec/enc/webp_enc.wasm';

/**
 * Re-encoding a stored image into another format, here rather than at the edge.
 *
 * Cloudflare's image transformations cannot read AVIF unless the zone is Enterprise, and even
 * then the source is capped at 1200px while these variants go to 1920 -- so the format we
 * chose to store is the one format that pipeline cannot open. Measured: an AVIF source
 * returns `ERROR 9520: Original image has unsupported format` while the same request against
 * a PNG source succeeds. Doing it in the worker removes the plan tier, the monthly quota and
 * the dimension ceiling in one move. See spec/architecture/delivery.md.
 *
 * Only decoders for what is actually stored, and only encoders for what is actually asked
 * for. The AVIF *encoder* is deliberately absent: it is 1.1MB compressed against 332KB for
 * the decoder, and apps/cms already produces AVIF locally where the time is free.
 */

/** workerd has no DOM, and the codecs exchange pixels as `ImageData`. */
if (typeof globalThis.ImageData === 'undefined') {
	// @ts-expect-error -- supplying the shape the codecs construct and read.
	globalThis.ImageData = class {
		data: Uint8ClampedArray;
		width: number;
		height: number;
		colorSpace = 'srgb';

		constructor(data: Uint8ClampedArray | number, width: number, height?: number) {
			if (typeof data === 'number') {
				this.width = data;
				this.height = width;
				this.data = new Uint8ClampedArray(this.width * this.height * 4);
			} else {
				this.data = data;
				this.width = width;
				this.height = height ?? 0;
			}
		}
	};
}

/**
 * A codec is initialised once per isolate, and only if something asks for it.
 *
 * Instantiating all four at module scope would put the cost on every request, including the
 * overwhelming majority that read a stored AVIF and never transcode anything.
 */
function once(start: () => Promise<unknown>): () => Promise<void> {
	let running: Promise<void> | undefined;
	return () => {
		running ??= start().then(() => undefined);
		return running;
	};
}

const readyAvifDecode = once(() => initAvifDecode(AVIF_DEC_WASM));
const readyPngDecode = once(() => initPngDecode(PNG_WASM));
const readyPngEncode = once(() => initPngEncode(PNG_WASM));
const readyJpegEncode = once(() => initJpegEncode(JPEG_ENC_WASM));
const readyWebpEncode = once(() => initWebpEncode(WEBP_ENC_WASM));

/** What a stored object can be, and what a request can ask to be given. */
export const DECODABLE = ['avif', 'png'] as const;
export const ENCODABLE = ['webp', 'jpeg', 'jpg', 'png'] as const;

export type Decodable = (typeof DECODABLE)[number];
export type Encodable = (typeof ENCODABLE)[number];

/**
 * Quality for the fallback formats.
 *
 * These are only ever served to a browser that cannot read AVIF, so they are a compatibility
 * path rather than the one being optimised. High enough that the fallback is not visibly
 * worse than the image everyone else gets.
 */
const QUALITY = 80;

export function isDecodable(extension: string): extension is Decodable {
	return (DECODABLE as readonly string[]).includes(extension);
}

export function isEncodable(extension: string): extension is Encodable {
	return (ENCODABLE as readonly string[]).includes(extension);
}

async function toPixels(bytes: ArrayBuffer, from: Decodable): Promise<ImageData> {
	let pixels: ImageData | null;
	if (from === 'avif') {
		await readyAvifDecode();
		pixels = await decodeAvif(bytes);
	} else {
		await readyPngDecode();
		pixels = await decodePng(bytes);
	}
	// The decoders answer null rather than throwing on input they cannot read. Stored objects
	// were written by apps/cms and should always decode, so reaching this means the object is
	// damaged -- which the caller turns into a 502 rather than an empty image.
	if (!pixels) throw new Error(`could not decode the stored ${from}`);
	return pixels;
}

async function fromPixels(pixels: ImageData, to: Encodable): Promise<ArrayBuffer> {
	switch (to) {
		case 'webp':
			await readyWebpEncode();
			return encodeWebp(pixels, { quality: QUALITY });
		case 'jpeg':
		case 'jpg':
			await readyJpegEncode();
			return encodeJpeg(pixels, { quality: QUALITY });
		case 'png':
			await readyPngEncode();
			return encodePng(pixels);
	}
}

/** Decode `bytes` and re-encode them as `to`. */
export async function transcode(
	bytes: ArrayBuffer,
	from: Decodable,
	to: Encodable,
): Promise<ArrayBuffer> {
	return fromPixels(await toPixels(bytes, from), to);
}
