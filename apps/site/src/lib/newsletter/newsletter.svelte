<script lang="ts">
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
	import {
		REVERSE,
		REVERSE_TOTAL,
		SEQUENCE,
		SEQUENCE_TOTAL,
		sequenceStyle,
	} from '$lib/newsletter/sequence';
	import * as m from '$lib/paraglide/messages';

	let {
		class: className = '',
		locale,
		offer = false,
	}: {
		class?: string;
		locale: LocaleCode;
		/**
		 * Offer a subscription rather than stand as a fixture: present only on a device that
		 * holds no subscription of its own.
		 *
		 * The homepage takes the default and is always there, because it is where a subscriber
		 * goes to cancel and something has to stay reachable. After an article the section is an
		 * offer, and an offer put to somebody who already accepted it is the thing this removes.
		 */
		offer?: boolean;
	} = $props();

	const engagement = createEngagementQuery();
	const newsletter = createNewsletterMutation();
	const cancellation = createCancelMutation();
	const subscribers = $derived(engagement.data?.subscriber_count ?? 0);
	let email = $state('');
	let status = $state<'idle' | 'sending' | 'confirmed' | 'error' | 'cancelled'>('idle');
	let subscription = $state<Subscription | undefined>();
	let confirmed = $state<string | undefined>();
	/**
	 * The address as it was typed, kept only long enough to hand the swap something to animate
	 * away from. Set by an interaction and never by the record read at mount, which is why a
	 * returning reader arrives at the confirmed pill already still.
	 */
	let entering = $state<string | undefined>();
	/**
	 * Where the transition has got to. `still` covers both ends of it: nothing is running, either
	 * because nothing has happened yet or because everything already has.
	 */
	let stage = $state<'still' | 'redacting' | 'settling' | 'undoing' | 'reverting' | 'restoring'>(
		'still',
	);
	let timers: ReturnType<typeof setTimeout>[] = [];

	/**
	 * Whether an offered section has been put on the page.
	 *
	 * **It is set once, at mount, and nothing ever clears it.** Deriving presence from the record
	 * instead would delete the section at the moment somebody subscribed inside it -- 2.1 seconds
	 * of confirmation playing inside an element that is removing itself. Latched, subscribing here
	 * keeps everything it put on screen, the pill and the control that undoes it alike, until the
	 * page is left; the next article is the first one to omit the section.
	 *
	 * That is the line the confirmation copy already sits on. What the reader just did lasts one
	 * visit, and what they are is what the next load reads.
	 */
	let appended = $state(false);
	const present = $derived(!offer || appended);

	// The record is on the reader's device, so the server renders the form and this replaces it
	// after mount. Both states are one pill tall, so the swap moves nothing around it. An offered
	// section has nothing to replace: the server sends none, and one is appended here if this
	// device turns out to have no subscription.
	$effect(() => {
		// Read into a local and test that, never the state field. Testing `subscription` would make
		// this effect depend on a value it also writes, so submitting -- which re-reads the record --
		// would re-run it, and the cleanup below would clear the timers of the sequence still playing.
		const record = readSubscription();
		subscription = record;
		if (!record) appended = true;
		return stop;
	});

	function stop() {
		for (const timer of timers) clearTimeout(timer);
		timers = [];
	}

	function after(delay: number, run: () => void) {
		timers.push(setTimeout(run, delay));
	}

	// Subscribing twice from a device that never held the token confirms the address without
	// offering to cancel it. That device genuinely cannot, and an unsubscribe control that always
	// fails would be worse than not showing one.
	const shown = $derived(subscription?.email ?? confirmed);
	const masked = $derived(shown ? maskEmail(shown) : '');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (status === 'sending') return;
		status = 'sending';
		const typed = email;
		try {
			const result = await newsletter.mutateAsync(email);
			email = '';
			confirmed = result.email;
			subscription = readSubscription();
			// Reduced motion arrives at the end of the sequence directly. Every stage of it exists
			// to be watched, so with nothing to watch there is only the state it lands on.
			if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
				status = 'confirmed';
				return;
			}
			// Only for this visit. A reload lands on the subscriber count, since by then the pill
			// already says the reader is on the list and the sentence has been read.
			entering = typed;
			stage = 'redacting';
			after(SEQUENCE.row.at, () => {
				status = 'confirmed';
				stage = 'settling';
			});
			after(SEQUENCE.undo.at, () => (stage = 'undoing'));
			after(SEQUENCE_TOTAL, () => {
				entering = undefined;
				stage = 'still';
			});
		} catch {
			status = 'error';
		}
	}

	async function unsubscribe() {
		// `reverting` is part of the guard, not just `isPending`: the record is held on screen after
		// the request has returned, and a second click would spend it on a subscription that is
		// already gone.
		if (!subscription || cancellation.isPending || stage === 'reverting') return;
		const record = subscription;
		try {
			await cancellation.mutateAsync(record);
			stop();
			entering = undefined;
			if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
				settle();
				return;
			}
			// The pill keeps showing the address it is undoing until the sequence has taken it back
			// off, so the record outlives the request that cancelled it by exactly that long.
			stage = 'reverting';
			after(REVERSE.form.at, () => {
				settle();
				stage = 'restoring';
			});
			after(REVERSE_TOTAL, () => (stage = 'still'));
		} catch {
			status = 'error';
		}
	}

	function settle() {
		subscription = undefined;
		confirmed = undefined;
		stage = 'still';
		status = 'cancelled';
	}
</script>

<!-- Both labels are laid out in every state and the unused one is only made invisible, so the
button is as wide as the wider of the two and its edge does not move when the copy changes. The
alternative is animating a width the stylesheet cannot know, which is a measurement this does not
otherwise need. See spec/engagement.md. -->
{#snippet label(subscribed: boolean)}
	<span class="labels" class:crossfading={entering} class:recrossing={stage === 'reverting'}>
		<span class:spent={subscribed} aria-hidden={subscribed}>
			{m['newsletter.subscribe']({}, { locale })}
		</span>
		<span class:spent={!subscribed} aria-hidden={!subscribed}>
			{m['newsletter.subscribed']({}, { locale })}
		</span>
	</span>
{/snippet}

<!-- The gap above belongs to whatever this follows, so it comes from the caller: after an article
     it separates two offerings, after the collected notes it closes a smaller block of apparatus,
     and those are not the same distance. A component that carried one number would make the
     shorter one an override fighting it. -->
{#if present}
	<section
		aria-labelledby="newsletter-heading"
		class="pill-metrics {className}"
		style={sequenceStyle()}
	>
		<h2 id="newsletter-heading" class="selectable mb-3 font-medium text-text-strong">
			{m['newsletter.heading']({}, { locale })}
		</h2>

		<!-- Two readings of one pitch, chosen by width. A phone gets the shorter one, which is the
		     same invitation in fewer sentences rather than a different offer -- the long version
		     runs to six lines there in English and five in German, which is a paragraph to read
		     before reaching the field it is asking you to fill in.

		     Both are in the markup and one is `display: none`, so a screen reader is read exactly
		     one of them. The bio does this with markers inside its markdown; a message has no
		     markdown to mark, so the choice is made here. See spec/styling.md. -->
		<p class="selectable hidden text-text-soft sm:block">
			{m['newsletter.pitch']({}, { locale })}
		</p>
		<p class="selectable text-text-soft sm:hidden">
			{m['newsletter.pitch.short']({}, { locale })}
		</p>

		<!-- One pill across both states. The box, its border and the button's place never move; only
		what sits in them is replaced, which is what leaves the swap something to animate rather than
		something to jump between. -->
		<div
			class="pill focus-input-shell mt-4 flex items-center gap-2 rounded-full border border-border bg-paper p-1.5 pl-5"
			role={shown ? 'status' : undefined}
		>
			{#if shown}
				<span aria-hidden="true" class="swap min-w-0 flex-1">
					{#if entering}
						<!-- A plain copy of what was typed, standing in for the field that has just gone so
						the address appears to be redacted in place rather than replaced. -->
						<span class="typed">{entering}</span>
					{/if}
					<!-- The address is already unreadable, so nothing is gained by letting it wrap. -->
					<span
						class="masked text-text"
						class:revealing={entering}
						class:dissolving={stage === 'reverting'}
					>
						{masked}
					</span>
				</span>
				<!-- The button's surface stays and its copy states the outcome, so the shape the reader
				just used becomes the label for what it did. It is inert -- there is nothing left to
				submit -- and it is the pill's whole accessible content, the masked address being of no
				use read aloud. -->
				<span
					class="chip flex h-full shrink-0 items-center rounded-full px-4 font-medium"
					class:cooling={entering}
					class:warming={stage === 'reverting'}
				>
					{@render label(stage !== 'reverting')}
				</span>
			{:else}
				<!-- `type="email"` plus `required` leaves validation to the browser: it is localized
				already, it reports before any request is made, and it needs no JavaScript.
				`display: contents` keeps the form out of the shared pill's layout. -->
				<form onsubmit={submit} class="contents">
					<input
						type="email"
						name="email"
						bind:value={email}
						required
						autocomplete="email"
						placeholder="you@example.com"
						aria-label={m['newsletter.email']({}, { locale })}
						disabled={status === 'sending'}
						class:returning={stage === 'restoring'}
						class="focus-input min-w-0 flex-1 bg-transparent text-text placeholder:text-text-soft disabled:text-text-soft"
					/>
					<button
						type="submit"
						disabled={status === 'sending'}
						aria-busy={status === 'sending'}
						class:reviving={stage === 'restoring'}
						class="focus-ring h-full shrink-0 rounded-full bg-ink px-4 font-medium text-page transition-opacity duration-200 hover:opacity-85 disabled:opacity-60"
					>
						{@render label(false)}
					</button>
				</form>
			{/if}
		</div>

		<!-- One row under the pill in every state, so nothing below the section moves as it changes.
		The left slot carries whatever the reader most recently needs to know and falls back to the
		count; the right slot is the only place a destructive action appears. -->
		<div
			class="row mt-3.5 flex items-baseline justify-between gap-6 text-[0.9375rem] text-text-soft"
		>
			{#if status === 'error'}
				<p class="selectable" role="alert">{m['newsletter.error']({}, { locale })}</p>
			{:else if status === 'cancelled'}
				<p class="selectable" role="status" class:returning={stage === 'restoring'}>
					{m['newsletter.unsubscribed']({}, { locale })}
				</p>
			{:else if status === 'confirmed'}
				<p
					class="selectable"
					role="status"
					class:arriving={entering}
					class:departing={stage === 'reverting'}
				>
					{m['newsletter.confirm']({}, { locale })}
				</p>
			{:else}
				<p class="selectable" class:leaving={entering}>
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

			<!-- Held back until the sequence reaches it. A control that undoes what the reader is still
			watching happen has nothing to undo yet, and it arrives directly below the button they just
			pressed, where a second click would otherwise land on it. -->
			{#if subscription && stage !== 'redacting' && stage !== 'settling'}
				<!-- Centred under the button above, in a cell the button's own width decides.

				     That width is not a number anybody can write: it is the wider of two labels as
				     this font renders them, and it moves with the language. Measuring it would mean
				     painting at one position and shifting after hydration, which is the failure
				     styling.md records for the rail. So the cell reserves the width the same way the
				     button does -- by laying both labels out and hiding them -- and the control
				     centres inside whatever that comes to. -->
				<span class="under-chip shrink-0">
					<span class="ghost px-4 text-base font-medium" aria-hidden="true"
						>{@render label(true)}</span
					>
					<button
						type="button"
						onclick={unsubscribe}
						disabled={cancellation.isPending || stage === 'reverting'}
						aria-busy={cancellation.isPending}
						class:arriving={stage === 'undoing'}
						class:departing={stage === 'reverting'}
						class="focus-link spring-underline transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong disabled:opacity-60"
					>
						{m['newsletter.unsubscribe']({}, { locale })}
					</button>
				</span>
			{/if}
		</div>
	</section>
{/if}

<style>
	/* Set on the section rather than on the pill, so the row below can measure from it too.
	   `.pill-metrics` above derives the radius and the overhang from this one number. */
	section {
		--pill-height: 3.375rem;
	}

	/* Both ends sit at the column edge, so both are pulled. The formula is in styles/app.css. */
	.pill {
		margin-inline: calc(-1 * var(--pill-overhang));
		/* Stated rather than left to `auto`, which draws an I-beam wherever it lands on text. The
		   only text here in the confirmed state is the masked address, and selecting that yields a
		   row of bullets rather than an address -- an invitation to copy something that is not
		   there. The field below re-asserts what a field is. */
		cursor: default;
	}

	.pill input {
		cursor: text;
	}

	/* Everything in this section that can be pressed says so, and nothing else does. The two here
	   are a `button` and a `button`, which browsers draw with an arrow; the inert chip above keeps
	   `not-allowed` because it is the one shaped like a control and is not one. */
	.pill button,
	.under-chip button {
		cursor: pointer;
	}

	/* The row ends where the button above it ends, so the cell below the button is the button.
	   The pill is drawn wider than the column by its overhang, and carries a border and a padding
	   inside that, so the button's box ends this far in. */
	.row {
		padding-inline-end: calc(0.375rem + 1px - var(--pill-overhang));
	}

	/* One cell, as wide as the button above, holding the hidden labels that decide that width and
	   the control that centres inside it. */
	.under-chip {
		display: inline-grid;
		place-items: center;
	}

	.under-chip > * {
		grid-area: 1 / 1;
	}

	/* It reserves width and paints nothing. `visibility` rather than `display` for the usual
	   reason: a removed box measures nothing.

	   It has to carry the button's own type, not the row's: this row is a size smaller and a
	   weight lighter, and either difference makes the reserved cell narrower than the thing it is
	   standing in for. The classes on it mirror `px-4` and `font-medium` on the button, plus the
	   base size the pill inherits and this row overrides. */
	.ghost {
		visibility: hidden;
		white-space: nowrap;
	}

	/* Both addresses occupy one cell, so the masked form arrives exactly where the field's text
	   was rather than beside it. */
	.swap {
		display: inline-grid;
		align-items: center;
	}

	.swap > span {
		grid-area: 1 / 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
	}

	.typed {
		color: var(--color-text);
		animation: lift var(--typed-for) ease var(--typed-at) both;
	}

	/* Ink is for the thing worth pressing. Once pressed this is a label, so it keeps the shape and
	   gives up the emphasis; cooling out of ink over the same span as the reveal shows it is the
	   same control settling rather than a different one appearing. */
	.chip {
		background: var(--color-paper-hover);
		color: var(--color-text-soft);
		/* It keeps the button's shape on purpose -- that is what makes the swap read as one
		   control settling rather than a second one appearing -- and the shape is the problem: a
		   pill in the place a pill was just pressed invites a second press. There is nothing left
		   to submit, so the pointer says so before the click happens. */
		cursor: not-allowed;
	}

	.cooling {
		animation: cool var(--chip-for) var(--ease-spring) var(--chip-at) both;
	}

	.warming {
		animation: cool var(--back-chip-for) var(--ease-spring) var(--back-chip-at) both reverse;
	}

	@keyframes cool {
		from {
			background-color: var(--color-ink);
			color: var(--color-page);
		}
		to {
			background-color: var(--color-paper-hover);
			color: var(--color-text-soft);
		}
	}

	.labels {
		display: inline-grid;
		place-items: center;
	}

	.labels > span {
		grid-area: 1 / 1;
		white-space: nowrap;
	}

	/* `visibility` rather than `display`, which is the whole point: the box still measures. Which
	   of the two is read aloud is marked separately, since the crossfade below has to turn this
	   back on to paint it. */
	.spent {
		visibility: hidden;
	}

	.crossfading > span,
	.recrossing > span {
		animation-timing-function: var(--ease-spring);
		animation-fill-mode: both;
	}

	.crossfading > span {
		animation-duration: var(--chip-for);
		animation-delay: var(--chip-at);
	}

	.recrossing > span {
		animation-duration: var(--back-chip-for);
		animation-delay: var(--back-chip-at);
	}

	.crossfading > .spent,
	.recrossing > .spent {
		animation-name: spend;
	}

	.crossfading > :not(.spent),
	.recrossing > :not(.spent) {
		animation-name: take;
	}

	@keyframes spend {
		from {
			visibility: visible;
			opacity: 1;
		}
		to {
			visibility: visible;
			opacity: 0;
		}
	}

	@keyframes take {
		from {
			opacity: 0;
		}
	}

	/* The line under the pill: the count clears out, then the confirmation arrives in its place. */
	.leaving {
		animation: lift var(--count-for) ease var(--count-at) both;
	}

	.arriving {
		animation: arrive var(--row-for) var(--ease-spring) both;
	}

	button.arriving {
		animation-duration: var(--undo-for);
	}

	/* The line under the pill leaves along the path it arrived by, so the two states are visibly one
	   thing changing rather than two that happen to occupy the same row. */
	.departing {
		animation: arrive var(--back-undo-for) var(--ease-spring) var(--back-undo-at) both reverse;
	}

	.returning {
		animation: arrive var(--back-form-for) var(--ease-spring) both;
	}

	/* The button does not fade in with the rest: it is the shape the chip has just finished warming
	   back into, arriving at the same ink it was handed, and fading it would blink the one element
	   that was continuous across the swap. It springs instead -- the moment it can be pressed again
	   is worth marking, and scale carries that without touching the colour that made it continuous. */
	.reviving {
		animation: revive var(--back-form-for) var(--ease-spring) both;
	}

	@keyframes revive {
		from {
			transform: scale(0.92);
		}
	}

	@keyframes arrive {
		from {
			transform: translateY(0.375rem);
			opacity: 0;
		}
	}

	/* Uncovered left to right, so the address reads as being redacted in place. A clip needs no
	   measurement, unlike the Support rail's masks: that geometry depends on the rendered width of
	   two labels, and this one is always the whole box. See spec/styling.md. */
	/* Not the spring. A spring is a settle: it spends 97% of the distance in the first half and
	   leaves the rest of the stage with nothing visibly happening. This sweep is meant to be
	   watched across its whole duration, so it eases in and out instead. */
	.revealing {
		animation: redact var(--redact-for) cubic-bezier(0.65, 0, 0.35, 1) var(--redact-at) both;
	}

	/* Not the sweep reversed. Redacting is something done to the address, edge and all; letting it
	   go is not, and a mirrored wipe would say the site was busy taking it back rather than simply
	   no longer holding it. So it goes soft instead of directional: blurred out of focus, drifting
	   very slightly, gone. */
	.dissolving {
		animation: dissolve var(--back-dissolve-for) ease-in var(--back-dissolve-at) both;
	}

	@keyframes dissolve {
		to {
			transform: translateX(0.5rem);
			opacity: 0;
			filter: blur(0.1875rem);
		}
	}

	@keyframes lift {
		to {
			transform: translateY(-0.375rem);
			opacity: 0;
		}
	}

	@keyframes redact {
		from {
			/* Vertical slack: an inset of zero would clip descenders against the line box. */
			clip-path: inset(-0.5rem 100% -0.5rem 0);
		}
		to {
			clip-path: inset(-0.5rem 0 -0.5rem 0);
		}
	}
</style>
