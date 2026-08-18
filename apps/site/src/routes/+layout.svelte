<script lang="ts">
	import { browser, dev } from '$app/environment';
	import { page } from '$app/state';
	import { URLS, pickUrls } from '@canmi/urls';
	import { createSyncStoragePersister } from '@tanstack/query-sync-storage-persister';
	import { QueryClient } from '@tanstack/svelte-query';
	import { PersistQueryClientProvider } from '@tanstack/svelte-query-persist-client';
	import { installFocusSourceTracker } from '$lib/client/focus-source';
	import { localeUrl } from '$lib/locale';
	import { QUERY_CACHE_MAX_AGE, QUERY_STALE_TIME } from '$lib/query';
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
	/**
	 * One robots directive per page, emitted in one place.
	 *
	 * It used to be a fixed `index, follow` in app.html, which meant a page wanting anything
	 * else appended a second, contradicting tag -- two directives that only behave because
	 * crawlers resolve a conflict by taking the most restrictive. Defaulting here instead lets
	 * a page replace the value rather than argue with it, and the default stays visible in the
	 * markup for every page that never thinks about it.
	 */
	const robots = $derived(
		'robots' in page.data && typeof page.data.robots === 'string'
			? page.data.robots
			: 'index, follow',
	);
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				staleTime: QUERY_STALE_TIME,
				gcTime: QUERY_CACHE_MAX_AGE,
			},
		},
	});
	const persister = createSyncStoragePersister({
		storage: browser ? localStorage : undefined,
		key: 'cache',
	});
	const persistOptions = {
		persister,
		maxAge: QUERY_CACHE_MAX_AGE,
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
			...(site.author.twitter
				? { url: `${URLS.external.social.twitter}/${site.author.twitter}` }
				: {}),
		},
	};
</script>

<svelte:head>
	<link rel="preconnect" href={cdn} crossorigin="anonymous" />
	<link rel="preconnect" href={URLS.external.googleFonts.css} />
	<link rel="preconnect" href={URLS.external.googleFonts.static} crossorigin="anonymous" />
	<link rel="preconnect" href={new URL(URLS.external.github.cdn).origin} crossorigin="anonymous" />
	<link
		rel="stylesheet"
		href="{URLS.external.googleFonts
			.css}/css2?family=Inter:wght@400;500;600;700&family=Libre+Baskerville:ital@1&family=Noto+Sans+SC:wght@400;500;600;700&display=swap"
	/>
	<link rel="canonical" href={canonical} />
	<meta name="robots" content={robots} />
	{#each articleLocale?.alternates ?? [] as alternate (alternate.code)}
		<link rel="alternate" hreflang={alternate.languageTag} href={alternate.href} />
	{/each}
	<!-- Site-wide, so it sits here rather than being repeated by every page that has a card. -->
	<meta property="og:site_name" content={site.name} />
	{#if site.author.twitter}
		<meta name="twitter:site" content="@{site.author.twitter}" />
		<meta name="twitter:creator" content="@{site.author.twitter}" />
	{/if}
	<!-- Safe despite the raw insertion: ldJson escapes what it serialises. Stated rather than
	     suppressed, because no linter here checks it -- oxlint does not parse svelte templates
	     and has no svelte plugin, so a `svelte/no-at-html-tags` directive would be decoration.
	     See spec/lint-format.md. -->
	{@html ldJson(website)}
	<link rel="alternate" type="application/atom+xml" href={feed} title={site.name} />
	<link rel="llms" type="text/markdown" href="/llms.txt" />
	<link rel="icon" type="image/png" sizes="96x96" href="{cdn}/favicon-96x96.png" />
	<link rel="icon" type="image/png" sizes="512x512" href="{cdn}/favicon-512x512.png" />
	<link rel="icon" type="image/svg+xml" sizes="any" href="{cdn}/favicon.svg" />
	<link rel="apple-touch-icon" href="{cdn}/apple-touch-icon.png" />
	<!-- Loaded in development too; data-domains keeps a dev session from reporting.
	     See spec/analytics.md. -->
	<script
		defer
		fetchpriority="low"
		src={URLS.external.umami}
		data-website-id="2b0a1e79-405a-47c0-a263-05732e0a130c"
		data-domains={new URL(URLS.apps.production.site).hostname}
		data-exclude="/@/*"
	></script>
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
