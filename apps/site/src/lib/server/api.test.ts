import { describe, expect, it } from 'vitest';
import { app } from './api';

describe('embedded /api hono', () => {
	it('responds at /api', async () => {
		const res = await app.fetch(new Request('http://localhost/api'));
		expect(res.status).toBe(200);
		expect(await res.text()).toBe('hello from hono');
	});

	it('404s outside /api basepath', async () => {
		const res = await app.fetch(new Request('http://localhost/other'));
		expect(res.status).toBe(404);
	});
});
