<script lang="ts">
	import { dev } from '$app/environment';
	import { page } from '$app/state';
	import { pickUrls } from '@canmi/urls';
	import { site } from '$lib/site';
	import BookOpenText from '@lucide/svelte/icons/book-open-text';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Type from '@lucide/svelte/icons/type';
	import * as m from '$lib/paraglide/messages';
	import type { Snippet } from 'svelte';
	import type { Alternate, ArticleMeta } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import { formatCompact } from './format';
	import Toc from './toc.svelte';
	import TranslationNotice from './translation-notice.svelte';

	type ArticleLocale = {
		code: LocaleCode;
		languageTag: string;
		canonical: string;
		alternates: Alternate[];
	};

	let {
		meta,
		chars,
		locale,
		children,
	}: { meta: ArticleMeta; chars: number; locale: ArticleLocale; children: Snippet } = $props();

	const urls = pickUrls(dev);

	let summaryOpen = $state(false);
	// One call only -- `$props.id()` may not be used twice in a component -- so the pair is
	// derived from a single stable base.
	const summaryId = $props.id();
	const summaryTrigger = `${summaryId}-trigger`;
	const summaryPanel = `${summaryId}-panel`;

	/**
	 * Standing in for a field that does not exist yet.
	 *
	 * `description` is the SEO meta description: already translated per view, and the only
	 * summary-shaped text an article carries today. A reader-facing summary is a different job
	 * and wants its own frontmatter key, written by the same pipeline that writes alt text and
	 * tags. Until then this shows the nearest honest thing rather than an empty panel.
	 */
	const summary = $derived(meta.description);

	/**
	 * The card for this article, at a URL nothing had to be told.
	 *
	 * `cms og` writes one card per article under the same path the article has, so the address
	 * follows from the route and no reference is stored anywhere. The cost is that the name is
	 * mutable -- an edited title reuses this URL -- which is why the CDN serves these for a
	 * week rather than a year. See spec/architecture.md.
	 */
	const card = $derived(`${urls.cdn}/opengraph${page.url.pathname.replace(/\/$/, '')}.png`);

	$effect(() => {
		document.documentElement.lang = locale.languageTag;
	});

	/**
	 * A JSON-LD block, safe to drop into markup.
	 *
	 * Every `<` in the payload becomes `\u003c`, and the closing tag is assembled rather than
	 * written, so no `</script` sequence exists anywhere here. A tokenizer scanning for one does
	 * not care that it sits inside a string, and neither case stays hypothetical once this data
	 * includes text written by something other than us.
	 */
	function ldJson(data: unknown): string {
		const json = JSON.stringify(data).replaceAll('<', String.raw`\u003c`);
		return `<script type="application/ld+json">${json}</${'script'}>`;
	}

	/** What this page is, for a reader that parses rather than renders. */
	const article = $derived({
		'@context': 'https://schema.org',
		'@type': 'Article',
		headline: meta.title,
		description: meta.description,
		image: card,
		datePublished: meta.created,
		dateModified: meta.lastmod,
		inLanguage: locale.languageTag,
		mainEntityOfPage: locale.canonical,
		author: { '@type': 'Person', name: site.author.name },
	});

	// Pin UTC so the shown day matches the authored frontmatter date everywhere it
	// renders, mirroring the article list (see card.svelte).
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
	<meta property="og:url" content={locale.canonical} />
	<meta property="og:locale" content={locale.languageTag} />
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

	<!-- eslint-disable-next-line svelte/no-at-html-tags -- escaped by ldJson above -->
	{@html ldJson(article)}
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<Toc />
	<article class="mx-auto max-w-180 px-6 py-24">
		<header>
			<h1 class="text-text-strong">{meta.title}</h1>
			<div class="mt-2 flex flex-wrap items-center gap-2 text-sm text-text-soft">
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
				<LanguageSwitcher code={locale.code} sourceLanguage={meta.lang} />
				{#if summary}
					<!-- A disclosure, not a menu: it is deliberately not dismissed by clicking
					     elsewhere, because a reader comparing the summary against the article is
					     doing exactly that -- clicking elsewhere. Only the trigger closes it. -->
					<button
						type="button"
						id={summaryTrigger}
						aria-expanded={summaryOpen}
						aria-controls={summaryPanel}
						onclick={() => (summaryOpen = !summaryOpen)}
						class="-mx-1 inline-flex cursor-pointer items-center gap-1 rounded-sm px-1 py-0.5 hover:bg-paper-hover hover:text-text-strong"
					>
						<Sparkles class="size-3.5" aria-hidden="true" />
						<span>{m['article.summary']({}, { locale: locale.code })}</span>
					</button>
				{/if}
			</div>
			{#if summary}
				<!-- Rows collapse to 0fr rather than the box to height 0, which is the one way to
				     animate to a height nobody measured. See spec/architecture.md on motion. -->
				<div class="summary-shell" data-open={summaryOpen}>
					<div class="overflow-hidden">
						<p
							id={summaryPanel}
							role="region"
							aria-labelledby={summaryTrigger}
							class="mt-3 border-l-2 border-border-strong pr-3 pl-3 text-sm leading-relaxed text-text-soft"
						>
							{summary}
						</p>
					</div>
				</div>
			{/if}
			{#if locale.code !== 'mw'}
				<TranslationNotice code={locale.code} sourceLanguage={meta.lang} />
			{/if}
		</header>

		<div class="article-body mt-8 leading-relaxed">
			{@render children()}
		</div>
	</article>
</main>

<style>
	/* Animating to `height: auto` is not possible, so the grid row is animated instead: 0fr to
	   1fr resolves against the content's own height without anyone measuring it. The child
	   needs `overflow: hidden` for the clip to happen. */
	.summary-shell {
		display: grid;
		grid-template-rows: 0fr;
		transition: grid-template-rows 260ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	.summary-shell[data-open='true'] {
		grid-template-rows: 1fr;
	}

	@media (prefers-reduced-motion: reduce) {
		.summary-shell {
			transition: none;
		}
	}

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
