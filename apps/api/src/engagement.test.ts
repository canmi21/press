import { readdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { Miniflare } from 'miniflare';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import app from './app';
import type { Bindings } from './bindings';
import { ARTICLE_SLUGS } from './slugs';

const MIGRATIONS = fileURLToPath(new URL('../drizzle', import.meta.url).href);
const IP_ONE = '203.0.113.10';
const IP_TWO = '2001:db8::20';
const allow: RateLimit = { limit: async () => ({ success: true }) };

let miniflare: Miniflare;
let database: Awaited<ReturnType<Miniflare['getD1Database']>>;

beforeAll(async () => {
	miniflare = new Miniflare({
		compatibilityDate: '2026-07-29',
		modules: true,
		script: 'export default { fetch() { return new Response("unused") } }',
		d1Databases: ['DATABASE'],
	});
	database = await miniflare.getD1Database('DATABASE');
	const migrationNames = (await readdir(MIGRATIONS))
		.filter((migrationName) => migrationName.endsWith('.sql'))
		.toSorted();
	const migrations = await Promise.all(
		migrationNames.map((migrationName) => readFile(`${MIGRATIONS}/${migrationName}`, 'utf8')),
	);
	for (const sql of migrations) {
		for (const statement of sql.split('--> statement-breakpoint')) {
			// oxlint-disable-next-line no-await-in-loop -- later statements depend on earlier DDL
			if (statement.trim()) await database.prepare(statement).run();
		}
	}
});

beforeEach(async () => {
	await database.batch([
		database.prepare('DELETE FROM newsletter_subscriptions'),
		database.prepare('DELETE FROM likes'),
		database.prepare('DELETE FROM article_reads'),
	]);
});

afterAll(async () => {
	await miniflare.dispose();
});

describe('newsletter', () => {
	it('canonicalizes and deduplicates by email without exposing an existing token', async () => {
		const first = await api('/newsletter', {
			method: 'POST',
			ip: IP_ONE,
			body: { email: ' Alice+notes@Example.com ' },
		});
		expect(first.status).toBe(201);
		const created = await first.json<{
			email: string;
			cancel_token: string;
			subscriber_count: number;
		}>();
		expect(created).toMatchObject({ email: 'alice@example.com', subscriber_count: 1 });
		expect(created.cancel_token).toMatch(/^[0-9a-f]{32}$/);

		const stored = await database
			.prepare('SELECT email, cancel_token_hash, ip FROM newsletter_subscriptions WHERE email = ?')
			.bind(created.email)
			.first<{ email: string; cancel_token_hash: string; ip: string }>();
		expect(stored).toMatchObject({ email: created.email, ip: IP_ONE });
		expect(stored?.cancel_token_hash).toMatch(/^[a-f0-9]{64}$/);
		expect(stored?.cancel_token_hash).not.toBe(created.cancel_token);

		const duplicate = await api('/newsletter', {
			method: 'POST',
			ip: IP_TWO,
			body: { email: 'alice+different@example.com' },
		});
		expect(duplicate.status).toBe(200);
		expect(await duplicate.json()).toEqual({
			email: 'alice@example.com',
			subscriber_count: 1,
		});

		const unchanged = await database
			.prepare('SELECT ip FROM newsletter_subscriptions WHERE email = ?')
			.bind(created.email)
			.first<{ ip: string }>();
		expect(unchanged?.ip).toBe(IP_ONE);
	});

	it('cancels only with the capability token', async () => {
		const created = await (
			await api('/newsletter', {
				method: 'POST',
				ip: IP_ONE,
				body: { email: 'reader@example.com' },
			})
		).json<{ cancel_token: string }>();

		const denied = await api('/newsletter', {
			method: 'DELETE',
			ip: IP_TWO,
			body: { email: 'reader@example.com', cancel_token: '0'.repeat(32) },
		});
		expect(denied.status).toBe(404);

		const cancelled = await api('/newsletter', {
			method: 'DELETE',
			ip: IP_TWO,
			body: { email: 'READER+tag@example.com', cancel_token: created.cancel_token },
		});
		expect(cancelled.status).toBe(200);
		expect(await cancelled.json()).toEqual({ cancelled: true, subscriber_count: 0 });
	});
});

describe('likes and engagement state', () => {
	it('allows one active like per raw IP and returns per-IP state', async () => {
		const first = await api('/like', { method: 'PUT', ip: IP_ONE, body: { liked: true } });
		expect(await first.json()).toEqual({ liked: true, like_count: 1 });

		const repeated = await api('/like', { method: 'PUT', ip: IP_ONE, body: { liked: true } });
		expect(await repeated.json()).toEqual({ liked: true, like_count: 1 });

		const second = await api('/like', { method: 'PUT', ip: IP_TWO, body: { liked: true } });
		expect(await second.json()).toEqual({ liked: true, like_count: 2 });

		const state = await api('/engagement', { ip: IP_ONE });
		expect(await state.json()).toEqual({ subscriber_count: 0, like_count: 2, liked: true });

		const removed = await api('/like', { method: 'PUT', ip: IP_ONE, body: { liked: false } });
		expect(await removed.json()).toEqual({ liked: false, like_count: 1 });
	});

	it('returns a retry hint when Cloudflare rejects a mutation', async () => {
		const deny: RateLimit = { limit: async () => ({ success: false }) };
		const response = await api(
			'/like',
			{ method: 'PUT', ip: IP_ONE, body: { liked: true } },
			{ LIKE_RATE_LIMITER: deny },
		);
		expect(response.status).toBe(429);
		expect(response.headers.get('Retry-After')).toBe('60');
		expect(await response.json()).toEqual({ error: 'rate_limited' });
	});

	it('rate limits the read-heavy state endpoint separately', async () => {
		const deny: RateLimit = { limit: async () => ({ success: false }) };
		const response = await api('/engagement', { ip: IP_ONE }, { ENGAGEMENT_RATE_LIMITER: deny });
		expect(response.status).toBe(429);
	});
});

describe('article reads', () => {
	const slug = [...ARTICLE_SLUGS][0] as string;

	it('counts from the first read and answers with the running total', async () => {
		const first = await api('/read', { method: 'POST', ip: IP_ONE, body: { slug } });
		expect(first.status).toBe(200);
		expect(await first.json()).toEqual({ slug, read_count: 1 });

		const second = await api('/read', { method: 'POST', ip: IP_TWO, body: { slug } });
		expect(await second.json()).toEqual({ slug, read_count: 2 });
	});

	it('refuses a slug that does not name an article', async () => {
		const response = await api('/read', {
			method: 'POST',
			ip: IP_ONE,
			body: { slug: 'made/up' },
		});
		expect(response.status).toBe(404);
		expect(await response.json()).toEqual({ error: 'unknown_article' });

		const rows = await database
			.prepare('SELECT COUNT(*) AS rows FROM article_reads')
			.first<{ rows: number }>();
		expect(rows?.rows).toBe(0);
	});

	// A second look inside the minute is the same read. The reader still needs the number to
	// put on the page, so the request is answered rather than refused.
	it('returns the unchanged count instead of an error once deduplicated', async () => {
		await api('/read', { method: 'POST', ip: IP_ONE, body: { slug } });

		const deny: RateLimit = { limit: async () => ({ success: false }) };
		const repeated = await api(
			'/read',
			{ method: 'POST', ip: IP_ONE, body: { slug } },
			{ READ_RATE_LIMITER: deny },
		);
		expect(repeated.status).toBe(200);
		expect(await repeated.json()).toEqual({ slug, read_count: 1 });
	});

	it('reports an unread article as zero rather than creating its row', async () => {
		const deny: RateLimit = { limit: async () => ({ success: false }) };
		const response = await api(
			'/read',
			{ method: 'POST', ip: IP_ONE, body: { slug } },
			{ READ_RATE_LIMITER: deny },
		);
		expect(await response.json()).toEqual({ slug, read_count: 0 });

		const rows = await database
			.prepare('SELECT COUNT(*) AS rows FROM article_reads')
			.first<{ rows: number }>();
		expect(rows?.rows).toBe(0);
	});

	it('rejects an IP walking every slug in turn', async () => {
		const deny: RateLimit = { limit: async () => ({ success: false }) };
		const response = await api(
			'/read',
			{ method: 'POST', ip: IP_ONE, body: { slug } },
			{ ENGAGEMENT_RATE_LIMITER: deny },
		);
		expect(response.status).toBe(429);
	});
});

type ApiOptions = {
	method?: string;
	ip: string;
	body?: Record<string, unknown>;
};

async function api(
	path: string,
	options: ApiOptions,
	overrides: Partial<Bindings> = {},
): Promise<Response> {
	const bindings = {
		DATABASE: database as unknown as Bindings['DATABASE'],
		ENGAGEMENT_RATE_LIMITER: allow,
		NEWSLETTER_RATE_LIMITER: allow,
		LIKE_RATE_LIMITER: allow,
		READ_RATE_LIMITER: allow,
		...overrides,
	} satisfies Bindings;
	return app.fetch(
		new Request(`${URLS.apps.production.api}${path}`, {
			method: options.method,
			headers: {
				'CF-Connecting-IP': options.ip,
				'Content-Type': 'application/json',
				Origin: URLS.apps.production.site,
			},
			body: options.body ? JSON.stringify(options.body) : undefined,
		}),
		bindings,
	);
}
