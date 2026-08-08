<script lang="ts">
	import { URLS } from '@canmi/urls';
	import FileText from '@lucide/svelte/icons/file-text';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Scale from '@lucide/svelte/icons/scale';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import { spaceScriptBoundaries } from '$lib/locale/spacing';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
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
	<title>{m['licenses.title']({}, { locale })}</title>
	<meta name="description" content={m['licenses.description']({}, { locale })} />
	<meta name="robots" content="noindex, follow" />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<header>
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
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<FolderOpen class="size-4" aria-hidden="true" />
				<span>{m['licenses.packages']({}, { locale })}</span>
			</a>
			<a
				href="/licenses.txt"
				data-sveltekit-reload
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<FileText class="size-4" aria-hidden="true" />
				<span>{m['licenses.index']({}, { locale })}</span>
			</a>
			<a
				href="/licenses/full.txt"
				data-sveltekit-reload
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<Scale class="size-4" aria-hidden="true" />
				<span>{m['licenses.full']({}, { locale })}</span>
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
