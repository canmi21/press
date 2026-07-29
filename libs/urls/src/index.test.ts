import { describe, expect, it } from 'vitest';
import { isDevHost, pickUrls, robotsTxt, robotsTxtBase, URLS } from './index';

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
		expect(URLS.apps.production).toHaveProperty('web');
		expect(URLS.apps.production).toHaveProperty('api');
		expect(URLS.apps.production).toHaveProperty('cdn');
		expect(URLS.internal).toHaveProperty('app');
		expect(URLS.internal).toHaveProperty('dev');
		expect(URLS.internal).toHaveProperty('prod');
		expect(URLS.external.github).toHaveProperty('cdn');
	});

	it('does not keep discarded app slots', () => {
		expect('res' in URLS.apps.production).toBe(false);
		expect('home' in URLS.apps.production).toBe(false);
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
		expect(isDevHost(hostname(URLS.apps.production.web))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.api))).toBe(false);
		expect(isDevHost(hostname(URLS.apps.production.cdn))).toBe(false);
	});

	it('rejects empty and arbitrary strings', () => {
		expect(isDevHost('')).toBe(false);
		expect(isDevHost('localhost.evil.com')).toBe(false);
	});
});

describe('robotsTxt', () => {
	it('returns the shared base without site additions', () => {
		expect(robotsTxt()).toBe(`${robotsTxtBase.join('\n')}\n`);
	});

	it('appends site-specific rules and sitemap entries', () => {
		expect(
			robotsTxt({
				disallow: ['/@/', '/private/'],
				sitemap: `${URLS.apps.production.web}/sitemap.xml`,
			}),
		).toBe(`${robotsTxtBase.join('\n')}
Disallow: /@/
Disallow: /private/

Sitemap: ${URLS.apps.production.web}/sitemap.xml
`);
	});
});

function hostname(url: string): string {
	return new URL(url).hostname;
}
