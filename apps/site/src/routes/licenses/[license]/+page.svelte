<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import PackageList from '$lib/licenses/package-list.svelte';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const numberLocale = $derived(locale === 'mw' ? 'en' : locale === 'tw' ? 'zh-TW' : locale);
	const count = $derived(new Intl.NumberFormat(numberLocale).format(data.license.count));
</script>

<svelte:head>
	<title>{data.license.license} · {m['licenses.title']({}, { locale })}</title>
	<meta
		name="description"
		content={m['licenses.license_description'](
			{ license: data.license.license, count },
			{ locale },
		)}
	/>
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
			<h1 class="break-words text-text-strong">{data.license.license}</h1>
			<p class="mt-4 leading-relaxed text-pretty text-text-soft">
				{m['licenses.license_summary']({ count }, { locale })}
			</p>
			<nav aria-label={m['licenses.actions']({}, { locale })} class="mt-4 flex flex-wrap gap-4">
				<a
					href={data.spdxHref}
					target="_blank"
					rel="noopener noreferrer"
					class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
				>
					<ExternalLink class="size-4" aria-hidden="true" />
					<span>{m['licenses.spdx']({}, { locale })}</span>
				</a>
				<LanguageSwitcher code={locale} />
			</nav>
		</header>

		{#each data.groups as group (group.registry)}
			<section aria-labelledby="registry-{group.registry}" class="mt-16">
				<div class="mb-3 flex items-baseline justify-between gap-4">
					<h2 id="registry-{group.registry}" class="font-medium text-text-strong">{group.name}</h2>
					<span class="font-mono text-[0.8125rem] tabular-nums text-text-soft">{group.rows.length}</span>
				</div>
				<PackageList rows={group.rows} {locale} license={data.license.license} />
			</section>
		{/each}
	</article>
</main>
