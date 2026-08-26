<script lang="ts">
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Thumbnail from '$lib/article/thumbnail.svelte';
	import { shortDate } from '$lib/format';

	let {
		path,
		title,
		subtitle,
		created,
	}: {
		path: string;
		title: string;
		subtitle: string;
		created: string;
	} = $props();

	const date = $derived(shortDate(created));
</script>

<!-- The same box the other block cards use, with the homepage's article thumbnail as its icon.
     The arrow points along rather than out of the page: this link stays on the site, and the
     corner arrow `::github` and `::linkcard` wear is what says a destination is elsewhere. -->
<a href="/{path}" class="article-card focus-ring">
	<Thumbnail />

	<div class="copy">
		<span class="name">{title}</span>
		<p class="subtitle">{subtitle}</p>
		<time datetime={created} class="date">{date}</time>
	</div>

	<span class="corner" aria-hidden="true">
		<ArrowRight class="size-4" strokeWidth={2} />
	</span>
</a>

<style>
	.article-card {
		position: relative;
		display: flex;
		width: 100%;
		max-width: 28rem;
		align-items: center;
		gap: 0.75rem;
		margin-block: 1.8em;
		margin-inline: auto;
		border: 0.0625rem solid var(--color-border);
		border-radius: 0.75rem;
		background: var(--color-paper);
		/* Room on the right for the corner arrow, so a long title never runs under it. */
		padding: 0.6rem 2rem 0.6rem 0.75rem;
		color: inherit;
		text-decoration: none;
		transition:
			background-color 150ms ease-out,
			border-color 150ms ease-out;
	}

	.article-card:hover,
	.article-card:focus-visible {
		border-color: var(--color-border-strong);
		background: var(--color-paper-hover);
	}

	.copy {
		display: flex;
		min-width: 0;
		flex: 1;
		flex-direction: column;
		gap: 0.15rem;
	}

	.name {
		overflow: hidden;
		color: var(--color-text-strong);
		font-size: 0.875rem;
		font-weight: 560;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.subtitle {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		margin: 0;
		color: var(--color-text-soft);
		font-size: 0.78125rem;
		line-height: 1.45;
	}

	.date {
		color: var(--color-text-soft);
		font-size: 0.71875rem;
		font-variant-numeric: tabular-nums;
	}

	.corner {
		position: absolute;
		top: 50%;
		right: 0.75rem;
		translate: 0 -50%;
		color: var(--color-text-soft);
		opacity: 0;
		transition: opacity 200ms ease-out;
	}

	.article-card:hover .corner,
	.article-card:focus-visible .corner {
		opacity: 1;
	}

	@media (prefers-reduced-motion: reduce) {
		.article-card,
		.corner {
			transition: none;
		}
	}
</style>
