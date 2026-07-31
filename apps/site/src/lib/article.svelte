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
	import { dev } from '$app/environment';
	import { page } from '$app/state';
	import { pickUrls } from '@canmi/urls';
	import BookOpenText from '@lucide/svelte/icons/book-open-text';
	import Type from '@lucide/svelte/icons/type';
	import type { Snippet } from 'svelte';
	import { formatCompact } from '$lib/format';
	import Toc from '$lib/toc.svelte';

	let { meta, chars, children }: { meta: ArticleMeta; chars: number; children: Snippet } = $props();

	const urls = pickUrls(dev);

	/**
	 * The card for this article, at a URL nothing had to be told.
	 *
	 * `cms og` writes one card per article under the same path the article has, so the address
	 * follows from the route and no reference is stored anywhere. The cost is that the name is
	 * mutable -- an edited title reuses this URL -- which is why the CDN serves these for a
	 * week rather than a year. See spec/architecture.md.
	 */
	const card = $derived(`${urls.cdn}/opengraph${page.url.pathname.replace(/\/$/, '')}.png`);
	const canonical = $derived(new URL(page.url.pathname, urls.site).href);

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

	<meta property="og:type" content="article" />
	<meta property="og:title" content={meta.title} />
	<meta property="og:description" content={meta.description} />
	<meta property="og:url" content={canonical} />
	<meta property="og:image" content={card} />
	<!-- Stated because a crawler that reserves the box before fetching draws it right. -->
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:alt" content={meta.title} />
	<meta property="article:published_time" content={meta.created} />

	<!-- `summary_large_image` is what makes X render the card at full width rather than as a
	     thumbnail beside the text, which is the only shape this layout is drawn for. -->
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={meta.title} />
	<meta name="twitter:description" content={meta.description} />
	<meta name="twitter:image" content={card} />
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
