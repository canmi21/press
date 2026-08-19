import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import {
	type Bindings,
	contentTypeFor,
	imageKey,
	isContentId,
	licenseKey,
	metaKey,
	read,
	STORED_FORMATS,
} from './index';

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

describe('where an object lives', () => {
	const CID = '44b6081deaf0242ca3bf83d62a3b6c95';

	/**
	 * The split that exists for a filesystem mirror rather than for R2, which has no directories.
	 * Two characters, then two more, then the whole id again -- matching what apps/cms writes.
	 */
	it('fans an image out over two levels', () => {
		expect(imageKey(CID, 'avif')).toBe(`image/44/b6/${CID}.avif`);
	});

	it('fans a licence text the same way', () => {
		expect(licenseKey(CID)).toBe(`license/44/b6/${CID}.txt`);
	});

	/**
	 * The asymmetry this module exists to state. A record is written once per asset, so its
	 * directory has a bound and the split buys nothing; reading it as though it were fanned is a
	 * 404 that looks exactly like a missing asset. Three copies of the layout had to agree on
	 * this and none of them said it.
	 */
	it('leaves a record flat, because there is one per asset', () => {
		expect(metaKey(CID)).toBe(`meta/${CID}.json`);
	});

	it('accepts an id of the shape apps/cms writes, and nothing else', () => {
		expect(isContentId(CID)).toBe(true);
		expect(isContentId(CID.toUpperCase())).toBe(false);
		expect(isContentId(CID.slice(0, 31))).toBe(false);
		expect(isContentId(`${CID}00`)).toBe(false);
		expect(isContentId('../../etc/passwd')).toBe(false);
	});
});

/**
 * The formats this probes for in development have to be the ones apps/cms actually wrote.
 *
 * Only reachable under `wrangler dev`, where the asset fetcher cannot list a prefix and the
 * lookup guesses instead. A format missing here is a favicon that resolves in production and
 * silently does not locally -- and one spelled differently is the same thing. They were: apps/cms
 * wrote `jpeg` while this asked for `jpg`.
 */
it('probes for the extensions apps/cms writes an icon under', () => {
	const source = readFileSync(
		fileURLToPath(new URL('../../../apps/cms/src/extension.rs', import.meta.url).href),
		'utf8',
	);
	const declaration = /const ICON_EXTENSIONS: \[&str; \d+\] = \[([^\]]*)\]/.exec(source);
	expect(declaration, 'ICON_EXTENSIONS moved or changed shape').not.toBeNull();

	// `JPEG` is a constant there rather than a literal, so the spelling it holds is resolved too.
	const jpeg = /const JPEG: &str = "([a-z]+)"/.exec(source)?.[1];
	const authoritative = declaration![1]!
		.split(',')
		.map((entry) => entry.trim())
		.filter(Boolean)
		.map((entry) => (entry === 'JPEG' ? jpeg : entry.replace(/"/g, '')));

	expect([...STORED_FORMATS]).toEqual(authoritative);
});
