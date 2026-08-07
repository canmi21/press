import { browser, dev } from '$app/environment';
import { pickUrls } from '@canmi/urls';
import { createQuery } from '@tanstack/svelte-query';
import { QUERY_CACHE_MAX_AGE, QUERY_STALE_TIME } from '$lib/query';

export const READS_QUERY_KEY = 'reads';

export type Reads = {
	slug: string;
	read_count: number;
};

const apiUrl = pickUrls(dev).api;

/**
 * The article's read count, counting this visit as one of them.
 *
 * A query rather than a mutation, even though the request has an effect. What the page wants
 * is the number, and the number is what has to survive a reload -- mutations are deliberately
 * never persisted, so a count written from one would be gone by the next visit. As a query it
 * lands in the same `localStorage["cache"]` container everything else uses, which is what lets
 * a returning reader see the previous number immediately instead of an empty space.
 *
 * Counting is therefore tied to the query firing, so refetch triggers are turned off: a read
 * is somebody opening the article, not somebody coming back to the tab. The server holds the
 * same line from its side at one count per IP per article per minute.
 */
export function createReadsQuery(slug: () => string) {
	return createQuery(() => ({
		queryKey: [READS_QUERY_KEY, slug()],
		queryFn: () => countRead(slug()),
		enabled: browser,
		staleTime: QUERY_STALE_TIME,
		gcTime: QUERY_CACHE_MAX_AGE,
		refetchOnWindowFocus: false,
		refetchOnReconnect: false,
		retry: 1,
	}));
}

async function countRead(slug: string): Promise<Reads> {
	const response = await fetch(`${apiUrl}/read`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ slug }),
	});
	if (!response.ok) throw new Error(`read request failed with ${response.status}`);

	const result = (await response.json()) as Reads;
	if (result.slug !== slug || !Number.isSafeInteger(result.read_count) || result.read_count < 0) {
		throw new Error('invalid read response');
	}
	return result;
}
