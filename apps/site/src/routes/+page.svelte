<script lang="ts">
	import { dev } from '$app/environment';
	import { imgsrc } from '@canmi/imgsrc';
	import { pickUrls } from '@canmi/urls';
	import GitMerge from '@lucide/svelte/icons/git-merge';
	import Lollipop from '@lucide/svelte/icons/lollipop';
	import ArticleList from '$lib/article-list.svelte';
	import PageBody from '$lib/content/page-body.svelte';
	import Icon from '$lib/icons.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const cdnUrl = pickUrls(dev).cdn;
	const avatarSrc = imgsrc('github:avatar:72544151@192', { cdnUrl });
	const commitHash = import.meta.env.VITE_COMMIT_HASH;

	// Icons are center-anchored, so each is size-compensated independently. Base
	// matches the inline text icons (h-4); wide/flat glyphs (Telegram) get a larger
	// box to read optically equal.
	const base = 'h-4 w-4';
	const links = [
		{ name: 'github', label: 'GitHub', href: 'https://github.com/canmi21', size: base },
		{
			name: 'twitter',
			label: 'X',
			href: 'https://twitter.com/intent/follow?screen_name=canmi21',
			size: base
		},
		{ name: 'nyaone', label: 'Nya.one', href: 'https://nya.one/@canmi', size: base },
		{ name: 'bluesky', label: 'Bluesky', href: 'https://bsky.app/profile/canmi.net', size: base },
		{ name: 'telegram', label: 'Telegram', href: 'https://t.me/canmi21', size: 'h-5 w-5' },
		{ name: 'sitemap', label: 'Sitemap', href: '/sitemap.xml', size: base },
		{
			name: 'travellings',
			label: 'Travellings',
			href: 'https://www.travellings.cn/go.html',
			size: base
		},
		{ name: 'moe', label: 'Travellings Moe', href: 'https://travel.moe/go?travel=on', size: base },
		{ name: 'rss', label: 'RSS feed', href: '/atom.xml', size: base }
	] as const;
</script>

<svelte:head>
	<title>{data.title}</title>
	<meta name="description" content={data.description} />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<header class="flex items-center gap-3">
			<img
				src={avatarSrc}
				alt="Canmi's avatar"
				width="52"
				height="52"
				fetchpriority="high"
				class="h-13 w-13 rounded-full border-2 border-border"
			/>
			<div>
				<h1 class="text-text-strong">Canmi Wu</h1>
				<p class="text-text-soft">Systems Engineer</p>
			</div>
		</header>

		<!-- Bio prose is compiled from contents/index.md (DLC directives), single-sourced
		with /llms.txt. PageBody keeps styled text as dead HTML and renders each social
		link live so its icon reuses the shared <Icon> component. -->
		<div class="mt-8 space-y-4 leading-relaxed text-pretty">
			<PageBody blocks={data.bio} />
		</div>

		<ArticleList articles={data.articles} />

		<!-- Tune the focus ring for the whole icon group: a 0.125rem (2px) gap with
		a 0.3125rem (5px) radius, inherited by each link's :focus-visible. Each link
		is a fixed 1.25rem box so the ring is identical across the row regardless of
		a glyph's optical-compensation size (e.g. Telegram's larger icon). -->
		<nav
			aria-label="Find me elsewhere"
			class="mt-20 flex flex-wrap items-center justify-center gap-3 [--focus-ring-offset:0.125rem] [--focus-ring-radius:0.3125rem]"
		>
			{#each links as { name, href, label, size } (label)}
				<a
					{href}
					aria-label={href.startsWith('/') ? label : `${label} (opens in new tab)`}
					title={label}
					class="inline-flex size-5 items-center justify-center text-text-soft transition-colors duration-200 hover:text-text-strong"
					{...href.startsWith('/') ? {} : { target: '_blank', rel: 'noopener noreferrer' }}
				>
					<Icon {name} class={size} />
				</a>
			{/each}
		</nav>

		<div
			class="mt-6 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs text-text-soft"
		>
			<a
				href="https://icp.gov.moe/?keyword=20260000"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-1.5 transition-colors duration-200 hover:text-text-strong"
			>
				<Lollipop class="h-3.5 w-3.5" aria-hidden="true" />
				<span>ICP 20260000</span>
				<span class="sr-only">(opens in new tab)</span>
			</a>
			<span class="inline-flex items-center gap-1.5">
				<GitMerge class="h-3.5 w-3.5" aria-hidden="true" />
				<span class="font-mono">{commitHash}</span>
			</span>
		</div>
	</article>
</main>
