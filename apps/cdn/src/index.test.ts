import { Hono } from 'hono';
import { describe, expect, it } from 'vitest';
import { cacheControl } from './cache';
import { candidates, isValidHostname } from './favicon';
import { contentTypeFor } from './store';

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

describe('contentTypeFor', () => {
	it('maps the formats actually stored', () => {
		expect(contentTypeFor('favicon/a.com/light.svg')).toBe('image/svg+xml');
		expect(contentTypeFor('favicon/a.com/dark.ico')).toBe('image/x-icon');
		expect(contentTypeFor('fonts/x.woff2')).toBe('font/woff2');
	});

	it('falls back rather than guessing', () => {
		expect(contentTypeFor('unknown')).toBe('application/octet-stream');
	});
});

describe('cacheControl', () => {
	const app = new Hono();
	app.use('*', cacheControl);
	app.get('/fonts/x.woff2', (c) => c.text('font'));
	app.get('/favicon.svg', (c) => c.text('icon'));
	app.get('/missing', (c) => c.json({ error: 'not found' }, 404));
	app.get('/preset', (c) => {
		c.header('Cache-Control', 'no-store');
		return c.text('special');
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
