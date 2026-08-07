import { browser, dev } from '$app/environment';
import { pickUrls } from '@canmi/urls';
import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
import { QUERY_CACHE_MAX_AGE, QUERY_STALE_TIME } from '$lib/query';

export const ENGAGEMENT_QUERY_KEY = ['engagement'] as const;

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

type CancelResult = {
	subscriber_count?: number;
};

/** What the browser holds about its own subscription. See spec/engagement.md. */
export type Subscription = {
	email: string;
	cancel_token: string;
};

const SUBSCRIPTION_KEY = 'email';
const CANCEL_TOKEN = /^[0-9a-f]{32}$/;

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
		staleTime: QUERY_STALE_TIME,
		gcTime: QUERY_CACHE_MAX_AGE,
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

export function createCancelMutation() {
	const client = useQueryClient();
	return createMutation<CancelResult, Error, Subscription>(() => ({
		mutationFn: cancel,
		onSuccess: (result) => {
			forgetSubscription();
			if (result.subscriber_count === undefined) return;
			client.setQueryData<Engagement>(ENGAGEMENT_QUERY_KEY, (current) => ({
				subscriber_count: result.subscriber_count ?? 0,
				like_count: current?.like_count ?? 0,
				liked: current?.liked ?? false,
			}));
		},
		onSettled: () => client.invalidateQueries({ queryKey: ENGAGEMENT_QUERY_KEY }),
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
		(result.cancel_token !== undefined && !CANCEL_TOKEN.test(result.cancel_token))
	) {
		throw new Error('invalid newsletter response');
	}
	return result;
}

async function cancel(subscription: Subscription): Promise<CancelResult> {
	const response = await fetch(`${apiUrl}/newsletter`, {
		method: 'DELETE',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(subscription),
	});
	// A subscription the server no longer has is the state being asked for, not a failure. The
	// record is stale -- most likely cancelled from another browser -- and reporting an error
	// would leave the reader looking at a subscription they cannot get rid of.
	if (response.status === 404) return {};

	const result = await jsonResponse<CancelResult>(response);
	if (!validCount(result.subscriber_count)) throw new Error('invalid cancellation response');
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
		localStorage.setItem(SUBSCRIPTION_KEY, JSON.stringify({ email, cancel_token: cancelToken }));
	} catch {
		// Storage can be unavailable in privacy modes. The server-side subscription still worked.
	}
}

/**
 * Only ever called after mount: this record is the reader's own device state, so the server
 * cannot render it and must not try.
 */
export function readSubscription(): Subscription | undefined {
	if (!browser) return undefined;
	let stored: string | null = null;
	try {
		stored = localStorage.getItem(SUBSCRIPTION_KEY);
	} catch {
		return undefined;
	}
	if (stored === null) return undefined;

	try {
		const record: unknown = JSON.parse(stored);
		if (!record || typeof record !== 'object') return undefined;
		const { email, cancel_token: token } = record as Partial<Subscription>;
		// A token that cannot be spent is worse than none: it would render an unsubscribe control
		// whose every use fails. An unreadable record is dropped rather than shown.
		if (
			typeof email !== 'string' ||
			!email ||
			typeof token !== 'string' ||
			!CANCEL_TOKEN.test(token)
		) {
			forgetSubscription();
			return undefined;
		}
		return { email, cancel_token: token };
	} catch {
		forgetSubscription();
		return undefined;
	}
}

export function forgetSubscription(): void {
	if (!browser) return;
	try {
		localStorage.removeItem(SUBSCRIPTION_KEY);
	} catch {
		// Nothing to do: the record is unreachable, which is the state being asked for.
	}
}
