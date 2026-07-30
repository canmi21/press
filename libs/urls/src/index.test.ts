import { describe, expect, it } from 'vitest';
import { isDevHost, pickUrls, URLS } from './index';

describe('pickUrls', () => {
	it('returns development app URLs when isDev=true', () => {
		expect(pickUrls(true)).toEqual(URLS.apps.development);
	});

	it('returns production app URLs when isDev=false', () => {
		expect(pickUrls(false)).toEqual(URLS.apps.production);
	});
});

describe('URLS', () => {
	it('keeps apps, internal domains, and external endpoints separate', () => {
		expect(URLS.apps.production).toHaveProperty('site');
		expect(URLS.apps.production).toHaveProperty('api');
		expect(URLS.apps.production).toHaveProperty('cdn');
		expect(URLS.internal).toHaveProperty('app');
		expect(URLS.internal).toHaveProperty('infra');
		expect(URLS.internal).toHaveProperty('link');
		expect(URLS.external.github).toHaveProperty('cdn');
	});

	it('does not keep discarded app slots', () => {
		expect('res' in URLS.apps.production).toBe(false);
		expect('home' in URLS.apps.production).toBe(false);
		expect('web' in URLS.apps.production).toBe(false);
	});

	it('does not keep retired domains', () => {
		// canmi.dev is not being renewed, and `prod` was renamed to `infra` because it read
		// as a sibling of apps.production while meaning something unrelated.
		expect('dev' in URLS.internal).toBe(false);
		expect('prod' in URLS.internal).toBe(false);
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
		expect(isDevHost(hostname(URLS.apps.production.site))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.api))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.cdn))).toBe(false);
	});

	it('rejects empty and arbitrary strings', () => {
		expect(isDevHost('')).toBe(false);
		expect(isDevHost('localhost.evil.com')).toBe(false);
	});
});

function hostname(url: string): string {
	return new URL(url).hostname;
}
