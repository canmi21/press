<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import Counter from '$lib/components/counter.svelte';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	// Standing in for the real figure until the subscriber count is served with the page.
	const PLACEHOLDER_SUBSCRIBERS = 1284;

	let {
		class: className = '',
		locale,
		subscribers = PLACEHOLDER_SUBSCRIBERS,
		onsubscribe = pretendToSubscribe,
	}: {
		class?: string;
		locale: LocaleCode;
		subscribers?: number;
		/** The seam the real endpoint plugs into; nothing else here knows where the address goes. */
		onsubscribe?: (email: string) => Promise<void>;
	} = $props();

	let email = $state('');
	let status = $state<'idle' | 'sending' | 'done'>('idle');

	// No endpoint yet. The delay exists so the sending state is visible rather than skipped.
	function pretendToSubscribe(_email: string): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, 700));
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (status === 'sending') return;
		status = 'sending';
		await onsubscribe(email);
		status = 'done';
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
			class="pill mt-4 flex items-center gap-2 rounded-full border border-border bg-paper p-1.5 pl-5"
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
				class="min-w-0 flex-1 bg-transparent text-text placeholder:text-text-soft disabled:text-text-soft"
			/>
			<button
				type="submit"
				disabled={status === 'sending'}
				aria-busy={status === 'sending'}
				class="h-full shrink-0 rounded-full bg-ink px-4 font-medium text-page transition-opacity duration-200 hover:opacity-85 disabled:opacity-60"
			>
				{m['newsletter.subscribe']({}, { locale })}
			</button>
		</form>

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

	/* The input has no border of its own, so its focus ring belongs to the pill around it --
	otherwise the ring draws inside the pill and reads as a second, misaligned edge. Same move
	as the article card, which hands its row ring to the sheet icon. */
	.pill:has(input:focus-visible) {
		outline: var(--focus-ring-width) solid transparent;
		outline-offset: var(--focus-ring-offset);
		box-shadow:
			0 0 0 var(--focus-ring-offset) var(--focus-ring-gap),
			0 0 0 calc(var(--focus-ring-offset) + var(--focus-ring-width)) var(--focus-ring-color);
	}

	.pill input:focus-visible {
		outline: none;
		box-shadow: none;
	}
</style>
