<script lang="ts">
	import { dev } from '$app/environment';
	import { imgsrc } from '@canmi/imgsrc';
	import { pickUrls, URLS } from '@canmi/urls';
	import Coffee from '@lucide/svelte/icons/coffee';
	import Lollipop from '@lucide/svelte/icons/lollipop';
	import ArticleList from '$lib/article/list.svelte';
	import Modal from '$lib/components/modal.svelte';
	import PageBody from '$lib/home/body.svelte';
	import Icon from '$lib/home/icons.svelte';
	import Newsletter from '$lib/newsletter/newsletter.svelte';
	import { CARD_HEIGHT, CARD_WIDTH, HOME_SLUG, cardUrl } from '$lib/opengraph';
	import * as m from '$lib/paraglide/messages';
	import { site } from '$lib/site';
	import Support from '$lib/support/support.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let sponsorOpen = $state(false);

	const cdnUrl = pickUrls(dev).cdn;
	const avatarSrc = imgsrc(`github:avatar:${site.author.githubId}@192`, { cdnUrl });
	const card = $derived(cardUrl(cdnUrl, HOME_SLUG, data.locale.code));
	const githubProfileUrl = `${URLS.external.github.web}/${site.author.github}`;
	const googleSourceUrl = new URL(URLS.external.google.sourcePreferences);
	googleSourceUrl.searchParams.set('q', URLS.apps.production.site);
	// Icons are center-anchored, so each is size-compensated independently. Base
	// matches the inline text icons (h-4); wide/flat glyphs (Telegram) get a larger
	// box to read optically equal.
	const base = 'h-4 w-4';
	// `document` keeps server-only resources out of the client page router.
	// See spec/locale.md#server-only-documents-leave-the-page-router.
	const links = [
		{ name: 'github', label: 'GitHub', href: githubProfileUrl, size: base },
		...(site.author.twitter
			? ([
					{
						name: 'twitter',
						label: 'Twitter',
						href: `${URLS.external.social.twitterIntent}?screen_name=${site.author.twitter}`,
						size: base,
					},
				] as const)
			: []),
		{
			name: 'nyaone',
			label: 'Nya.one',
			href: `${URLS.external.social.fediverse}/@${site.author.fediverse}`,
			size: base,
		},
		{
			name: 'bluesky',
			label: 'Bluesky',
			href: `${URLS.external.social.bluesky}/${site.author.bluesky}`,
			size: base,
		},
		{
			name: 'telegram',
			label: 'Telegram',
			href: `${URLS.external.social.telegram}/${site.author.telegram}`,
			size: 'h-5 w-5',
		},
		{ name: 'sitemap', label: 'Sitemap', href: '/sitemap.xml', size: base, document: true },
		{
			name: 'travellings',
			label: 'Travellings',
			href: URLS.external.webring.travellings,
			size: base,
		},
		{ name: 'moe', label: 'Travellings Moe', href: URLS.external.webring.moe, size: base },
		{ name: 'rss', label: 'RSS feed', href: '/atom.xml', size: base, document: true },
	] as const;
</script>

<svelte:head>
	<title>{data.title}</title>
	<meta name="description" content={data.description} />
	<!--
		The home page had no card at all, while `cms og` had been rendering one for it since the
		beginning. A page that advertises nothing is shared as a bare link, which is the one
		place a card is most worth having.
	-->
	<meta property="og:type" content="website" />
	<meta property="og:title" content={data.title} />
	<meta property="og:description" content={data.description} />
	<meta property="og:url" content={URLS.apps.production.site} />
	<meta property="og:image" content={card} />
	<meta property="og:image:width" content={CARD_WIDTH} />
	<meta property="og:image:height" content={CARD_HEIGHT} />
	<meta property="og:image:alt" content={data.title} />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<header class="flex items-center gap-3">
			<img
				src={avatarSrc}
				alt={`${site.author.name}'s avatar`}
				width="52"
				height="52"
				fetchpriority="high"
				class="h-13 w-13 rounded-full border-2 border-border"
			/>
			<div>
				<h1 class="text-text-strong">{site.author.fullName}</h1>
				<p class="text-text-soft">{site.author.role}</p>
			</div>
		</header>

		<!-- Bio prose is compiled from contents/index.md (DLC directives), single-sourced
		with /llms.txt. PageBody keeps styled text as dead HTML and renders each social
		link live so its icon reuses the shared <Icon> component. -->
		<div class="mt-8 space-y-4 leading-relaxed text-pretty">
			<PageBody blocks={data.bio} locale={data.locale.code} />
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
							: `${link.label} (${m['support.new-tab']({}, { locale: data.locale.code })})`}
						title={link.label}
						data-sveltekit-reload={'document' in link ? true : undefined}
						class="focus-ring inline-flex size-5 items-center justify-center rounded-[0.3125rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
						{...link.href.startsWith('/') ? {} : { target: '_blank', rel: 'noopener noreferrer' }}
					>
						<Icon name={link.name} class={link.size} />
					</a>
				{/each}
			</nav>

			<a
				href="{URLS.external.icpmoe}/?keyword=20260000"
				target="_blank"
				rel="noopener noreferrer"
				class="focus-link inline-flex items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
			>
				<Lollipop class="h-4 w-4" aria-hidden="true" />
				<span>ICP 20260000</span>
				<span class="sr-only">({m['support.new-tab']({}, { locale: data.locale.code })})</span>
			</a>
		</div>
	</article>
</main>

<Modal
	open={sponsorOpen}
	title={m['sponsor.title']({}, { locale: data.locale.code })}
	closeLabel={m['sponsor.close']({}, { locale: data.locale.code })}
	onOpenChange={(open) => (sponsorOpen = open)}
>
	{#snippet icon()}
		<Coffee class="size-4" aria-hidden="true" />
	{/snippet}
	{m['sponsor.notice']({}, { locale: data.locale.code })}
</Modal>
