<script lang="ts">
	import { dev } from '$app/environment';
	import { page } from '$app/state';
	import { URLS, pickUrls } from '@canmi/urls';
	import { site } from '$lib/site';
	import '../styles/app.css';
	import '@canmi/fonts/mono.css';

	const cdn = pickUrls(dev).cdn;
	const canonical = $derived(`${URLS.apps.production.site}${page.url.pathname}`);
	let { children } = $props();
</script>

<svelte:head>
	<link rel="preconnect" href={cdn} crossorigin="anonymous" />
	<link rel="canonical" href={canonical} />
	<link rel="alternate" type="application/atom+xml" href="/atom.xml" title={site.name} />
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

{@render children()}
