<script lang="ts">
	import { dev } from '$app/environment';
	import { pickUrls, URLS } from '@canmi/urls';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { localeUrl } from '$lib/locale';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import PackageList from '$lib/licenses/package-list.svelte';
	import { CARD_HEIGHT, CARD_WIDTH, cardUrl } from '$lib/opengraph';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';
	import { compactCount, intlLocale } from '$lib/format';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const numberLocale = $derived(intlLocale(locale));
	const count = $derived(new Intl.NumberFormat(numberLocale).format(data.license.count));
	const title = $derived(data.license.license);
	const description = $derived(
		m['licenses.license_description']({ license: data.license.license, count }, { locale }),
	);
	const slug = $derived(`licenses/${data.license.slug}`);
	const cdn = pickUrls(dev).cdn;
	const canonical = $derived(localeUrl(`${URLS.apps.production.site}/${slug}`, locale));
	const card = $derived(cardUrl(cdn, slug, locale));
</script>

<svelte:head>
	<title>{title} · {m['licenses.title']({}, { locale })}</title>
	<meta name="description" content={description} />
	<meta property="og:type" content="website" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	<meta property="og:image" content={card} />
	<meta property="og:image:width" content={CARD_WIDTH} />
	<meta property="og:image:height" content={CARD_HEIGHT} />
	<meta property="og:image:alt" content={title} />
	<meta name="twitter:card" content="summary_large_image" />
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
					class="quiet-control text-[0.9375rem]"
				>
					<span class="focus-link-inner inline-flex items-center gap-1.5">
						<ExternalLink class="size-3.5" aria-hidden="true" />
						<span>{m['licenses.spdx']({}, { locale })}</span>
					</span>
				</a>
				<LanguageSwitcher code={locale} />
			</nav>
		</header>

		{#each data.groups as group (group.registry)}
			<section aria-labelledby="registry-{group.registry}" class="mt-16">
				<div class="mb-3 flex items-baseline justify-between gap-4">
					<h2 id="registry-{group.registry}" class="font-medium text-text-strong">{group.name}</h2>
					<span class="font-mono text-[0.8125rem] tabular-nums text-text-soft"
						>{compactCount(group.rows.length)}</span
					>
				</div>
				<PackageList rows={group.rows} {locale} license={data.license.license} />
			</section>
		{/each}
	</article>
</main>
