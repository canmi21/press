<script lang="ts">
	import Coffee from '@lucide/svelte/icons/coffee';
	import Heart from '@lucide/svelte/icons/heart';
	import Star from '@lucide/svelte/icons/star';
	import { animate } from 'motion';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import { createEngagementQuery, createLikeMutation } from '$lib/engagement/engagement.svelte';
	import { PUBLIC_LANGUAGE, type LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

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
		onsponsor,
	}: {
		locale: LocaleCode;
		sourcePreferenceHref: string;
		/** Becomes an `<a>` once there is somewhere to send people; see libs/urls. */
		onsponsor?: () => void;
	} = $props();

	const engagement = createEngagementQuery();
	const like = createLikeMutation();
	const liked = $derived(engagement.data?.liked ?? false);
	const actionAnimations = new WeakMap<HTMLElement, AnimationControl>();
	const actionChromeWidths = new WeakMap<HTMLElement, number>();

	const count = $derived(engagement.data?.like_count ?? 0);
	const numberLocale = $derived(locale === 'mw' ? 'en-US' : PUBLIC_LANGUAGE[locale]);
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
			geometry.prefix.mask.style.width = remFromMeasuredPixels(
				geometry.prefix.width * progress,
			);
		}
		if (geometry.suffix) {
			geometry.suffix.mask.style.width = remFromMeasuredPixels(
				geometry.suffix.width * progress,
			);
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
			class="action like focus-ring"
		>
			<Heart class="icon" fill={liked ? 'currentColor' : 'none'} aria-hidden="true" />
			{@render copy(formattedCount, m['support.like']({ count: formattedCount }, { locale }))}
		</button>

		<a
			href={sourcePreferenceHref}
			target="_blank"
			rel="noopener noreferrer"
			aria-label={`${m['support.google']({}, { locale })} (${m['support.new-tab']({}, { locale })})`}
			data-expanded="false"
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action focus-ring"
		>
			<Star class="icon" aria-hidden="true" />
			{@render copy(
				m['support.google-short']({}, { locale }),
				m['support.google']({}, { locale }),
			)}
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
