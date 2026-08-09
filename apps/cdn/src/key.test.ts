import { describe, expect, it } from 'vitest';
import { keyFor, licenseKeyFor, parseName, validatorFor } from './key';

describe('validatorFor', () => {
	it('distinguishes the formats one id serves', () => {
		// A tag shared across formats would let a cache answer a WebP request with the AVIF
		// bytes it already holds, which is a corrupt image rather than a slow one.
		const cid = '44b6081deaf0242ca3bf83d62a3b6c95';
		expect(validatorFor(cid, 'avif')).not.toBe(validatorFor(cid, 'webp'));
	});

	it('is quoted, as an entity tag has to be', () => {
		expect(validatorFor('44b6081deaf0242ca3bf83d62a3b6c95', 'avif')).toBe(
			'"44b6081deaf0242ca3bf83d62a3b6c95.avif"',
		);
	});
});

describe('keyFor', () => {
	it('fans out over the first four characters', () => {
		expect(keyFor('44b6081deaf0242ca3bf83d62a3b6c95', 'avif')).toBe(
			'image/44/b6/44b6081deaf0242ca3bf83d62a3b6c95.avif',
		);
	});

	it('matches the layout apps/cms writes', () => {
		// Two spellings of one scheme is one more than can be kept in step, so this is the
		// test that fails if either side moves.
		const key = keyFor('abcdef0123456789abcdef0123456789', 'png');
		expect(key.split('/').slice(0, 3)).toEqual(['image', 'ab', 'cd']);
		expect(key.split('/').pop()).toBe('abcdef0123456789abcdef0123456789.png');
	});
});

describe('licenseKeyFor', () => {
	it('fans out the same way, under its own prefix', () => {
		expect(licenseKeyFor('7ed218d2928b1ff56267b33a04541b5f')).toBe(
			'license/7e/d2/7ed218d2928b1ff56267b33a04541b5f.txt',
		);
	});

	// The point of the route is that this key is never a URL. If the two ever diverge the
	// worker stops finding what apps/cms wrote, which is the failure this pins down.
	it('matches the layout apps/cms writes', () => {
		const key = licenseKeyFor('abcdef0123456789abcdef0123456789');
		expect(key.split('/').slice(0, 3)).toEqual(['license', 'ab', 'cd']);
		expect(key.split('/').pop()).toBe('abcdef0123456789abcdef0123456789.txt');
	});
});

describe('parseName', () => {
	it('splits a content id from its extension', () => {
		expect(parseName('44b6081deaf0242ca3bf83d62a3b6c95.avif')).toEqual({
			cid: '44b6081deaf0242ca3bf83d62a3b6c95',
			extension: 'avif',
		});
	});

	it('lowercases both halves', () => {
		expect(parseName('44B6081DEAF0242CA3BF83D62A3B6C95.AVIF')).toEqual({
			cid: '44b6081deaf0242ca3bf83d62a3b6c95',
			extension: 'avif',
		});
	});

	it('rejects anything that is not a 128-bit hex id', () => {
		// The id length is what makes a key unguessable-by-typo rather than merely unusual, so
		// a short or non-hex name is refused before it reaches the bucket.
		expect(parseName('short.avif')).toBeNull();
		expect(parseName('44b6081deaf0242ca3bf83d62a3b6c9.avif')).toBeNull();
		expect(parseName('zzb6081deaf0242ca3bf83d62a3b6c95.avif')).toBeNull();
	});

	it('rejects a name with no extension', () => {
		expect(parseName('44b6081deaf0242ca3bf83d62a3b6c95')).toBeNull();
		expect(parseName('.avif')).toBeNull();
	});

	it('rejects a doubled extension rather than reading past it', () => {
		// Splitting on the last dot leaves `{cid}.avif` as the id, which is not one. Accepting
		// it would mean two names resolving to the same object, and a content address that is
		// not unique stops being an address.
		expect(parseName('44b6081deaf0242ca3bf83d62a3b6c95.avif.webp')).toBeNull();
	});
});
