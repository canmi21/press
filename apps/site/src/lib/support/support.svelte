<script lang="ts">
	import Coffee from '@lucide/svelte/icons/coffee';
	import GitPullRequestArrow from '@lucide/svelte/icons/git-pull-request-arrow';
	import Heart from '@lucide/svelte/icons/heart';
	import Star from '@lucide/svelte/icons/star';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import { animate } from 'motion';
	import { PUBLIC_LANGUAGE, type LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	const WIDTH_SPRING = { type: 'spring' as const, stiffness: 420, damping: 28, mass: 0.85 };
	type AnimationControl = { stop: () => void };

	let {
		locale,
		likes,
		commit,
		commitHref,
		followHref,
		sourcePreferenceHref,
		onlike,
		onsponsor,
	}: {
		locale: LocaleCode;
		likes: number;
		commit: string;
		commitHref: string;
		followHref: string;
		sourcePreferenceHref: string;
		/** The seam the real endpoint plugs into; the count on screen is the local guess. */
		onlike?: (liked: boolean) => Promise<void>;
		/** Becomes an `<a>` once there is somewhere to send people; see libs/urls. */
		onsponsor?: () => void;
	} = $props();

	let liked = $state(false);
	const actionAnimations = new WeakMap<HTMLElement, AnimationControl>();
	const actionChromeWidths = new WeakMap<HTMLElement, number>();

	// Counted optimistically off the server's figure, so the button answers the click rather
	// than a round trip.
	const count = $derived(likes + (liked ? 1 : 0));
	const numberLocale = $derived(locale === 'mw' ? 'en-US' : PUBLIC_LANGUAGE[locale]);
	const numberFormat = $derived(new Intl.NumberFormat(numberLocale));
	const formattedCount = $derived(numberFormat.format(count));

	function toggle() {
		liked = !liked;
		void onlike?.(liked);
	}

	function setExpanded(action: HTMLElement, expanded: boolean) {
		const short = action.querySelector<HTMLElement>('.short');
		const long = action.querySelector<HTMLElement>('.long');
		if (!short || !long) return;

		const currentWidth = action.getBoundingClientRect().width;
		let chromeWidth = actionChromeWidths.get(action);
		if (chromeWidth === undefined) {
			chromeWidth = currentWidth - short.scrollWidth;
			actionChromeWidths.set(action, chromeWidth);
		}

		const targetWidth = chromeWidth + (expanded ? long.scrollWidth : short.scrollWidth);
		actionAnimations.get(action)?.stop();
		actionAnimations.delete(action);
		action.style.width = `${currentWidth}px`;
		action.dataset.expanded = String(expanded);

		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			action.style.width = expanded ? `${targetWidth}px` : '';
			return;
		}

		let control: AnimationControl;
		control = animate(currentWidth, targetWidth, {
			...WIDTH_SPRING,
			onUpdate: (width) => {
				action.style.width = `${width}px`;
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
	<span class="copy" aria-hidden="true">
		<span class="short">{short}</span>
		<span class="long">{long}</span>
	</span>
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
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action like"
		>
			<Heart class="icon" fill={liked ? 'currentColor' : 'none'} aria-hidden="true" />
			{@render copy(formattedCount, m['support.like']({ count: formattedCount }, { locale }))}
		</button>

		<a
			href={commitHref}
			target="_blank"
			rel="noopener noreferrer"
			aria-label={`${m['support.revision']({ commit }, { locale })} (opens in new tab)`}
			data-expanded="false"
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action"
		>
			<GitPullRequestArrow class="icon" aria-hidden="true" />
			{@render copy(commit, m['support.revision']({ commit }, { locale }))}
		</a>

		<a
			href={followHref}
			target="_blank"
			rel="noopener noreferrer"
			aria-label={`${m['support.follow']({}, { locale })} (opens in new tab)`}
			data-expanded="false"
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action"
		>
			<UserPlus class="icon" aria-hidden="true" />
			{@render copy(
				m['support.follow-short']({}, { locale }),
				m['support.follow']({}, { locale }),
			)}
		</a>

		<a
			href={sourcePreferenceHref}
			target="_blank"
			rel="noopener noreferrer"
			aria-label={`${m['support.google']({}, { locale })} (opens in new tab)`}
			data-expanded="false"
			onmouseenter={expand}
			onmouseleave={collapse}
			onfocus={expandFromFocus}
			onblur={collapse}
			class="action"
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
			class="action"
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
		border: 1px solid var(--color-border);
		border-radius: 9999px;
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
		display: inline-grid;
	}

	.copy > span {
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
