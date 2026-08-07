<script lang="ts">
	import { browser, dev } from '$app/environment';
	import { page } from '$app/state';
	import { URLS, pickUrls } from '@canmi/urls';
	import { createSyncStoragePersister } from '@tanstack/query-sync-storage-persister';
	import { QueryClient } from '@tanstack/svelte-query';
	import { PersistQueryClientProvider } from '@tanstack/svelte-query-persist-client';
	import { installFocusSourceTracker } from '$lib/client/focus-source';
	import {
		ENGAGEMENT_CACHE_MAX_AGE,
		ENGAGEMENT_STALE_TIME,
	} from '$lib/engagement/engagement.svelte';
	import { localeUrl } from '$lib/locale';
	import { site } from '$lib/site';
	import '../styles/app.css';
	import '@canmi/fonts/mono.css';

	const cdn = pickUrls(dev).cdn;
	const locale = $derived('locale' in page.data ? page.data.locale : undefined);
	const articleLocale = $derived(
		locale && 'canonical' in locale && 'alternates' in locale ? locale : undefined,
	);
	const canonical = $derived(
		articleLocale?.canonical ?? `${URLS.apps.production.site}${page.url.pathname}`,
	);
	const feed = $derived(localeUrl('/atom.xml', locale?.code ?? 'mw'));
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				staleTime: ENGAGEMENT_STALE_TIME,
				gcTime: ENGAGEMENT_CACHE_MAX_AGE,
			},
		},
	});
	const persister = createSyncStoragePersister({
		storage: browser ? localStorage : undefined,
		key: 'cache',
	});
	const persistOptions = {
		persister,
		maxAge: ENGAGEMENT_CACHE_MAX_AGE,
		dehydrateOptions: {
			shouldDehydrateQuery: (query: { state: { status: string } }) =>
				query.state.status === 'success',
			shouldDehydrateMutation: () => false,
		},
	};
	let { children } = $props();

	$effect(() => installFocusSourceTracker());

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

	/**
	 * Structured data for the site itself.
	 *
	 * Built from the same config the visible chrome reads, so there is no second copy of the
	 * site's name to fall out of step. An article adds its own `Article` node; this one says
	 * what the site is, which no page has to repeat.
	 */
	const website = {
		'@context': 'https://schema.org',
		'@type': 'WebSite',
		name: site.name,
		description: site.tagline,
		url: URLS.apps.production.site,
		author: {
			'@type': 'Person',
			name: site.author.name,
			...(site.author.x ? { url: `${URLS.external.social.x}/${site.author.x}` } : {}),
		},
	};
</script>

<svelte:head>
	<link rel="preconnect" href={cdn} crossorigin="anonymous" />
	<link rel="canonical" href={canonical} />
	{#each articleLocale?.alternates ?? [] as alternate (alternate.code)}
		<link rel="alternate" hreflang={alternate.languageTag} href={alternate.href} />
	{/each}
	<!-- Site-wide, so it sits here rather than being repeated by every page that has a card. -->
	<meta property="og:site_name" content={site.name} />
	{#if site.author.x}
		<meta name="twitter:site" content="@{site.author.x}" />
		<meta name="twitter:creator" content="@{site.author.x}" />
	{/if}
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- escaped by ldJson above -->
	{@html ldJson(website)}
	<link rel="alternate" type="application/atom+xml" href={feed} title={site.name} />
	<link rel="llms" type="text/markdown" href="/llms.txt" />
	<link rel="icon" type="image/png" sizes="96x96" href="{cdn}/favicon-96x96.png" />
	<link rel="icon" type="image/png" sizes="512x512" href="{cdn}/favicon-512x512.png" />
	<link rel="icon" type="image/svg+xml" sizes="any" href="{cdn}/favicon.svg" />
	<link rel="apple-touch-icon" href="{cdn}/apple-touch-icon.png" />
	{#if !dev}
		<script
			defer
			src={URLS.external.insights}
			data-cf-beacon={`{"token": "c004f82a8f14429694781a554291a897"}`}
		></script>
	{/if}
</svelte:head>

<PersistQueryClientProvider client={queryClient} {persistOptions}>
	{@render children()}
</PersistQueryClientProvider>
