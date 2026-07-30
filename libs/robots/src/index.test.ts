import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import { robotsTxt, robotsTxtBase } from './index';

describe('robotsTxt', () => {
	it('returns the shared base without site additions', () => {
		expect(robotsTxt()).toBe(`${robotsTxtBase.join('\n')}\n`);
	});

	it('appends site-specific rules and sitemap entries', () => {
		expect(
			robotsTxt({
				disallow: ['/@/', '/private/'],
				sitemap: `${URLS.apps.production.site}/sitemap.xml`,
			}),
		).toBe(`${robotsTxtBase.join('\n')}
Disallow: /@/
Disallow: /private/

Sitemap: ${URLS.apps.production.site}/sitemap.xml
`);
	});

	it('accepts several sitemaps', () => {
		expect(robotsTxt({ sitemap: ['/a.xml', '/b.xml'] })).toBe(`${robotsTxtBase.join('\n')}

Sitemap: /a.xml
Sitemap: /b.xml
`);
	});

	it('treats an empty sitemap as absent', () => {
		expect(robotsTxt({ sitemap: null })).toBe(`${robotsTxtBase.join('\n')}\n`);
	});
});
