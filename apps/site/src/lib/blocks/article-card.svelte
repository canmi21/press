<script lang="ts">
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Type from '@lucide/svelte/icons/type';
	import { formatCompact } from '$lib/article/format';
	import Thumbnail from '$lib/article/thumbnail.svelte';
	import { shortDate } from '$lib/format';

	let {
		path,
		title,
		subtitle,
		description,
		created,
		chars,
	}: {
		path: string;
		title: string;
		subtitle: string;
		description: string;
		created: string;
		/** Absent when the route could not measure the target. See its comment in types.ts. */
		chars?: number;
	} = $props();

	const date = $derived(shortDate(created));
</script>

<!-- Built on the tweet card's shell -- same width, same three bands, same corner reveal -- because
     both are one linked thing quoted into prose and a second box shape would say they differ.
     What changes is the arrow: this link stays on the site, so it points along rather than out. -->
<a href="/{path}" class="article-card focus-ring">
	<header class="header">
		<Thumbnail scale={1.5} />
		<span class="heading">
			<span class="name">{title}</span>
			<span class="subtitle">{subtitle}</span>
		</span>
	</header>

	<p class="description">{description}</p>

	<footer class="stats">
		<time datetime={created}>{date}</time>
		{#if chars !== undefined}
			<span class="stat" aria-label="{chars.toLocaleString('en-US')} characters">
				<Type class="size-3.5" aria-hidden="true" />
				<span class="tabular-nums">{formatCompact(chars)}</span>
			</span>
		{/if}
	</footer>

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
		margin-block: 1.8em;
		flex-direction: column;
		gap: 0.7rem;
		border: 0.0625rem solid var(--color-border);
		border-radius: 0.75rem;
		background: var(--color-paper);
		padding: 0.75rem;
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

	.header {
		display: flex;
		min-width: 0;
		align-items: center;
		gap: 0.75rem;
	}

	.heading {
		display: flex;
		min-width: 0;
		flex-direction: column;
		gap: 0.15rem;
	}

	.name {
		overflow: hidden;
		color: var(--color-text-strong);
		font-size: 0.9375rem;
		font-weight: 560;
		line-height: 1.35;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.subtitle {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		color: var(--color-text-soft);
		font-size: 0.78125rem;
		line-height: 1.45;
	}

	.description {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		margin: 0;
		color: var(--color-text);
		font-size: 0.875rem;
		line-height: 1.55;
	}

	.stats,
	.stat {
		display: flex;
		align-items: center;
	}

	.stats {
		gap: 0.9rem;
		color: var(--color-text-soft);
		font-size: 0.71875rem;
	}

	.stat {
		gap: 0.25rem;
	}

	.corner {
		position: absolute;
		right: 0.75rem;
		bottom: 0.75rem;
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
