<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import Counter from '$lib/components/counter.svelte';
	import {
		createEngagementQuery,
		createNewsletterMutation,
	} from '$lib/engagement/engagement.svelte';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let {
		class: className = '',
		locale,
	}: {
		class?: string;
		locale: LocaleCode;
	} = $props();

	const engagement = createEngagementQuery();
	const subscription = createNewsletterMutation();
	const subscribers = $derived(engagement.data?.subscriber_count ?? 0);
	let email = $state('');
	let status = $state<'idle' | 'sending' | 'done' | 'error'>('idle');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (status === 'sending') return;
		status = 'sending';
		try {
			const result = await subscription.mutateAsync(email);
			email = result.email;
			status = 'done';
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

	{#if status === 'done'}
		<!-- Same pill metrics as the form, so confirming does not shift the page under the
		reader's cursor. -->
		<p
			role="status"
			class="pill mt-4 flex items-center gap-2 rounded-full border border-border bg-paper px-5 text-text-soft"
		>
			<Check class="h-4 w-4 shrink-0" aria-hidden="true" />
			<span>{m['newsletter.confirm']({}, { locale })}</span>
		</p>
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
		{#if status === 'error'}
			<p role="alert" class="mt-3 text-[0.9375rem] text-text-soft">
				{m['newsletter.error']({}, { locale })}
			</p>
		{/if}

		<p class="mt-3.5 text-[0.9375rem] text-text-soft">
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
</section>

<style>
	/* Both ends sit at the column edge, so both are pulled. The formula is in styles/app.css. */
	.pill {
		--pill-height: 3.375rem;
		margin-inline: calc(-1 * var(--pill-overhang));
	}

</style>
