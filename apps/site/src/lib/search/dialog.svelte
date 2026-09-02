<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
	import { Dialog } from 'bits-ui';
	import { goto } from '$app/navigation';
	import { animateHeight, type AnimationControl } from '$lib/client/collapse';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import { m } from '$lib/paraglide/messages';
	import type { LocaleCode } from '$lib/locale/index.ts';
	import { groupHits, markup, search, worthSearching, type SearchHit } from './index.ts';

	let { locale }: { locale: LocaleCode } = $props();

	/**
	 * How long the field stays quiet before a request goes out.
	 *
	 * The middle of the three options: searching per keystroke spends ten requests on one search,
	 * and searching on Enter asks for a keypress that the pause already tells us about. 300ms
	 * rather than 250 because the difference is below what a typist notices and above what the
	 * monthly request budget notices -- see spec/search.md on which limit binds first.
	 */
	const QUIET_MS = 300;

	let open = $state(false);
	let query = $state('');
	let hits = $state<SearchHit[]>([]);
	let active = $state(0);
	let searching = $state(false);
	let failed = $state(false);
	let input = $state<HTMLInputElement>();
	let bodyEl = $state<HTMLElement>();
	/** Hidden overflow and a pinned height belong to the move, not to the panel at rest. */
	let bodyMoving = $state(false);
	let bodyMotion: AnimationControl | undefined;
	/**
	 * What the body was last measured against.
	 *
	 * `undefined` means nothing has been measured yet, which is the state a freshly opened dialog
	 * is in: the panel plays its own entrance, and a body animating its height inside that would
	 * be two moves arguing about the same edge. So the first pass records the height it was given
	 * and animates nothing.
	 */
	let bodyState: string | undefined;
	/**
	 * The height the body had before the contents it is holding were replaced.
	 *
	 * Read in `$effect.pre` because a plain effect is too late: by the time it runs the box has
	 * already been laid out around the new results, so asking it where it is would answer with
	 * where it is going and the move would be a jump of nothing.
	 */
	let bodyFrom: number | undefined;

	let timer: ReturnType<typeof setTimeout> | undefined;
	/**
	 * Which request the answer on screen is allowed to come from.
	 *
	 * Debouncing alone does not order the answers: type, pause, type, pause, and the first
	 * response can still arrive second. Without this the reader sees the results for a prefix of
	 * what they typed, which looks like the search being wrong rather than late. A counter rather
	 * than an `AbortController` because the client exposes no signal to abort with -- and because
	 * the request has already left either way, so what matters is which answer may be believed.
	 */
	let issued = 0;

	function reset() {
		query = '';
		hits = [];
		active = 0;
		searching = false;
		failed = false;
		clearTimeout(timer);
		// Nothing already in flight may write to a dialog that has been closed and reopened.
		issued += 1;
		bodyMotion?.stop();
		bodyMotion = undefined;
		bodyMoving = false;
		bodyState = undefined;
		bodyFrom = undefined;
	}

	function onOpenChange(next: boolean) {
		open = next;
		if (!next) reset();
	}

	async function run(text: string) {
		const ticket = ++issued;
		searching = true;
		try {
			const found = await search(text, locale);
			if (ticket !== issued) return;
			hits = found;
			active = 0;
			failed = false;
		} catch {
			if (ticket !== issued) return;
			hits = [];
			failed = true;
		} finally {
			if (ticket === issued) searching = false;
		}
	}

	function onInput() {
		clearTimeout(timer);
		const text = query.trim();
		if (!worthSearching(text)) {
			// Deleting back to one letter must clear the answer to two, not leave it standing.
			issued += 1;
			hits = [];
			searching = false;
			failed = false;
			return;
		}
		timer = setTimeout(() => run(text), QUIET_MS);
	}

	function open_(hit: SearchHit) {
		onOpenChange(false);
		// An index record's address is absolute and may carry `?lang=`; `goto` keeps it a client
		// navigation rather than a reload.
		void goto(new URL(hit.url).pathname + new URL(hit.url).search + new URL(hit.url).hash);
	}

	function onKeydown(event: KeyboardEvent) {
		if (rows.length === 0) return;
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			active = (active + 1) % rows.length;
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			active = (active - 1 + rows.length) % rows.length;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			const hit = rows[active];
			if (hit) open_(hit);
		}
	}

	/**
	 * The global binding.
	 *
	 * Both modifiers, because the site is read on both kinds of machine and neither audience
	 * should have to learn the other's key. Captured on the window so it works wherever the
	 * reader is, and stood down inside a text field so it cannot steal a keystroke someone meant
	 * for what they were writing.
	 */
	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key.toLowerCase() === 'k' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			onOpenChange(!open);
		}
	}

	// Focus after bits-ui has mounted and settled the surface, not on the same tick.
	$effect(() => {
		if (open && input) input.focus();
	});

	const groups = $derived(groupHits(hits));
	/**
	 * The same results as one sequence, for the keyboard.
	 *
	 * Arrow keys move between sections, not between articles -- the grouping is what the reader
	 * sees, not a level they have to step through. Derived from `groups` rather than from `hits`
	 * so what the caps dropped is not silently still reachable.
	 */
	const rows = $derived(groups.flatMap((group) => group.sections));

	const empty = $derived(
		!searching && !failed && hits.length === 0 && worthSearching(query.trim()),
	);

	/**
	 * A stand-in for what the body is showing, so the height is re-measured when that changes and
	 * not once per keystroke. Typing inside a debounce leaves the results standing; measuring
	 * then would cost two forced reflows to discover that nothing moved.
	 */
	const bodyShowing = $derived(
		groups.length > 0
			? groups.map((group) => `${group.path}/${group.sections.length}`).join('|')
			: failed
				? 'failed'
				: empty
					? 'empty'
					: 'idle',
	);

	function settleBody() {
		// Handed back to `auto`, so the body keeps following its own content: a panel pinned to a
		// measured number would stop answering a window resize or a font landing late.
		if (bodyEl) bodyEl.style.height = 'auto';
		bodyMoving = false;
		bodyMotion = undefined;
	}

	/**
	 * Carry the body from the height it has to the height its new contents want.
	 *
	 * The target is measured by letting the layout settle it -- `auto` for one synchronous read,
	 * with the panel's own max-height doing the clamping through flex -- rather than by taking
	 * `scrollHeight` the way a disclosure does. A list of forty hits wants far more room than the
	 * panel will ever give it, and the number to animate to is the one the panel would have
	 * arrived at anyway. Reading it this way also means the header and footer are never written
	 * down here as a figure to subtract and keep in step.
	 */
	function resizeBody(from: number) {
		if (!bodyEl) return;
		bodyMotion?.stop();
		bodyMotion = undefined;

		bodyEl.style.height = 'auto';
		const target = bodyEl.getBoundingClientRect().height;
		// Back before the browser paints, so the measurement is never a frame the reader sees.
		bodyEl.style.height = remFromMeasuredPixels(from);

		bodyMoving = true;
		bodyMotion = animateHeight(bodyEl, target, (finished) => {
			// A stopped animation is not guaranteed to stay silent; settling the wrong one would
			// pin the body to the height an interrupted move was travelling to.
			if (finished !== undefined && bodyMotion !== finished) return;
			settleBody();
		});
	}

	$effect.pre(() => {
		const showing = bodyShowing;
		if (!bodyEl || showing === bodyState) return;
		bodyFrom = bodyEl.getBoundingClientRect().height;
	});

	$effect(() => {
		const showing = bodyShowing;
		if (!bodyEl || showing === bodyState) return;
		const first = bodyState === undefined;
		bodyState = showing;
		const from = bodyFrom;
		bodyFrom = undefined;
		if (first || from === undefined) return;
		resizeBody(from);
	});
</script>

<svelte:window onkeydown={onWindowKeydown} />

<Dialog.Root {open} {onOpenChange}>
	<Dialog.Portal>
		<Dialog.Overlay class="search-overlay fixed inset-0 z-60" />
		<Dialog.Content
			class="search-panel fixed top-[12vh] left-1/2 z-60 flex max-h-[70vh] w-[min(38rem,calc(100vw-2rem))] flex-col overflow-hidden rounded-xl border border-border bg-paper text-text shadow-lg"
		>
			<Dialog.Title class="sr-only">{m['search.title']({}, { locale })}</Dialog.Title>

			<div class="flex shrink-0 items-center gap-3 border-b border-border px-4">
				<SearchIcon class="size-4 shrink-0 text-text-soft" aria-hidden="true" />
				<input
					bind:this={input}
					bind:value={query}
					oninput={onInput}
					onkeydown={onKeydown}
					type="search"
					autocomplete="off"
					autocorrect="off"
					spellcheck="false"
					placeholder={m['search.placeholder']({}, { locale })}
					aria-label={m['search.title']({}, { locale })}
					class="min-w-0 flex-1 bg-transparent py-3.5 text-[0.9375rem] text-text-strong outline-none placeholder:text-text-soft"
				/>
				{#if searching}
					<span class="search-pulse size-1.5 shrink-0 rounded-full bg-text-soft"></span>
				{/if}
			</div>

			<!-- One element holds every state the body can be in, because the height is animated
			     between them and a height belongs to a box that stays put. -->
			<div bind:this={bodyEl} class="search-body min-h-0" data-moving={bodyMoving || undefined}>
				{#if groups.length > 0}
					<ul class="p-1.5">
						{#each groups as group (group.path)}
							<li class="mb-1 last:mb-0">
								<!-- The title once, for the whole group. Repeating it on every section spent a line
								     each time telling the reader something the first line already told them. -->
								<p class="truncate px-2.5 pt-2 pb-1 text-[0.9375rem] font-medium text-text-strong">
									{group.title}
								</p>
								<ul>
									{#each group.sections as hit (hit.objectID)}
										{@const index = rows.indexOf(hit)}
										<li>
											<button
												type="button"
												onclick={() => open_(hit)}
												onmousemove={() => (active = index)}
												class="focus-ring block w-full rounded-md px-2.5 py-1.5 text-left transition-colors duration-100"
												class:bg-paper-hover={index === active}
											>
												{#if hit.heading}
													<span class="block truncate text-[0.8125rem] text-text">
														<!-- Escaped in `markup`; the only tags here are the ones it inserted. -->
														{@html markup(hit._highlightResult?.heading?.value, hit.heading)}
													</span>
												{/if}
												<span
													class="block line-clamp-2 text-[0.8125rem] leading-snug text-text-soft"
												>
													{@html markup(hit._snippetResult?.text?.value, hit.text.slice(0, 160))}
												</span>
											</button>
										</li>
									{/each}
								</ul>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="px-4 py-6 text-center text-[0.8125rem] text-text-soft">
						{#if failed}
							{m['search.failed']({}, { locale })}
						{:else if empty}
							{m['search.empty']({}, { locale })}
						{:else}
							{m['search.idle']({}, { locale })}
						{/if}
					</p>
				{/if}
			</div>

			<div
				class="flex shrink-0 items-center justify-between gap-3 border-t border-border px-4 py-2 text-[0.6875rem] text-text-soft"
			>
				<span class="flex items-center gap-1.5">
					<kbd class="search-key">↑</kbd>
					<kbd class="search-key">↓</kbd>
					<span>{m['search.hint.move']({}, { locale })}</span>
				</span>
				<span class="flex items-center gap-1.5">
					<kbd class="search-key"><CornerDownLeft class="size-2.5" aria-hidden="true" /></kbd>
					<span>{m['search.hint.open']({}, { locale })}</span>
				</span>
				<span class="flex items-center gap-1.5">
					<kbd class="search-key">esc</kbd>
					<span>{m['search.hint.close']({}, { locale })}</span>
				</span>
			</div>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	/* A heavier blur than the modal's: this surface covers the page rather than sitting on it,
	   and the page behind should read as set aside rather than merely dimmed. */
	:global(.search-overlay) {
		background: color-mix(in oklch, var(--color-page) 55%, transparent);
		-webkit-backdrop-filter: blur(0.75rem);
		backdrop-filter: blur(0.75rem);
		transition: opacity 150ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	:global(.search-panel) {
		transform: translateX(-50%);
		transition:
			opacity 150ms cubic-bezier(0.22, 1, 0.36, 1),
			transform 150ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	:global(.search-overlay[data-starting-style]),
	:global(.search-overlay[data-ending-style]),
	:global(.search-panel[data-starting-style]),
	:global(.search-panel[data-ending-style]) {
		opacity: 0;
	}

	:global(.search-panel[data-starting-style]),
	:global(.search-panel[data-ending-style]) {
		transform: translateX(-50%) scale(0.98) translateY(-0.25rem);
	}

	:global(.search-key) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.125rem;
		height: 1.125rem;
		padding: 0 0.25rem;
		border: 1px solid var(--color-border);
		border-radius: 0.25rem;
		background: var(--color-page);
		font-size: 0.625rem;
		line-height: 1;
	}

	/* The match marks come from `markup`, so they are styled globally rather than scoped away. */
	:global(.search-panel mark) {
		background: none;
		color: var(--color-text-strong);
		font-weight: 500;
	}

	:global(.search-pulse) {
		animation: search-pulse 900ms ease-in-out infinite;
	}

	@keyframes search-pulse {
		0%,
		100% {
			opacity: 0.25;
		}
		50% {
			opacity: 1;
		}
	}

	/*
	 * The body scrolls at rest and is clipped while it moves: an animated height that let its
	 * scrollbar show would flash one in and out on every result that changes the panel's size.
	 */
	.search-body {
		overflow-y: auto;
	}

	.search-body[data-moving] {
		overflow: hidden;
		will-change: height;
	}

	/* The search input's own clear button is the browser's, not this design system's. */
	input[type='search']::-webkit-search-decoration,
	input[type='search']::-webkit-search-cancel-button {
		-webkit-appearance: none;
		appearance: none;
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.search-overlay),
		:global(.search-panel) {
			transition: none;
		}

		:global(.search-pulse) {
			animation: none;
		}
	}
</style>
