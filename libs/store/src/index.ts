import type { Fetcher, R2Bucket } from '@cloudflare/workers-types';

/**
 * Reading the bytes behind a key, from whichever store this deployment has.
 *
 * Production has the R2 bucket that mirrors `data/public`. Development has `data/public`
 * itself, handed over by `wrangler dev --assets`, because the local tree is the source of
 * truth and dev should read the source rather than a copy of it. A worker cannot open the
 * directory itself -- workerd's `node:fs` is a virtual filesystem and cannot see host paths
 * (verified), so the runtime has to pass it in.
 *
 * Everything above this module works in keys and knows nothing about which one answered.
 */

/**
 * Origin for asset-fetcher requests. `.invalid` is reserved by RFC 2606 to never resolve,
 * which is the point: the fetcher routes on the path and ignores the host, and a name that
 * cannot resolve makes it impossible for this to accidentally become a real request.
 */
const ASSET_ORIGIN = 'https://assets.invalid';

export type Bindings = {
	PUBLIC?: R2Bucket;
	/** Present only under `wrangler dev --assets`; see the dev task in mise.toml. */
	ASSETS?: Fetcher;
};

export type Found = {
	body: ReadableStream;
	contentType: string;
	etag?: string;
};

export async function read(env: Bindings, key: string): Promise<Found | null> {
	if (env.PUBLIC) return readFromBucket(env.PUBLIC, key);
	if (env.ASSETS) return readFromAssets(env.ASSETS, key);
	throw new Error('no store bound: expected PUBLIC in production or ASSETS under wrangler dev');
}

async function readFromBucket(bucket: R2Bucket, key: string): Promise<Found | null> {
	const object = await bucket.get(key);
	if (!object?.body) return null;
	return {
		body: object.body,
		contentType: object.httpMetadata?.contentType ?? contentTypeFor(key),
		etag: object.httpEtag,
	};
}

async function readFromAssets(assets: Fetcher, key: string): Promise<Found | null> {
	// The host is ignored by the assets fetcher; only the path matters.
	const response = await assets.fetch(`${ASSET_ORIGIN}/${key}`);
	if (!response.ok || !response.body) return null;
	return {
		body: response.body,
		contentType: response.headers.get('content-type') ?? contentTypeFor(key),
		// No validator, deliberately. Measured: wrangler's asset fetcher sends no ETag of its
		// own, and synthesising one here would let a browser hold a file that is being edited
		// on disk. Development should always answer with what the tree currently says.
	};
}

/**
 * The first key under `prefix`, or null.
 *
 * Used where the extension is not known ahead of time: a favicon is stored as whatever format
 * the site served, so the lookup is by directory rather than by exact name.
 *
 * The assets fetcher cannot list, so development probes the formats the CMS is able to write.
 * That list is short and closed -- see `extension_for` in apps/cms -- and a format missing
 * from it could not have been stored in the first place.
 */
export async function findOne(env: Bindings, prefix: string): Promise<string | null> {
	if (env.PUBLIC) {
		const listed = await env.PUBLIC.list({ prefix, limit: 1 });
		return listed.objects[0]?.key ?? null;
	}
	if (env.ASSETS) {
		for (const extension of STORED_FORMATS) {
			const key = `${prefix}${extension}`;
			const response = await env.ASSETS.fetch(`${ASSET_ORIGIN}/${key}`);
			if (response.ok) return key;
		}
		return null;
	}
	throw new Error('no store bound: expected PUBLIC in production or ASSETS under wrangler dev');
}

/**
 * What a content id looks like.
 *
 * BLAKE3 truncated to 128 bits, hex encoded. Checked before a key is built from one, because an
 * id becomes a path segment and an unchecked one is a way to ask the bucket for something else.
 */
const CONTENT_ID = /^[0-9a-f]{32}$/;

export function isContentId(value: string): boolean {
	return CONTENT_ID.test(value);
}

/**
 * Where an object lives, by kind.
 *
 * The layout is a fact about the bucket, so it belongs to the module that reads the bucket
 * rather than to each caller. It was written out three times -- apps/cms writes it, apps/cdn
 * reads it, apps/api reads it -- and the three had to agree on something none of them stated:
 * which prefixes fan out and which do not.
 *
 * **Only `image` and `license` fan out.** The two-level split exists for a filesystem mirror,
 * where one directory holding every object eventually stops being openable; R2 has no
 * directories and does not care either way. So it is applied where the count is unbounded and
 * skipped where a record is written once per asset -- `meta` is flat, and reading it as though
 * it were fanned is a 404 that looks like a missing asset.
 *
 * Never published. A caller asks for `{cid}.{ext}` and the prefix and the split are put back on
 * here; putting them in a URL would make the bucket's layout an interface nobody could change.
 */
export function imageKey(cid: string, extension: string): string {
	return fanned('image', cid, extension);
}

/** Licence texts are stored the same way, and are always plain text. */
export function licenseKey(cid: string): string {
	return fanned('license', cid, 'txt');
}

/** The record `cms image` writes beside the variants. Flat: one per asset, not per format. */
export function metaKey(cid: string): string {
	return `meta/${cid}.json`;
}

function fanned(prefix: string, cid: string, extension: string): string {
	return `${prefix}/${cid.slice(0, 2)}/${cid.slice(2, 4)}/${cid}.${extension}`;
}

/** Every extension apps/cms will write an icon under. */
const STORED_FORMATS = ['svg', 'png', 'jpg', 'ico'] as const;

/** A stored object as an HTTP response, with ETag only when the store supplied one. */
export function toResponse(found: Found): Response {
	const headers = new Headers({ 'Content-Type': found.contentType });
	if (found.etag) headers.set('ETag', found.etag);
	return new Response(found.body, { headers });
}

/**
 * Content type from the key, for objects stored without one.
 *
 * R2 keeps whatever `httpMetadata` was set at upload, and rclone does set it, but an object
 * put by hand through the dashboard has none. Serving those as `application/octet-stream`
 * makes a browser download a favicon instead of drawing it.
 */
export function contentTypeFor(key: string): string {
	const extension = key.split('.').pop()?.toLowerCase() ?? '';
	switch (extension) {
		case 'svg':
			return 'image/svg+xml';
		case 'png':
			return 'image/png';
		case 'jpg':
		case 'jpeg':
			return 'image/jpeg';
		case 'ico':
			return 'image/x-icon';
		case 'woff2':
			return 'font/woff2';
		case 'json':
			return 'application/json';
		case 'txt':
			return 'text/plain; charset=utf-8';
		default:
			return 'application/octet-stream';
	}
}
