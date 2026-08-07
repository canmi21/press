import { browser, dev } from '$app/environment';
import { pickUrls } from '@canmi/urls';
import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';

export const ENGAGEMENT_QUERY_KEY = ['engagement'] as const;
export const ENGAGEMENT_STALE_TIME = 15 * 60 * 1_000;
export const ENGAGEMENT_CACHE_MAX_AGE = 7 * 24 * 60 * 60 * 1_000;

export type Engagement = {
	subscriber_count: number;
	like_count: number;
	liked: boolean;
};

type NewsletterResult = {
	email: string;
	cancel_token?: string;
	subscriber_count: number;
};

type LikeResult = {
	like_count: number;
	liked: boolean;
};

const apiUrl = pickUrls(dev).api;

export function createEngagementQuery() {
	return createQuery(() => ({
		queryKey: ENGAGEMENT_QUERY_KEY,
		queryFn: fetchEngagement,
		enabled: browser,
		staleTime: ENGAGEMENT_STALE_TIME,
		gcTime: ENGAGEMENT_CACHE_MAX_AGE,
		retry: 1,
	}));
}

export function createNewsletterMutation() {
	const client = useQueryClient();
	return createMutation<NewsletterResult, Error, string>(() => ({
		mutationFn: subscribe,
		onSuccess: (result) => {
			if (result.cancel_token) rememberSubscription(result.email, result.cancel_token);
			client.setQueryData<Engagement>(ENGAGEMENT_QUERY_KEY, (current) => ({
				subscriber_count: result.subscriber_count,
				like_count: current?.like_count ?? 0,
				liked: current?.liked ?? false,
			}));
		},
	}));
}

export function createLikeMutation() {
	const client = useQueryClient();
	return createMutation<LikeResult, Error, boolean, { previous?: Engagement }>(() => ({
		mutationFn: setLike,
		onMutate: async (liked) => {
			await client.cancelQueries({ queryKey: ENGAGEMENT_QUERY_KEY });
			const previous = client.getQueryData<Engagement>(ENGAGEMENT_QUERY_KEY);
			client.setQueryData<Engagement>(ENGAGEMENT_QUERY_KEY, (current) => ({
				subscriber_count: current?.subscriber_count ?? 0,
				like_count: Math.max(
					0,
					(current?.like_count ?? 0) + (liked === (current?.liked ?? false) ? 0 : liked ? 1 : -1),
				),
				liked,
			}));
			return { previous };
		},
		onError: (_error, _liked, context) => {
			if (context?.previous) {
				client.setQueryData(ENGAGEMENT_QUERY_KEY, context.previous);
			} else {
				client.removeQueries({ queryKey: ENGAGEMENT_QUERY_KEY, exact: true });
			}
		},
		onSuccess: (result) => {
			client.setQueryData<Engagement>(ENGAGEMENT_QUERY_KEY, (current) => ({
				subscriber_count: current?.subscriber_count ?? 0,
				like_count: result.like_count,
				liked: result.liked,
			}));
		},
		onSettled: () => client.invalidateQueries({ queryKey: ENGAGEMENT_QUERY_KEY }),
	}));
}

async function fetchEngagement(): Promise<Engagement> {
	const response = await fetch(`${apiUrl}/engagement`);
	return engagementResponse(response);
}

async function subscribe(email: string): Promise<NewsletterResult> {
	const response = await fetch(`${apiUrl}/newsletter`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ email }),
	});
	const result = await jsonResponse<NewsletterResult>(response);
	if (
		typeof result.email !== 'string' ||
		!validCount(result.subscriber_count) ||
		(result.cancel_token !== undefined && !/^[0-9a-f]{32}$/.test(result.cancel_token))
	) {
		throw new Error('invalid newsletter response');
	}
	return result;
}

async function setLike(liked: boolean): Promise<LikeResult> {
	const response = await fetch(`${apiUrl}/like`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ liked }),
	});
	const result = await jsonResponse<LikeResult>(response);
	if (typeof result.liked !== 'boolean' || !validCount(result.like_count)) {
		throw new Error('invalid like response');
	}
	return result;
}

async function engagementResponse(response: Response): Promise<Engagement> {
	const result = await jsonResponse<Engagement>(response);
	if (
		!validCount(result.subscriber_count) ||
		!validCount(result.like_count) ||
		typeof result.liked !== 'boolean'
	) {
		throw new Error('invalid engagement response');
	}
	return result;
}

async function jsonResponse<T>(response: Response): Promise<T> {
	if (!response.ok) throw new Error(`engagement request failed with ${response.status}`);
	return (await response.json()) as T;
}

function validCount(value: unknown): value is number {
	return Number.isSafeInteger(value) && (value as number) >= 0;
}

function rememberSubscription(email: string, cancelToken: string): void {
	if (!browser) return;
	try {
		localStorage.setItem('email', JSON.stringify({ email, cancel_token: cancelToken }));
	} catch {
		// Storage can be unavailable in privacy modes. The server-side subscription still worked.
	}
}
