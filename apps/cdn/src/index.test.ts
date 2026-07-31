import { robotsTxt } from '@canmi/robots';
import { Hono } from 'hono';
import { describe, expect, it } from 'vitest';
import { cacheControl } from './cache';
import { candidates, isValidHostname } from './favicon';

describe('isValidHostname', () => {
	it('accepts an ordinary hostname', () => {
		expect(isValidHostname('example.com')).toBe(true);
		expect(isValidHostname('blog.example.co.uk')).toBe(true);
	});

	it('rejects things that are not public sites', () => {
		// A worker resolving `localhost` would reach itself, and a bare address is not a site.
		expect(isValidHostname('localhost')).toBe(false);
		expect(isValidHostname('127.0.0.1')).toBe(false);
		expect(isValidHostname('nodots')).toBe(false);
	});

	it('keeps a hostname that merely starts with a digit', () => {
		expect(isValidHostname('1.example.com')).toBe(true);
	});

	it('rejects malformed labels', () => {
		expect(isValidHostname('-lead.example.com')).toBe(false);
		expect(isValidHostname('trail-.example.com')).toBe(false);
		expect(isValidHostname('a..example.com')).toBe(false);
		expect(isValidHostname('bad_underscore.com')).toBe(false);
	});
});

describe('candidates', () => {
	it('honours a named tone exactly, with no substitute', () => {
		// Returning the other shade would be invisible to the caller, which would then draw a
		// light icon on a dark surface believing it had asked for and received the right one.
		expect(candidates('dark')).toEqual(['dark']);
		expect(candidates('light')).toEqual(['light']);
	});

	it('accepts either variant when no tone is named', () => {
		expect(candidates(undefined)).toEqual(['light', 'dark']);
	});

	it('treats an unrecognised tone as no tone', () => {
		expect(candidates('sepia')).toEqual(['light', 'dark']);
	});
});

describe('cacheControl', () => {
	const app = new Hono();
	app.use('*', cacheControl);
	app.get('/fonts/x.woff2', (c) => c.text('font'));
	app.get('/image/44b6081deaf0242ca3bf83d62a3b6c95.avif', (c) => c.text('bytes'));
	app.get('/image/44b6081deaf0242ca3bf83d62a3b6c95.gone', (c) => c.json({ error: 'x' }, 404));
	app.get('/favicon.svg', (c) => c.text('icon'));
	app.get('/missing', (c) => c.json({ error: 'not found' }, 404));
	app.get('/preset', (c) => {
		c.header('Cache-Control', 'no-store');
		return c.text('special');
	});

	it('keeps a hashed name for a year', async () => {
		// The year rests on the name being a hash, not on a promise anyone has to remember.
		const res = await app.request('/image/44b6081deaf0242ca3bf83d62a3b6c95.avif');
		expect(res.headers.get('Cache-Control')).toBe('public, max-age=31536000, immutable');
	});

	it('will not keep an error for a year, hashed or not', async () => {
		// An error is a statement about right now: the asset may be published a minute later,
		// and a year-long 404 would outlive the reason for it.
		const res = await app.request('/image/44b6081deaf0242ca3bf83d62a3b6c95.gone');
		expect(res.headers.get('Cache-Control')).toBe('public, max-age=300');
	});

	it('marks fonts immutable for a year', async () => {
		const res = await app.request('/fonts/x.woff2');
		expect(res.headers.get('Cache-Control')).toBe('public, max-age=31536000, immutable');
	});

	it('gives everything else a week', async () => {
		const res = await app.request('/favicon.svg');
		expect(res.headers.get('Cache-Control')).toBe('public, max-age=604800');
	});

	it('caches misses briefly instead of not at all', async () => {
		// A missing favicon is requested on every page view; without this each one is a full
		// trip to the origin.
		const res = await app.request('/missing');
		expect(res.headers.get('Cache-Control')).toBe('public, max-age=300');
	});

	it('never overrides a header a route set deliberately', async () => {
		const res = await app.request('/preset');
		expect(res.headers.get('Cache-Control')).toBe('no-store');
	});
});

describe('robots policy', () => {
	// The change this guards was written once and silently lost to a bad patch, and nothing
	// noticed until a card failed to appear. What a crawler is allowed to fetch is worth an
	// assertion rather than a reading of the source.
	const text = robotsTxt({ allow: ['/opengraph/'], disallow: ['/'] });

	it('lets crawlers reach the cards a page advertises', () => {
		expect(text).toContain('Allow: /opengraph/');
	});

	it('still keeps everything else out', () => {
		expect(text).toContain('Disallow: /');
	});

	it('allows before it disallows, which is the order that decides', () => {
		expect(text.indexOf('Allow: /opengraph/')).toBeLessThan(text.indexOf('Disallow: /'));
	});
});
