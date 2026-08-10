<script lang="ts">
	import { dev } from '$app/environment';
	import { pickUrls, URLS } from '@canmi/urls';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import FileText from '@lucide/svelte/icons/file-text';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Scale from '@lucide/svelte/icons/scale';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import { localeUrl } from '$lib/locale';
	import { spaceScriptBoundaries } from '$lib/locale/spacing';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import { CARD_HEIGHT, CARD_WIDTH, cardUrl } from '$lib/opengraph';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const title = $derived(m['licenses.title']({}, { locale }));
	const description = $derived(m['licenses.description']({}, { locale }));
	const slug = 'licenses';
	const cdn = pickUrls(dev).cdn;
	const canonical = $derived(localeUrl(`${URLS.apps.production.site}/${slug}`, locale));
	const card = $derived(cardUrl(cdn, slug, locale));
	const numberLocale = $derived(locale === 'mw' ? 'en' : locale === 'tw' ? 'zh-TW' : locale);
	const count = $derived(new Intl.NumberFormat(numberLocale).format(data.total));
	const registryParts = $derived.by(() => {
		const parts = new Intl.ListFormat(numberLocale, { style: 'long', type: 'conjunction' })
			.formatToParts(data.registries.map(({ name }) => name));
		const spaced = spaceScriptBoundaries(parts.map(({ value }) => value));
		return parts.map((part, index) => ({
			type: part.type,
			value: part.value,
			gapBefore: (spaced[index] ?? '').length > part.value.length,
			registry: part.type === 'element'
				? data.registries.find(({ name }) => name === part.value)
				: undefined,
		}));
	});
</script>

<svelte:head>
	<title>{title}</title>
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
	<!--
		No robots meta: the directory pages of the licence surface are indexable, which is what
		app.html already says by default. Only one page here departs from it, and that is the
		individual package page, which sets `noindex, follow` itself. See the sitemap route.
	-->
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<nav aria-label={m['licenses.breadcrumb']({}, { locale })}>
			<a
				href="/"
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<ArrowLeft class="size-4" aria-hidden="true" />
				<span>{m['nav.home']({}, { locale })}</span>
			</a>
		</nav>

		<header class="mt-8">
			<h1 class="text-text-strong">{m['licenses.title']({}, { locale })}</h1>
			<div class="mt-4 space-y-4 leading-relaxed text-pretty">
				<p>{m['licenses.built']({}, { locale })}</p>
				<p>{m['licenses.thanks']({}, { locale })}</p>
				<p class="text-text-soft">
					<ParaglideMessage message={m['licenses.below']} inputs={{}} options={{ locale }}>
						{#snippet license()}<a
								href="{URLS.external.spdx}/MIT.html"
								target="_blank"
								rel="noopener noreferrer"
								class="focus-link spring-underline article-link text-text"
								>{m['licenses.mit']({}, { locale })}</a
							>{/snippet}
						{#snippet link()}<a
								href={URLS.source}
								target="_blank"
								rel="noopener noreferrer"
								class="focus-link spring-underline article-link text-text"
								>{m['licenses.repository']({}, { locale })}</a
							>{/snippet}
					</ParaglideMessage>
				</p>
			</div>
		</header>

		<p class="mt-8 text-pretty text-text-soft">
			<ParaglideMessage
				message={m['licenses.census']}
				inputs={{ count, licenses: data.licenses.length }}
				options={{ locale }}
			>
				{#snippet registries()}{#each registryParts as part, index (index)}{#if part.gapBefore}{' '}{/if}{#if part.registry}<a
							href={part.registry.href}
							target="_blank"
							rel="noopener noreferrer"
							class="focus-link spring-underline article-link text-text">{part.value}</a
						>{:else}{part.value}{/if}{/each}{/snippet}
			</ParaglideMessage>
		</p>

		<nav aria-label={m['licenses.actions']({}, { locale })} class="mt-4 flex flex-wrap gap-4">
			<a
				href="/licenses/pkgs"
				class="quiet-control text-[0.9375rem]"
			>
				<span class="focus-link-inner inline-flex items-center gap-1.5">
					<FolderOpen class="size-3.5" aria-hidden="true" />
					<span>{m['licenses.packages']({}, { locale })}</span>
				</span>
			</a>
			<a
				href="/licenses.txt"
				data-sveltekit-reload
				class="quiet-control text-[0.9375rem]"
			>
				<span class="focus-link-inner inline-flex items-center gap-1.5">
					<FileText class="size-3.5" aria-hidden="true" />
					<span>{m['licenses.index']({}, { locale })}</span>
				</span>
			</a>
			<a
				href="/licenses/full.txt"
				data-sveltekit-reload
				class="quiet-control text-[0.9375rem]"
			>
				<span class="focus-link-inner inline-flex items-center gap-1.5">
					<Scale class="size-3.5" aria-hidden="true" />
					<span>{m['licenses.full']({}, { locale })}</span>
				</span>
			</a>
			<LanguageSwitcher code={locale} />
		</nav>

		<section aria-labelledby="license-directory" class="mt-16">
			<h2 id="license-directory" class="mb-3 font-medium text-text-strong">
				{m['licenses.directory']({}, { locale })}
			</h2>
			{#each data.licenses as entry (entry.slug)}
				<a
					href="/licenses/{entry.slug}"
					class="focus-ring-within -mx-2 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
				>
					<span class="flex min-w-0 items-center gap-3">
						<span class="focus-link-inner min-w-0 truncate">{entry.license}</span>
						<span class="h-0 min-w-6 flex-1 border-t border-dashed border-border-strong"></span>
					</span>
					<span class="font-mono text-[0.9375rem] tabular-nums text-text-soft">{entry.count}</span>
				</a>
			{/each}
			<p class="mt-3 text-[0.8125rem] text-pretty text-text-soft">
				<span aria-hidden="true">*&nbsp;</span>{m['licenses.multiple']({}, { locale })}
			</p>
		</section>
	</article>
</main>
