import { describe, expect, it } from 'vitest';
import { type Bindings, contentTypeFor, read } from './index';

/** Enough of an R2 bucket to answer one key. */
function bucket(keys: Record<string, string>) {
	return {
		get: async (key: string) =>
			key in keys
				? { body: new Response(keys[key]).body, httpMetadata: {}, httpEtag: '"live"' }
				: null,
	} as unknown as NonNullable<Bindings['PUBLIC']>;
}

/** Enough of the assets fetcher to answer one path. */
function assets(paths: Record<string, string>) {
	return {
		fetch: async (url: string) => {
			const key = new URL(url).pathname.slice(1);
			return key in paths ? new Response(paths[key]) : new Response('', { status: 404 });
		},
	} as unknown as NonNullable<Bindings['ASSETS']>;
}

describe('read', () => {
	it('prefers the bucket when both are bound', async () => {
		// Deploying with a stale assets binding must not quietly serve last week's file.
		const env = { PUBLIC: bucket({ 'a.txt': 'bucket' }), ASSETS: assets({ 'a.txt': 'local' }) };
		const found = await read(env, 'a.txt');
		expect(await new Response(found?.body).text()).toBe('bucket');
	});

	it('falls back to local files when only assets are bound', async () => {
		const found = await read({ ASSETS: assets({ 'a.txt': 'local' }) }, 'a.txt');
		expect(await new Response(found?.body).text()).toBe('local');
	});

	it('reports a miss the same way from either store', async () => {
		// Both workers turn null into a 404, so the two stores have to agree on what absent is.
		expect(await read({ PUBLIC: bucket({}) }, 'gone.txt')).toBeNull();
		expect(await read({ ASSETS: assets({}) }, 'gone.txt')).toBeNull();
	});

	it('refuses to run with nothing bound', async () => {
		// Silently returning null here would look exactly like an empty bucket, and a
		// misconfigured deployment would read as a site whose assets had all been deleted.
		await expect(read({}, 'a.txt')).rejects.toThrow('no store bound');
	});
});

describe('contentTypeFor', () => {
	it('maps the formats actually stored', () => {
		expect(contentTypeFor('favicon/a.com/light.svg')).toBe('image/svg+xml');
		expect(contentTypeFor('favicon/a.com/dark.ico')).toBe('image/x-icon');
		expect(contentTypeFor('fonts/x.woff2')).toBe('font/woff2');
		expect(contentTypeFor('meta/44b6081deaf0242ca3bf83d62a3b6c95.json')).toBe('application/json');
	});

	it('falls back rather than guessing', () => {
		expect(contentTypeFor('unknown')).toBe('application/octet-stream');
	});
});
