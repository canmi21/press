<script lang="ts">
	import Coffee from '@lucide/svelte/icons/coffee';
	import Heart from '@lucide/svelte/icons/heart';
	import Star from '@lucide/svelte/icons/star';
	import { animate } from 'motion';
	import { recall, remember } from '$lib/client/state';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import { createEngagementQuery, createLikeMutation } from '$lib/engagement/engagement.svelte';
	import { PUBLIC_LANGUAGE, type LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';
	import { intlLocale } from '$lib/format';

	/**
	 * Expanding answers to whether the pointer can hover, not to how wide the window is.
	 *
	 * A touch screen has no hover, but a tap synthesises `mouseenter` -- so the pill would grow
	 * under the finger that meant to press it, and then sit expanded with no pointer to leave and
	 * take it back. Reading it costs a press either way; growing first only moves the target.
	 *
	 * Width was the wrong question. An iPad is wider than any breakpoint this site draws and still
	 * has nothing that hovers, so a width guard was open on the one device class it was written
	 * for. This asks the capability directly, which is also why it needs no copy of a breakpoint.
	 *
	 * Queried live rather than once, so a tablet that is given a trackpad finds the other answer.
	 * See spec/styling.md.
	 */
	const HOVERS = '(hover: hover)';

	const WIDTH_SPRING = { type: 'spring' as const, stiffness: 420, damping: 28, mass: 0.85 };
	type AnimationControl = { stop: () => void };
	type CopyGeometry = {
		shortWidth: number;
		longWidth: number;
		prefix?: { mask: HTMLElement; width: number };
		suffix?: { mask: HTMLElement; width: number };
	};

	let {
		locale,
		sourcePreferenceHref,
		repositoryHref,
		onsponsor,
	}: {
		locale: LocaleCode;
		sourcePreferenceHref: string;
		repositoryHref: string;
		/** Becomes an `<a>` once there is somewhere to send people; see libs/urls. */
		onsponsor?: () => void;
	} = $props();

	/**
	 * Which favour this row asks for, in the one slot that asks for one.
	 *
	 * Asking the same reader for the same thing on every visit is asking nothing: once they have
	 * set the source preference there is nothing left to set, and the pill goes on offering it.
	 * So the slot moves on. `support.preferred` in the reader's state record -- see
	 * `client/state.ts` -- says this reader has been sent to Google at some point, and from their
	 * next visit the slot asks for a star instead.
	 *
	 * `sessionStorage["preferred"]` is what keeps it from moving under them. A reader who clicks
	 * and comes back to the tab -- or reloads, or navigates and returns -- would otherwise find a
	 * different control where they just pressed one, which reads as the page having changed its
	 * mind. Within the tab that did it, the pill stays where it was. That one is a bare
	 * `sessionStorage` key and deliberately not part of the record: it describes the tab rather
	 * than the reader, and the record is what a later build syncs between their devices.
	 *
	 * The server has neither store, so it renders Google -- right for every first-time reader,
	 * which is everyone it can see -- and a returning reader's pill changes after hydration. Both
	 * short forms are a six-letter brand name, so what moves is the label and not the row.
	 */
	const PREFERRED = 'support.preferred';
	let asksForStar = $state(false);

	$effect(() => {
		try {
			const thisTab = sessionStorage.getItem(PREFERRED) !== null;
			asksForStar = !thisTab && recall(localStorage, PREFERRED, false);
		} catch {
			// Private browsing, or storage the reader has turned off. The default already stands.
		}
	});

	const favourLabel = $derived(
		asksForStar ? m['support.github']({}, { locale }) : m['support.google']({}, { locale }),
	);
	const favourShort = $derived(
		asksForStar
			? m['support.github-short']({}, { locale })
			: m['support.google-short']({}, { locale }),
	);

	/** Both stores, because each answers a different question about the same click. */
	function recordPreferred() {
		remember(localStorage, PREFERRED, true);
		try {
			// Not part of the record: this one is about the tab, not about the reader, and the
			// record is what a later build will sync between their devices.
			sessionStorage.setItem(PREFERRED, '1');
		} catch {
			// Nothing to record into. The slot simply goes on asking, which is the old behaviour.
		}
	}

	const engagement = createEngagementQuery();
	const like = createLikeMutation();
	const liked = $derived(engagement.data?.liked ?? false);
	const actionAnimations = new WeakMap<HTMLElement, AnimationControl>();
	const actionChromeWidths = new WeakMap<HTMLElement, number>();

	const count = $derived(engagement.data?.like_count ?? 0);
	const numberLocale = $derived(intlLocale(locale));
	const numberFormat = $derived(new Intl.NumberFormat(numberLocale));
	const formattedCount = $derived(numberFormat.format(count));

	function toggle() {
		if (!like.isPending) like.mutate(!liked);
	}

	function splitCopy(short: string, long: string) {
		const start = long.indexOf(short);
		if (start === -1) return undefined;
		return {
			prefix: long.slice(0, start),
			shared: short,
			suffix: long.slice(start + short.length),
		};
	}

	function measureCopy(action: HTMLElement): CopyGeometry | undefined {
		const shared = action.querySelector<HTMLElement>('.shared');
		if (shared) {
			const prefixMask = action.querySelector<HTMLElement>('.prefix-mask');
			const prefixText = prefixMask?.firstElementChild as HTMLElement | undefined;
			const suffixMask = action.querySelector<HTMLElement>('.suffix-mask');
			const suffixText = suffixMask?.firstElementChild as HTMLElement | undefined;
			const shortWidth = shared.scrollWidth;
			const prefixWidth = prefixText?.scrollWidth ?? 0;
			const suffixWidth = suffixText?.scrollWidth ?? 0;
			return {
				shortWidth,
				longWidth: prefixWidth + shortWidth + suffixWidth,
				prefix: prefixMask ? { mask: prefixMask, width: prefixWidth } : undefined,
				suffix: suffixMask ? { mask: suffixMask, width: suffixWidth } : undefined,
			};
		}

		const short = action.querySelector<HTMLElement>('.short');
		const long = action.querySelector<HTMLElement>('.long');
		if (!short || !long) return undefined;
		return { shortWidth: short.scrollWidth, longWidth: long.scrollWidth };
	}

	function revealCopy(width: number, chromeWidth: number, geometry: CopyGeometry) {
		const distance = geometry.longWidth - geometry.shortWidth;
		if (distance <= 0) return;
		const progress = Math.max(0, (width - chromeWidth - geometry.shortWidth) / distance);
		if (geometry.prefix) {
			geometry.prefix.mask.style.width = remFromMeasuredPixels(geometry.prefix.width * progress);
		}
		if (geometry.suffix) {
			geometry.suffix.mask.style.width = remFromMeasuredPixels(geometry.suffix.width * progress);
		}
	}

	function setExpanded(action: HTMLElement, expanded: boolean) {
		const geometry = measureCopy(action);
		if (!geometry) return;

		const currentWidth = action.getBoundingClientRect().width;
		let chromeWidth = actionChromeWidths.get(action);
		if (chromeWidth === undefined) {
			chromeWidth = currentWidth - geometry.shortWidth;
			actionChromeWidths.set(action, chromeWidth);
		}

		const targetWidth = chromeWidth + (expanded ? geometry.longWidth : geometry.shortWidth);
		actionAnimations.get(action)?.stop();
		actionAnimations.delete(action);
		action.style.width = remFromMeasuredPixels(currentWidth);
		action.dataset.expanded = String(expanded);

		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			action.style.width = expanded ? remFromMeasuredPixels(targetWidth) : '';
			revealCopy(targetWidth, chromeWidth, geometry);
			return;
		}

		let control: AnimationControl;
		control = animate(currentWidth, targetWidth, {
			...WIDTH_SPRING,
			onUpdate: (width) => {
				action.style.width = remFromMeasuredPixels(width);
				revealCopy(width, chromeWidth, geometry);
			},
			onComplete: () => {
				if (actionAnimations.get(action) !== control) return;
				actionAnimations.delete(action);
				if (!expanded) action.style.width = '';
			},
		});
		actionAnimations.set(action, control);
	}

	function expand(event: MouseEvent) {
		// Only the pointer path is guarded. Keyboard focus arrives through `expandFromFocus`, and
		// `:focus-visible` is never what a tap produces, so it carries none of the risk above --
		// a tablet with a keyboard still gets the full label on Tab. Collapsing is never guarded
		// either: whatever opened a pill has to be able to put it back.
		if (!window.matchMedia(HOVERS).matches) return;
		setExpanded(event.currentTarget as HTMLElement, true);
	}

	function expandFromFocus(event: FocusEvent) {
		const action = event.currentTarget as HTMLElement;
		if (action.matches(':focus-visible')) setExpanded(action, true);
	}

	function collapse(event: MouseEvent | FocusEvent) {
		setExpanded(event.currentTarget as HTMLElement, false);
	}
</script>

{#snippet copy(short: string, long: string)}
	{@const parts = splitCopy(short, long)}
	{#if parts}
		<span class="copy segmented" aria-hidden="true">
			<span class="prefix-mask reveal-mask"><span>{parts.prefix}</span></span>
			<span class="shared">{parts.shared}</span>
			<span class="suffix-mask reveal-mask"><span>{parts.suffix}</span></span>
		</span>
	{:else}
		<span class="copy fallback" aria-hidden="true">
			<span class="short">{short}</span>
			<span class="long">{long}</span>
		</span>
	{/if}
{/snippet}

<section aria-labelledby="support-heading" class="mt-16">
	<h2 id="support-heading" class="font-medium text-text-strong">
		{m['support.heading']({}, { locale })}
	</h2>

	<div class="mt-3 flex flex-wrap items-center gap-1.5">
		<!-- `tabular-nums` because the count is the one thing here that changes while the reader is
		     looking at it, and Inter's proportional digits are not the same width: `1` is 6.6px
		     against `4`'s 10.5px. A like the reader just gave would resize its own pill and shift
		     the two beside it. Tabular figures give every digit the widest one's advance, so the
		     width answers only to how many digits there are -- a change that has a reason the
		     reader can see. It is a feature of this same font, not a monospace face: only the
		     digits take the fixed advance and `likes` beside them is untouched. -->
		<button
			type="button"
			aria-pressed={liked}
			aria-label={m['support.like']({ count: formattedCount }, { locale })}
			data-liked={liked}
			data-expanded="false"
			onclick={toggle}
			disabled={like.isPending}
			aria-busy={like.isPending}
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action like focus-ring tabular-nums"
		>
			<Heart class="icon" fill={liked ? 'currentColor' : 'none'} aria-hidden="true" />
			{@render copy(formattedCount, m['support.like']({ count: formattedCount }, { locale }))}
		</button>

		<!-- One slot, two favours. The star is right for either: it is the mark Google's preference
		     list and GitHub's repositories both use, so the pill keeps its shape and only its
		     words change. -->
		<a
			href={asksForStar ? repositoryHref : sourcePreferenceHref}
			target="_blank"
			rel="noopener"
			aria-label={`${favourLabel} (${m['support.new-tab']({}, { locale })})`}
			data-expanded="false"
			onclick={() => {
				if (!asksForStar) recordPreferred();
			}}
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action focus-ring"
		>
			<Star class="icon" aria-hidden="true" />
			{@render copy(favourShort, favourLabel)}
		</a>

		<button
			type="button"
			onclick={() => onsponsor?.()}
			aria-label={m['support.sponsor']({}, { locale })}
			data-expanded="false"
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action focus-ring"
		>
			<Coffee class="icon" aria-hidden="true" />
			{@render copy(
				m['support.sponsor-short']({}, { locale }),
				m['support.sponsor']({}, { locale }),
			)}
		</button>
	</div>
</section>

<style>
	.action {
		/* Two of the three are buttons and the third is a link, so without this the row draws two
		   arrows and one hand for three controls that do the same kind of thing. */
		cursor: pointer;
		display: inline-flex;
		height: 2.25rem;
		flex-shrink: 0;
		align-items: center;
		overflow: hidden;
		border: 0.0625rem solid var(--color-border);
		border-radius: 624.9375rem;
		background: var(--color-paper);
		padding-inline: 0.75rem;
		font-weight: 500;
		color: var(--color-text-strong);
		transition:
			background-color 200ms,
			border-color 200ms,
			color 200ms;
	}

	.action :global(.icon) {
		width: 1rem;
		height: 1rem;
		margin-inline-end: 0.5rem;
		flex-shrink: 0;
	}

	.copy {
		flex: none;
	}

	.segmented {
		display: inline-flex;
		align-items: center;
	}

	.reveal-mask {
		width: 0;
		flex: none;
		overflow: hidden;
	}

	.reveal-mask > span,
	.shared {
		display: block;
		width: max-content;
		white-space: pre;
	}

	.fallback {
		display: inline-grid;
	}

	.fallback > span {
		grid-area: 1 / 1;
		justify-self: start;
		white-space: nowrap;
	}

	.short {
		opacity: 1;
		transition: opacity 120ms ease 80ms;
	}

	.long {
		max-width: 0;
		overflow: hidden;
		opacity: 0;
		transition: opacity 140ms ease;
	}

	.action:is(:hover, :focus-visible) {
		border-color: var(--color-border-strong);
		background: var(--color-paper-hover);
	}

	:global(.action[data-expanded='true']) .short {
		opacity: 0;
		transition-delay: 0ms;
	}

	:global(.action[data-expanded='true']) .long {
		max-width: 14rem;
		opacity: 1;
		transition-delay: 80ms;
	}

	.like[data-liked='true']:is(:hover, :focus-visible) {
		border-color: transparent;
		background: var(--color-ink);
		color: var(--color-page);
	}

	@media (prefers-reduced-motion: reduce) {
		.action,
		.short,
		.long {
			transition: none;
		}
	}
</style>
