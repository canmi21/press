<script lang="ts">
	import Coffee from '@lucide/svelte/icons/coffee';
	import Heart from '@lucide/svelte/icons/heart';
	import Star from '@lucide/svelte/icons/star';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import { PUBLIC_LANGUAGE, type LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let {
		locale,
		likes,
		visitors,
		days,
		updated,
		words,
		commit,
		commitHref,
		followHref,
		sourcePreferenceHref,
		onlike,
		onsponsor,
	}: {
		locale: LocaleCode;
		likes: number;
		visitors: number;
		/** Days the site has been up. */
		days: number;
		/** Days since the last article changed. */
		updated: number;
		/** Words across every article. */
		words: number;
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

	// Counted optimistically off the server's figure, so the button answers the click rather
	// than a round trip.
	const count = $derived(likes + (liked ? 1 : 0));
	const numberLocale = $derived(locale === 'mw' ? 'en-US' : PUBLIC_LANGUAGE[locale]);
	const numberFormat = $derived(new Intl.NumberFormat(numberLocale));

	function toggle() {
		liked = !liked;
		void onlike?.(liked);
	}
</script>

<section aria-labelledby="colophon-heading" class="mt-16">
	<div class="grid grid-cols-[1fr_auto] items-center gap-x-3 gap-y-2 sm:flex sm:flex-wrap">
		<h2 id="colophon-heading" class="font-medium text-text-strong">
			{m['colophon.heading']({}, { locale })}
		</h2>

		<button
			type="button"
			aria-pressed={liked}
			onclick={toggle}
			class="inline-flex h-9 shrink-0 items-center gap-1.5 rounded-full px-2.5 font-medium transition-colors duration-200 sm:ml-auto {liked
				? 'bg-ink text-page'
				: 'text-text-soft hover:bg-paper-hover hover:text-text-strong'}"
		>
			<Heart class="h-4 w-4 shrink-0" fill={liked ? 'currentColor' : 'none'} aria-hidden="true" />
			<span class="tabular-nums">{numberFormat.format(count)}</span>
			<span class="sr-only">{m['colophon.likes']({}, { locale })}</span>
		</button>

		<div class="col-span-2 flex flex-wrap items-center justify-end gap-1.5">
			<a
				href={followHref}
				target="_blank"
				rel="noopener noreferrer"
				class="action"
			>
				<UserPlus class="h-4 w-4 shrink-0" aria-hidden="true" />
				<span>{m['colophon.follow']({}, { locale })}</span>
				<span class="sr-only">(opens in new tab)</span>
			</a>

			<a
				href={sourcePreferenceHref}
				target="_blank"
				rel="noopener noreferrer"
				class="action"
				title={m['colophon.prefer']({}, { locale })}
			>
				<Star class="h-4 w-4 shrink-0" aria-hidden="true" />
				<span class="sr-only sm:not-sr-only">{m['colophon.prefer']({}, { locale })}</span>
				<span class="sr-only">(opens in new tab)</span>
			</a>

			<button
				type="button"
				onclick={() => onsponsor?.()}
				class="action"
			>
				<Coffee class="h-4 w-4 shrink-0" aria-hidden="true" />
				<span>{m['colophon.sponsor']({}, { locale })}</span>
			</button>
		</div>
	</div>

	<div class="mt-5 overflow-hidden rounded-lg border border-border bg-paper">
		<dl class="stats grid grid-cols-2 md:grid-cols-4">
			<div class="stat">
				<dt>{m['colophon.visitors']({}, { locale })}</dt>
				<dd>{numberFormat.format(visitors)}</dd>
			</div>

			<div class="stat">
				<dt>{m['colophon.online']({}, { locale })}</dt>
				<dd>{m['colophon.days']({ days: numberFormat.format(days) }, { locale })}</dd>
			</div>

			<div class="stat">
				<dt>{m['colophon.words']({}, { locale })}</dt>
				<dd>{numberFormat.format(words)}</dd>
			</div>

			<div class="stat">
				<dt>{m['colophon.last-update']({}, { locale })}</dt>
				<dd>{m['colophon.updated']({ days: numberFormat.format(updated) }, { locale })}</dd>
			</div>
		</dl>

		<div
			class="flex flex-wrap items-baseline gap-x-5 gap-y-1.5 border-t border-border px-4 py-3 text-sm"
		>
			<span class="inline-flex items-baseline gap-1.5">
				<span class="text-text-soft">{m['colophon.revision']({}, { locale })}</span>
				<a
					href={commitHref}
					target="_blank"
					rel="noopener noreferrer"
					class="rounded-sm font-mono text-[0.8125rem] text-text-strong underline decoration-border-strong underline-offset-4 transition-colors duration-200 hover:decoration-text-strong"
				>
					{commit}<span class="sr-only"> (opens in new tab)</span>
				</a>
			</span>
			<span class="inline-flex items-baseline gap-1.5">
				<span class="text-text-soft">{m['colophon.articles']({}, { locale })}</span>
				<span class="font-mono text-[0.8125rem] text-text-strong">CC BY-NC 4.0</span>
			</span>
			<span class="inline-flex items-baseline gap-1.5">
				<span class="text-text-soft">{m['colophon.code']({}, { locale })}</span>
				<span class="font-mono text-[0.8125rem] text-text-strong">MIT</span>
			</span>
		</div>
	</div>
</section>

<style>
	.action {
		display: inline-flex;
		height: 2.25rem;
		flex-shrink: 0;
		align-items: center;
		gap: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 9999px;
		background: var(--color-paper);
		padding-inline: 0.875rem;
		font-weight: 500;
		color: var(--color-text-strong);
		transition:
			background-color 200ms,
			border-color 200ms;
	}

	.action:hover {
		border-color: var(--color-border-strong);
		background: var(--color-paper-hover);
	}

	.stat {
		display: flex;
		flex-direction: column;
		padding: 1rem;
	}

	.stat:nth-child(even) {
		border-inline-start: 1px solid var(--color-border);
	}

	.stat:nth-child(n + 3) {
		border-top: 1px solid var(--color-border);
	}

	.stat dd {
		order: -1;
		font-family: var(--font-mono);
		font-size: 1.125rem;
		font-variant-numeric: tabular-nums;
		font-weight: 500;
		line-height: 1.5rem;
		color: var(--color-text-strong);
	}

	.stat dt {
		margin-top: 0.125rem;
		font-size: 0.875rem;
		color: var(--color-text-soft);
	}

	@media (width >= 48rem) {
		.stat:not(:first-child) {
			border-inline-start: 1px solid var(--color-border);
		}

		.stat:nth-child(n + 3) {
			border-top: 0;
		}
	}
</style>
