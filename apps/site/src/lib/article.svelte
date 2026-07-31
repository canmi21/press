<script module lang="ts">
	export type ArticleMeta = {
		title: string;
		subtitle: string;
		description: string;
		lang: 'zh' | 'en' | 'ja';
		created: string;
		lastmod: string;
		// Hard-coded view count carried over from the old site; swap for an API later.
		views?: number;
	};
</script>

<script lang="ts">
	import BookOpenText from '@lucide/svelte/icons/book-open-text';
	import Type from '@lucide/svelte/icons/type';
	import type { Snippet } from 'svelte';
	import { formatCompact } from '$lib/format';
	import Toc from '$lib/toc.svelte';

	let { meta, chars, children }: { meta: ArticleMeta; chars: number; children: Snippet } = $props();

	// Pin UTC so the shown day matches the authored frontmatter date everywhere it
	// renders, mirroring the article list (see article-card.svelte).
	const date = $derived(
		new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			timeZone: 'UTC'
		}).format(new Date(meta.created))
	);
</script>

<svelte:head>
	<title>{meta.title}: {meta.subtitle}</title>
	<meta name="description" content={meta.description} />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<Toc />
	<article class="mx-auto max-w-180 px-6 py-24">
		<header>
			<h1 class="text-text-strong">{meta.title}</h1>
			<div class="mt-2 flex items-center gap-2 text-sm text-text-soft">
				<time datetime={meta.created}>{date}</time>
				<span
					class="inline-flex items-center gap-1"
					title="{chars} characters"
					aria-label="{chars.toLocaleString('en-US')} characters"
				>
					<Type class="size-3.5" aria-hidden="true" />
					{formatCompact(chars)}
				</span>
				{#if meta.views != null}
					<span
						class="inline-flex items-center gap-1"
						title="{meta.views} views"
						aria-label="{meta.views.toLocaleString('en-US')} views"
					>
						<BookOpenText class="size-3.5" aria-hidden="true" />
						{formatCompact(meta.views)}
					</span>
				{/if}
			</div>
		</header>

		<div class="article-body mt-8 space-y-4 leading-relaxed">
			{@render children()}
		</div>
	</article>
</main>

<style>
	.article-body {
		font-size: 0.9375rem;
	}

	.article-body :global(strong) {
		font-weight: 500;
		color: var(--color-text-strong);
	}

	.article-body :global(s) {
		color: var(--color-text-soft);
	}

	.article-body :global(code:not(pre code)) {
		box-shadow: inset 0 0 0 0.0625rem var(--color-border-strong);
		border-radius: 0.375rem;
		background: var(--color-paper);
		padding: 0.125rem 0.375rem;
		font-size: 0.875rem;
	}
</style>
