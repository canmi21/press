<script lang="ts">
	import { dev } from '$app/environment';
	import { imgsrc } from '@canmi/imgsrc';
	import { pickUrls, URLS } from '@canmi/urls';
	import Lollipop from '@lucide/svelte/icons/lollipop';
	import ArticleList from '$lib/article/list.svelte';
	import Modal from '$lib/components/modal.svelte';
	import PageBody from '$lib/home/body.svelte';
	import Icon from '$lib/home/icons.svelte';
	import Newsletter from '$lib/newsletter/newsletter.svelte';
	import { site } from '$lib/site';
	import Support from '$lib/support/support.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let sponsorOpen = $state(false);

	const cdnUrl = pickUrls(dev).cdn;
	const avatarSrc = imgsrc('github:avatar:72544151@192', { cdnUrl });
	const githubProfileUrl = `${URLS.external.github.web}/canmi21`;
	const repositoryUrl = `${githubProfileUrl}/workspace`;
	const googleSourceUrl = new URL(URLS.external.google.sourcePreferences);
	googleSourceUrl.searchParams.set('q', URLS.apps.production.site);
	// Icons are center-anchored, so each is size-compensated independently. Base
	// matches the inline text icons (h-4); wide/flat glyphs (Telegram) get a larger
	// box to read optically equal.
	const base = 'h-4 w-4';
	// `document` keeps server-only resources out of the client page router.
	// See spec/locale.md#server-only-documents-leave-the-page-router.
	const links = [
		{ name: 'github', label: 'GitHub', href: repositoryUrl, size: base },
		{
			name: 'twitter',
			label: 'X',
			href: 'https://twitter.com/intent/follow?screen_name=canmi21',
			size: base
		},
		{ name: 'nyaone', label: 'Nya.one', href: 'https://nya.one/@canmi', size: base },
		{ name: 'bluesky', label: 'Bluesky', href: 'https://bsky.app/profile/canmi.net', size: base },
		{
			name: 'telegram',
			label: 'Telegram',
			href: `${URLS.external.social.telegram}/${site.author.telegram}`,
			size: 'h-5 w-5'
		},
		{ name: 'sitemap', label: 'Sitemap', href: '/sitemap.xml', size: base, document: true },
		{
			name: 'travellings',
			label: 'Travellings',
			href: 'https://www.travellings.cn/go.html',
			size: base
		},
		{ name: 'moe', label: 'Travellings Moe', href: 'https://travel.moe/go?travel=on', size: base },
		{ name: 'rss', label: 'RSS feed', href: '/atom.xml', size: base, document: true }
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

		<ArticleList articles={data.articles} heading={data.writing} />

		<Newsletter locale={data.locale.code} />

		<Support
			locale={data.locale.code}
			sourcePreferenceHref={googleSourceUrl.href}
			onsponsor={() => (sponsorOpen = true)}
		/>

		<!-- Left-aligned like everything above it: the page is one text column all the way down,
		and a centred footer was the only thing arguing otherwise. The ICP badge shares the row
		rather than taking one of its own. -->
		<div class="mt-12 flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
			<nav aria-label="Find me elsewhere" class="flex flex-wrap items-center gap-3">
			{#each links as link (link.label)}
				<a
					href={link.href}
					aria-label={link.href.startsWith('/')
						? link.label
						: `${link.label} (opens in new tab)`}
					title={link.label}
					data-sveltekit-reload={'document' in link ? true : undefined}
					class="focus-ring inline-flex size-5 items-center justify-center rounded-[0.3125rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
					{...link.href.startsWith('/')
						? {}
						: { target: '_blank', rel: 'noopener noreferrer' }}
				>
					<Icon name={link.name} class={link.size} />
				</a>
			{/each}
			</nav>

			<a
				href="https://icp.gov.moe/?keyword=20260000"
				target="_blank"
				rel="noopener noreferrer"
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<Lollipop class="h-4 w-4" aria-hidden="true" />
				<span>ICP 20260000</span>
				<span class="sr-only">(opens in new tab)</span>
			</a>
		</div>
	</article>
</main>

<Modal
	open={sponsorOpen}
	title="Sponsor unavailable"
	closeLabel="Close sponsor notice"
	onOpenChange={(open) => (sponsorOpen = open)}
>
	Sponsors are currently unavailable due to U.S. F-1 immigration restrictions.
</Modal>
