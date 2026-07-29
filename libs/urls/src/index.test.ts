import { describe, expect, it } from 'vitest';
import { isDevHost, pickUrls, URLS } from './index';

describe('pickUrls', () => {
	it('returns development map when isDev=true', () => {
		expect(pickUrls(true)).toEqual(URLS.development);
	});

	it('returns production map when isDev=false', () => {
		expect(pickUrls(false)).toEqual(URLS.production);
	});
});

describe('isDevHost', () => {
	it('matches localhost', () => {
		expect(isDevHost('localhost')).toBe(true);
	});

	it('matches 127.0.0.1', () => {
		expect(isDevHost('127.0.0.1')).toBe(true);
	});

	it('rejects production hosts', () => {
		expect(isDevHost('canmi.net')).toBe(false);
		expect(isDevHost('api.ffoni.com')).toBe(false);
		expect(isDevHost('cdn.ffoni.com')).toBe(false);
	});

	it('rejects empty and arbitrary strings', () => {
		expect(isDevHost('')).toBe(false);
		expect(isDevHost('localhost.evil.com')).toBe(false);
	});
});
