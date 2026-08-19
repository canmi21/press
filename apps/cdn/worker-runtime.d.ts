/// <reference types="@cloudflare/workers-types" />

// Runtime APIs for this worker. The reference sits here rather than in tsconfig.workers.json's
// `types` because that field resolves from the directory holding the config, and the package is
// installed per workspace member -- so the root cannot see it and this file can.

/**
 * The pixel buffer the codecs exchange, supplied by this worker rather than by the runtime.
 *
 * workerd has no `ImageData`, so `transcode.ts` installs one on `globalThis` before any codec
 * runs. Declared here because that makes the type and the implementation the same claim: while
 * the DOM library was in scope this checked against a browser's `ImageData` and happened to
 * agree, which is not the same as being right -- the assignment needed a `@ts-expect-error` to
 * get past a checker that was describing a real gap.
 *
 * Only what the codecs construct and read. `@jsquash` passes width, height and an RGBA buffer;
 * nothing here uses the rest of the browser's surface.
 */
interface ImageData {
	readonly data: Uint8ClampedArray;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: string;
}

declare var ImageData: {
	prototype: ImageData;
	new (data: Uint8ClampedArray, width: number, height?: number): ImageData;
	new (width: number, height: number): ImageData;
};
