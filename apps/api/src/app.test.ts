import { developmentUrl, URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import app from './app';

const dev = `${URLS.apps.development.api}/`;
const prod = `${URLS.apps.production.api}/`;

describe('GET /', () => {
	it('sends a development request to the development site', async () => {
		const res = await app.fetch(new Request(dev));
		expect(res.status).toBe(302);
		expect(res.headers.get('Location')).toBe(`${URLS.apps.development.site}/?ref=api`);
	});

	it('sends a production request to the production site', async () => {
		const res = await app.fetch(new Request(prod));
		expect(res.status).toBe(302);
		expect(res.headers.get('Location')).toBe(`${URLS.apps.production.site}/?ref=api`);
	});
});

describe('GET /favicon.ico', () => {
	it('points at the CDN for the matching environment', async () => {
		const local = await app.fetch(new Request(`${dev}favicon.ico`));
		expect(local.status).toBe(301);
		expect(local.headers.get('Location')).toBe(`${URLS.apps.development.cdn}/favicon.ico`);

		const remote = await app.fetch(new Request(`${prod}favicon.ico`));
		expect(remote.headers.get('Location')).toBe(`${URLS.apps.production.cdn}/favicon.ico`);
	});
});

describe('GET /robots.txt', () => {
	it('asks crawlers to stay out entirely', async () => {
		const res = await app.fetch(new Request(`${prod}robots.txt`));
		expect(await res.text()).toContain('Disallow: /');
	});
});

describe('CORS', () => {
	it('allows the site', async () => {
		const res = await app.fetch(
			new Request(prod, { headers: { Origin: URLS.apps.production.site } }),
		);
		expect(res.headers.get('Access-Control-Allow-Origin')).toBe(URLS.apps.production.site);
	});

	// An overlay workspace's site answers on a slot port the list cannot name; see
	// spec/toolchain.md. The base API lets it in only while it is itself a development host.
	it('allows an overlay site while answering on a development host', async () => {
		const overlay = developmentUrl('site', 2);
		const res = await app.fetch(new Request(dev, { headers: { Origin: overlay } }));
		expect(res.headers.get('Access-Control-Allow-Origin')).toBe(overlay);
	});

	it('does not extend that to production', async () => {
		const overlay = developmentUrl('site', 2);
		const res = await app.fetch(new Request(prod, { headers: { Origin: overlay } }));
		expect(res.headers.get('Access-Control-Allow-Origin')).toBeNull();
	});

	it('does not allow an unknown origin', async () => {
		// The list is an allowlist; anything not on it gets no header at all, which is what
		// makes a browser refuse the response.
		const res = await app.fetch(new Request(prod, { headers: { Origin: 'https://evil.test' } }));
		expect(res.headers.get('Access-Control-Allow-Origin')).toBeNull();
	});
});
