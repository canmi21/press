import type { R2Bucket } from '@cloudflare/workers-types';

/**
 * Reading from the bucket that mirrors `data/public`.
 *
 * There is no local-versus-remote branch here. `wrangler dev --remote` points development at
 * the same bucket production uses, so this code has one path and cannot behave differently in
 * the place where nobody is watching.
 */

export type Found = {
	body: ReadableStream;
	contentType: string;
	etag: string;
};

export async function read(bucket: R2Bucket, key: string): Promise<Found | null> {
	const object = await bucket.get(key);
	if (!object || !object.body) return null;
	return {
		body: object.body as unknown as ReadableStream,
		contentType: object.httpMetadata?.contentType ?? contentTypeFor(key),
		etag: object.httpEtag,
	};
}

/**
 * The first key under `prefix`, or null.
 *
 * Used where the extension is not known ahead of time: a favicon is stored as whatever format
 * the site served, so the lookup is by directory rather than by exact name.
 */
export async function findOne(bucket: R2Bucket, prefix: string): Promise<string | null> {
	const listed = await bucket.list({ prefix, limit: 1 });
	return listed.objects[0]?.key ?? null;
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
