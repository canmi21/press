<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const numberLocale = $derived(locale === 'mw' ? 'en' : locale === 'tw' ? 'zh-TW' : locale);
	const count = $derived(new Intl.NumberFormat(numberLocale).format(data.total));
</script>

<svelte:head>
	<title>{m['licenses.packages']({}, { locale })} · {m['licenses.title']({}, { locale })}</title>
	<meta name="description" content={m['licenses.packages_description']({ count }, { locale })} />
	<meta name="robots" content="noindex, follow" />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<nav aria-label={m['licenses.breadcrumb']({}, { locale })}>
			<a
				href="/licenses"
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<ArrowLeft class="size-4" aria-hidden="true" />
				<span>{m['licenses.all_licenses']({}, { locale })}</span>
			</a>
		</nav>

		<header class="mt-8">
			<h1 class="text-text-strong">{m['licenses.packages']({}, { locale })}</h1>
			<p class="mt-4 leading-relaxed text-pretty text-text-soft">
				{m['licenses.packages_summary']({ count }, { locale })}
			</p>
			<div class="mt-4 text-[0.9375rem] text-text-soft"><LanguageSwitcher code={locale} /></div>
		</header>

		<section aria-labelledby="registry-directory" class="mt-16">
			<h2 id="registry-directory" class="mb-3 font-medium text-text-strong">
				{m['licenses.registries']({}, { locale })}
			</h2>
			{#each data.registries as registry (registry.id)}
				<a
					href={registry.href}
					class="focus-ring-within -mx-2 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
				>
					<span class="flex min-w-0 items-center gap-3">
						<span class="focus-link-inner min-w-0 truncate">{registry.name}</span>
						<span class="h-0 min-w-6 flex-1 border-t border-dashed border-border-strong"></span>
					</span>
					<span class="font-mono text-[0.9375rem] tabular-nums text-text-soft">{registry.count}</span>
				</a>
			{/each}
		</section>
	</article>
</main>
