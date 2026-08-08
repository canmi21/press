<script lang="ts">
	import { dev } from '$app/environment';
	import { pickUrls } from '@canmi/urls';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import FileText from '@lucide/svelte/icons/file-text';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import { githubAvatar, textUrl } from '$lib/licenses';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const cdn = pickUrls(dev).cdn;
</script>

<svelte:head>
	<title>{data.coordinates.name} {data.coordinates.version} · {m['licenses.title']({}, { locale })}</title>
	<meta
		name="description"
		content={data.entry.description
			?? m['licenses.package_description'](
				{ name: data.coordinates.name, version: data.coordinates.version },
				{ locale },
			)}
	/>
	<meta name="robots" content="noindex, follow" />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<nav
			aria-label={m['licenses.breadcrumb']({}, { locale })}
			class="flex min-w-0 flex-wrap items-center gap-x-2 text-[0.9375rem] text-text-soft"
		>
			<a
				href="/licenses"
				class="focus-link transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
				>{m['licenses.all_licenses']({}, { locale })}</a
			>
			<span aria-hidden="true">/</span>
			<a
				href="/licenses/pkgs"
				class="focus-link transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
				>{m['licenses.packages']({}, { locale })}</a
			>
			<span aria-hidden="true">/</span>
			<a
				href={data.registry.directoryHref}
				class="focus-link transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
				>{data.registry.name}</a
			>
		</nav>

		<header class="mt-8">
			<div class="flex min-w-0 flex-wrap items-baseline gap-x-3 gap-y-1">
				<h1 class="min-w-0 break-words text-text-strong">{data.coordinates.name}</h1>
				<span class="font-mono text-[0.9375rem] text-text-soft">{data.coordinates.version}</span>
			</div>
			{#if data.entry.description}
				<p class="mt-4 leading-relaxed text-pretty text-text-soft">{data.entry.description}</p>
			{/if}
			<nav aria-label={m['licenses.actions']({}, { locale })} class="mt-4 flex flex-wrap gap-4">
				<a
					href={data.textHref}
					data-sveltekit-reload
					class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
				>
					<FileText class="size-4" aria-hidden="true" />
					<span>{m['licenses.package_notice']({}, { locale })}</span>
				</a>
				<LanguageSwitcher code={locale} />
			</nav>
		</header>

		<section aria-labelledby="package-metadata" class="mt-16">
			<h2 id="package-metadata" class="mb-4 font-medium text-text-strong">
				{m['licenses.package']({}, { locale })}
			</h2>
			<dl class="grid grid-cols-[6.5rem_minmax(0,1fr)] gap-x-4 gap-y-3 text-[0.9375rem]">
				<dt class="text-text-soft">{m['licenses.registry']({}, { locale })}</dt>
				<dd class="min-w-0">
					<a
						href={data.registry.packageHref}
						target="_blank"
						rel="noopener noreferrer"
						class="focus-link spring-underline article-link inline-flex max-w-full items-center gap-1.5"
					>
						<span class="truncate">{data.registry.name}</span>
						<ExternalLink class="size-3.5 shrink-0" aria-hidden="true" />
					</a>
				</dd>
				{#if data.repository.href}
					<dt class="text-text-soft">{m['licenses.repository_label']({}, { locale })}</dt>
					<dd class="min-w-0">
						{#if data.repository.github}
							<span class="inline-flex min-w-0 items-center gap-2">
								<img
									src={githubAvatar(cdn, data.repository.github.owner, 48)}
									alt=""
									width="24"
									height="24"
									loading="lazy"
									class="size-6 shrink-0 rounded-full bg-paper"
								/>
								<span class="min-w-0 truncate">
									<a
										href="{data.githubHref}/{data.repository.github.owner}"
										target="_blank"
										rel="noopener noreferrer"
										class="focus-link spring-underline article-link"
										>{data.repository.github.owner}</a
									>/<a
										href={data.repository.github.url}
										target="_blank"
										rel="noopener noreferrer"
										class="focus-link spring-underline article-link"
										>{data.repository.github.name}</a
									>
								</span>
							</span>
						{:else}
							<a
								href={data.repository.href}
								target="_blank"
								rel="noopener noreferrer"
								class="focus-link spring-underline article-link block w-fit max-w-full truncate"
								>{data.repository.href}</a
							>
						{/if}
					</dd>
				{/if}
				{#if data.entry.homepage}
					<dt class="text-text-soft">{m['licenses.homepage']({}, { locale })}</dt>
					<dd class="min-w-0">
						<a
							href={data.entry.homepage}
							target="_blank"
							rel="noopener noreferrer"
							class="focus-link spring-underline article-link block w-fit max-w-full truncate"
							>{data.entry.homepage}</a
						>
					</dd>
				{/if}
				{#if data.entry.documentation}
					<dt class="text-text-soft">{m['licenses.documentation']({}, { locale })}</dt>
					<dd class="min-w-0">
						<a
							href={data.entry.documentation}
							target="_blank"
							rel="noopener noreferrer"
							class="focus-link spring-underline article-link block w-fit max-w-full truncate"
							>{data.entry.documentation}</a
						>
					</dd>
				{/if}
			</dl>
		</section>

		<section aria-labelledby="package-terms" class="mt-16">
			<h2 id="package-terms" class="mb-4 font-medium text-text-strong">
				{m['licenses.terms']({}, { locale })}
			</h2>
			<p class="font-mono text-[0.9375rem] break-words text-text-strong">{data.entry.spdx}</p>
			{#if data.entry.asserted}
				<p class="mt-2 text-[0.8125rem] leading-relaxed text-pretty text-text-soft">
					{m['licenses.asserted_note']({}, { locale })}
				</p>
			{/if}
			<div class="mt-3 flex flex-wrap gap-2">
				{#each data.licenses as license (license.license)}
					<a
						href={license.href}
						class="focus-ring rounded-[0.375rem] border border-border px-2 py-1 font-mono text-[0.8125rem] text-text-soft transition-colors duration-200 hover:border-border-strong hover:text-text-strong focus-visible:text-text-strong"
						>{license.license}</a
					>
				{/each}
			</div>
		</section>

		{#if data.entry.authors?.length}
			<section aria-labelledby="package-authors" class="mt-16">
				<h2 id="package-authors" class="mb-4 font-medium text-text-strong">
					{m['licenses.authors']({}, { locale })}
				</h2>
				<ul class="space-y-3">
					{#each data.entry.authors as author, index (`${author.name}:${author.github ?? ''}:${index}`)}
						<li class="flex min-w-0 items-center gap-3">
							{#if author.github}
								<img
									src={githubAvatar(cdn, author.github, 64)}
									alt=""
									width="32"
									height="32"
									loading="lazy"
									class="size-8 shrink-0 rounded-full bg-paper"
								/>
							{/if}
							<span class="min-w-0">
								<span class="block truncate text-text-strong">{author.name}</span>
								{#if author.github}
									<a
										href="{data.githubHref}/{author.github}"
										target="_blank"
										rel="noopener noreferrer"
										class="focus-link spring-underline article-link block w-fit text-[0.8125rem] text-text-soft"
										>@{author.github}</a
									>
								{/if}
							</span>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<section aria-labelledby="license-files" class="mt-16">
			<h2 id="license-files" class="mb-4 font-medium text-text-strong">
				{m['licenses.files']({}, { locale })}
			</h2>
			{#if data.entry.texts?.length}
				<ul>
					{#each data.entry.texts as file (`${file.name}:${file.cid}`)}
						<li>
							<a
								href={textUrl(cdn, file.cid)}
								target="_blank"
								rel="noopener noreferrer"
								class="focus-ring-within -mx-2 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
							>
								<span class="focus-link-inner min-w-0 truncate">{file.name}</span>
								<span class="font-mono text-[0.75rem] text-text-soft">raw</span>
							</a>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="text-[0.9375rem] text-text-soft">{m['licenses.no_files']({}, { locale })}</p>
			{/if}
		</section>
	</article>
</main>
