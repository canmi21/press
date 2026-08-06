import { and, count, eq } from 'drizzle-orm';
import { drizzle } from 'drizzle-orm/d1';
import { Hono } from 'hono';
import { bodyLimit } from 'hono/body-limit';
import type { Bindings } from './bindings';
import { canonicalEmail } from './email';
import { likes, newsletterSubscriptions } from './schema';

const MAX_BODY_SIZE = 1_024;
const NO_STORE = { 'Cache-Control': 'no-store' } as const;
const JSON_LIMIT = bodyLimit({
	maxSize: MAX_BODY_SIZE,
	onError: (c) => c.json({ error: 'body_too_large' }, 413, NO_STORE),
});

const engagement = new Hono<{ Bindings: Bindings }>();

engagement.get('/engagement', async (c) => {
	const ip = clientIp(c.req.raw);
	if (!ip) return c.json({ error: 'client_ip_unavailable' }, 400, NO_STORE);
	if (!(await withinLimit(c.env.ENGAGEMENT_RATE_LIMITER, ip))) return rateLimited();

	const database = drizzle(c.env.DATABASE);
	const [subscriberCount, likeCount, like] = await Promise.all([
		rowCount(database, newsletterSubscriptions),
		rowCount(database, likes),
		database.select({ ip: likes.ip }).from(likes).where(eq(likes.ip, ip)).limit(1),
	]);

	return c.json(
		{
			subscriber_count: subscriberCount,
			like_count: likeCount,
			liked: like.length === 1,
		},
		200,
		{ 'Cache-Control': 'private, no-cache' },
	);
});

engagement.post('/newsletter', JSON_LIMIT, async (c) => {
	const ip = clientIp(c.req.raw);
	if (!ip) return c.json({ error: 'client_ip_unavailable' }, 400, NO_STORE);
	if (!(await withinLimit(c.env.NEWSLETTER_RATE_LIMITER, ip))) {
		return rateLimited();
	}

	const body = await readObject(c.req.raw);
	const email = canonicalEmail(body?.email);
	if (!email) return c.json({ error: 'invalid_email' }, 400, NO_STORE);

	const cancelToken = randomToken();
	const cancelTokenHash = await sha256(cancelToken);
	const database = drizzle(c.env.DATABASE);
	const inserted = await database
		.insert(newsletterSubscriptions)
		.values({
			email,
			cancelTokenHash,
			ip,
			createdAt: Date.now(),
		})
		.onConflictDoNothing({ target: newsletterSubscriptions.email })
		.returning({ email: newsletterSubscriptions.email });
	const subscriberCount = await rowCount(database, newsletterSubscriptions);

	if (inserted.length === 0) {
		return c.json({ email, subscriber_count: subscriberCount }, 200, NO_STORE);
	}
	return c.json(
		{ email, cancel_token: cancelToken, subscriber_count: subscriberCount },
		201,
		NO_STORE,
	);
});

engagement.delete('/newsletter', JSON_LIMIT, async (c) => {
	const ip = clientIp(c.req.raw);
	if (!ip) return c.json({ error: 'client_ip_unavailable' }, 400, NO_STORE);
	if (!(await withinLimit(c.env.NEWSLETTER_RATE_LIMITER, ip))) {
		return rateLimited();
	}

	const body = await readObject(c.req.raw);
	const email = canonicalEmail(body?.email);
	const token = body?.cancel_token;
	if (!email || typeof token !== 'string' || token.length > 128) {
		return c.json({ error: 'invalid_cancellation' }, 400, NO_STORE);
	}

	const tokenHash = await sha256(token);
	const database = drizzle(c.env.DATABASE);
	const deleted = await database
		.delete(newsletterSubscriptions)
		.where(
			and(
				eq(newsletterSubscriptions.email, email),
				eq(newsletterSubscriptions.cancelTokenHash, tokenHash),
			),
		)
		.returning({ email: newsletterSubscriptions.email });
	if (deleted.length === 0) {
		return c.json({ error: 'subscription_not_found' }, 404, NO_STORE);
	}

	const subscriberCount = await rowCount(database, newsletterSubscriptions);
	return c.json({ cancelled: true, subscriber_count: subscriberCount }, 200, NO_STORE);
});

engagement.put('/like', JSON_LIMIT, async (c) => {
	const ip = clientIp(c.req.raw);
	if (!ip) return c.json({ error: 'client_ip_unavailable' }, 400, NO_STORE);
	if (!(await withinLimit(c.env.LIKE_RATE_LIMITER, ip))) return rateLimited();

	const body = await readObject(c.req.raw);
	if (typeof body?.liked !== 'boolean') {
		return c.json({ error: 'invalid_like' }, 400, NO_STORE);
	}

	const database = drizzle(c.env.DATABASE);
	if (body.liked) {
		await database
			.insert(likes)
			.values({ ip, createdAt: Date.now() })
			.onConflictDoNothing({ target: likes.ip });
	} else {
		await database.delete(likes).where(eq(likes.ip, ip));
	}

	const likeCount = await rowCount(database, likes);
	return c.json({ liked: body.liked, like_count: likeCount }, 200, NO_STORE);
});

function clientIp(request: Request): string | undefined {
	return request.headers.get('CF-Connecting-IP') || undefined;
}

async function withinLimit(limiter: RateLimit, key: string): Promise<boolean> {
	return (await limiter.limit({ key })).success;
}

function rateLimited(): Response {
	return Response.json(
		{ error: 'rate_limited' },
		{
			status: 429,
			headers: { ...NO_STORE, 'Retry-After': '60' },
		},
	);
}

async function readObject(request: Request): Promise<Record<string, unknown> | undefined> {
	try {
		const body: unknown = await request.json();
		if (body && typeof body === 'object' && !Array.isArray(body)) {
			return body as Record<string, unknown>;
		}
	} catch {
		// A malformed or absent JSON body is handled as invalid input by the route.
	}
	return undefined;
}

function randomToken(): string {
	const bytes = crypto.getRandomValues(new Uint8Array(32));
	let binary = '';
	for (const byte of bytes) binary += String.fromCharCode(byte);
	return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

async function sha256(value: string): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value));
	return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

type Database = ReturnType<typeof drizzle>;
type CountableTable = typeof likes | typeof newsletterSubscriptions;

async function rowCount(database: Database, table: CountableTable): Promise<number> {
	const [result] = await database.select({ value: count() }).from(table);
	return result?.value ?? 0;
}

export default engagement;
