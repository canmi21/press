<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let {
		locale,
		subscribers,
		onsubscribe = pretendToSubscribe,
	}: {
		locale: LocaleCode;
		subscribers: number;
		/** The seam the real endpoint plugs into; nothing else here knows where the address goes. */
		onsubscribe?: (email: string) => Promise<void>;
	} = $props();

	let email = $state('');
	let status = $state<'idle' | 'sending' | 'done'>('idle');

	// No endpoint yet. The delay exists so the sending state is visible rather than skipped.
	function pretendToSubscribe(_email: string): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, 700));
	}

	const digits = $derived(String(Math.max(0, Math.trunc(subscribers))));

	// One cell per digit, with a wider gap where a thousands separator would otherwise go. The
	// grouping is drawn rather than formatted, so no locale supplies a separator character.
	const digitCells = $derived(
		[...digits].map((digit, index, all) => ({
			digit,
			grouped: index > 0 && (all.length - index) % 3 === 0,
		})),
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (status === 'sending') return;
		status = 'sending';
		await onsubscribe(email);
		status = 'done';
	}
</script>

<section aria-labelledby="newsletter-heading" class="mt-16">
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
				inputs={{ count: digits }}
				options={{ locale }}
			>
				{#snippet cells()}
					<!-- Drawn from `digitCells` rather than the message's own text, so each digit keeps
					its box; the markup tag only records where a translator wants the number. -->
					<!-- Baseline, not text-bottom: an inline-flex takes its baseline from its first item,
					so the cells hand the digit's own baseline up to the sentence. -->
					<span class="digit-cells mx-1 inline-flex">
						{#each digitCells as cell, i (i)}
							<span
								class="ml-px flex h-6 w-4.5 items-center justify-center rounded-[0.25rem] border border-border bg-paper font-mono text-[0.8125rem] text-text-strong tabular-nums"
								class:grouped={cell.grouped}>{cell.digit}</span
							>
						{/each}
					</span>
				{/snippet}
			</ParaglideMessage>
		</p>
	{/if}
</section>

<style>
	.pill {
		/* Pinned rather than derived from padding, because the optical compensation below needs a
		   radius it can compute from. */
		--pill-height: 3.375rem;
		--pill-radius: calc(var(--pill-height) / 2);
		/* A pill's edge, averaged down its own height, sits (1 - pi/4)r inside its box: the two
		   caps remove exactly that much of the area a flush rectangle would cover. Pushing the box
		   out by the same amount makes it read as the width of the text column rather than as an
		   indent. Lower the coefficient if it overshoots; 0 is the uncompensated box. */
		--pill-overhang: calc(var(--pill-radius) * 0.2146);
		height: var(--pill-height);
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

	/* Digits sit wholly above the baseline, so a box aligned by baseline lands about 0.08em below
	   the ink centre of the text around it. Measured on this page that centre is 0.370em above
	   the baseline for Latin and 0.360em for CJK, so one shift serves both scripts and a
	   per-script calibration would be arguing over 0.15px. Only CJK makes the error visible: its
	   ink block is taller and denser, so there is something solid to judge the box against.

	   Transform rather than vertical-align, which grows the line box and drags the baseline along
	   with the cells, cancelling most of its own correction. */
	.digit-cells {
		transform: translateY(-0.08em);
	}

	/* Where the comma was. */
	.grouped {
		margin-left: 0.3125rem;
	}
</style>
