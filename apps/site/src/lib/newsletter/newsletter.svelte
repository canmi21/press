<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import Counter from '$lib/components/counter.svelte';
	import {
		createCancelMutation,
		createEngagementQuery,
		createNewsletterMutation,
		readSubscription,
		type Subscription,
	} from '$lib/engagement/engagement.svelte';
	import type { LocaleCode } from '$lib/locale';
	import { maskEmail } from '$lib/newsletter/mask';
	import * as m from '$lib/paraglide/messages';

	let {
		class: className = '',
		locale,
	}: {
		class?: string;
		locale: LocaleCode;
	} = $props();

	const engagement = createEngagementQuery();
	const newsletter = createNewsletterMutation();
	const cancellation = createCancelMutation();
	const subscribers = $derived(engagement.data?.subscriber_count ?? 0);
	let email = $state('');
	let status = $state<'idle' | 'sending' | 'confirmed' | 'error' | 'cancelled'>('idle');
	let subscription = $state<Subscription | undefined>();
	let confirmed = $state<string | undefined>();

	// The record is on the reader's device, so the server renders the form and this replaces it
	// after mount. Both states are one pill tall, so the swap moves nothing around it.
	$effect(() => {
		subscription = readSubscription();
	});

	// Subscribing twice from a device that never held the token confirms the address without
	// offering to cancel it. That device genuinely cannot, and an unsubscribe control that always
	// fails would be worse than not showing one.
	const shown = $derived(subscription?.email ?? confirmed);
	const masked = $derived(shown ? maskEmail(shown) : '');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (status === 'sending') return;
		status = 'sending';
		try {
			const result = await newsletter.mutateAsync(email);
			email = '';
			// Only for this visit. A reload lands on the subscriber count, since by then the pill
			// already says the reader is on the list and the sentence has been read.
			status = 'confirmed';
			confirmed = result.email;
			subscription = readSubscription();
		} catch {
			status = 'error';
		}
	}

	async function unsubscribe() {
		if (!subscription || cancellation.isPending) return;
		const record = subscription;
		try {
			await cancellation.mutateAsync(record);
			subscription = undefined;
			confirmed = undefined;
			status = 'cancelled';
		} catch {
			status = 'error';
		}
	}
</script>

<section aria-labelledby="newsletter-heading" class="mt-16 {className}">
	<h2 id="newsletter-heading" class="mb-3 font-medium text-text-strong">
		{m['newsletter.heading']({}, { locale })}
	</h2>

	<p class="text-pretty text-text-soft">{m['newsletter.pitch']({}, { locale })}</p>

	{#if shown}
		<!-- Same pill metrics as the form, so confirming does not shift the page under the
		reader's cursor. The check occupies the submit button's place rather than replacing the
		whole row, which keeps the shape the reader just used. -->
		<div
			role="status"
			class="pill mt-4 flex items-center gap-2 rounded-full border border-border bg-paper p-1.5 pl-5"
		>
			<span class="sr-only">{m['newsletter.subscribed']({}, { locale })}</span>
			<!-- The address is already unreadable, so nothing is gained by letting it wrap or
			overflow the pill. -->
			<span aria-hidden="true" class="min-w-0 flex-1 truncate text-text">{masked}</span>
			<span
				class="flex aspect-square h-full shrink-0 items-center justify-center rounded-full bg-ink text-page"
			>
				<Check class="h-4 w-4" aria-hidden="true" />
			</span>
		</div>
	{:else}
		<!-- `type="email"` plus `required` leaves validation to the browser: it is localized
		already, it reports before any request is made, and it needs no JavaScript. -->
		<form
			onsubmit={submit}
			class="pill focus-input-shell mt-4 flex items-center gap-2 rounded-full border border-border bg-paper p-1.5 pl-5"
		>
			<input
				type="email"
				name="email"
				bind:value={email}
				required
				autocomplete="email"
				placeholder="you@example.com"
				aria-label={m['newsletter.email']({}, { locale })}
				disabled={status === 'sending'}
				class="focus-input min-w-0 flex-1 bg-transparent text-text placeholder:text-text-soft disabled:text-text-soft"
			/>
			<button
				type="submit"
				disabled={status === 'sending'}
				aria-busy={status === 'sending'}
				class="focus-ring h-full shrink-0 rounded-full bg-ink px-4 font-medium text-page transition-opacity duration-200 hover:opacity-85 disabled:opacity-60"
			>
				{m['newsletter.subscribe']({}, { locale })}
			</button>
		</form>
	{/if}

	<!-- One row under the pill in every state, so nothing below the section moves as it changes.
	The left slot carries whatever the reader most recently needs to know and falls back to the
	count; the right slot is the only place a destructive action appears. -->
	<div
		class="mt-3.5 flex items-baseline justify-between gap-6 text-[0.9375rem] text-text-soft"
	>
		{#if status === 'error'}
			<p role="alert">{m['newsletter.error']({}, { locale })}</p>
		{:else if status === 'cancelled'}
			<p role="status">{m['newsletter.unsubscribed']({}, { locale })}</p>
		{:else if status === 'confirmed'}
			<p role="status">{m['newsletter.confirm']({}, { locale })}</p>
		{:else}
			<p>
				<ParaglideMessage
					message={m['newsletter.subscribers']}
					inputs={{ count: subscribers }}
					options={{ locale }}
				>
					<!-- The cells are drawn rather than taken from the message's own text; the markup tag
					only records where a translator wants the number to sit. -->
					{#snippet cells()}
						<Counter value={subscribers} />
					{/snippet}
				</ParaglideMessage>
			</p>
		{/if}

		{#if subscription}
			<button
				type="button"
				onclick={unsubscribe}
				disabled={cancellation.isPending}
				aria-busy={cancellation.isPending}
				class="focus-link spring-underline shrink-0 transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong disabled:opacity-60"
			>
				{m['newsletter.unsubscribe']({}, { locale })}
			</button>
		{/if}
	</div>
</section>

<style>
	/* Both ends sit at the column edge, so both are pulled. The formula is in styles/app.css. */
	.pill {
		--pill-height: 3.375rem;
		margin-inline: calc(-1 * var(--pill-overhang));
	}

</style>
