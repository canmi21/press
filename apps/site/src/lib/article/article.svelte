<script lang="ts">
	import { dev } from '$app/environment';
	import { page } from '$app/state';
	import { pickUrls } from '@canmi/urls';
	import { site } from '$lib/site';
	import BookOpenText from '@lucide/svelte/icons/book-open-text';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Type from '@lucide/svelte/icons/type';
	import IconClaude from '~icons/mingcute/claude-line';
	import IconGemini from '~icons/mingcute/google-gemini-line';
	import IconOpenAi from '~icons/mingcute/openai-line';
	import { layoutWithLines, measureNaturalWidth, prepareWithSegments } from '@chenglou/pretext';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import * as m from '$lib/paraglide/messages';
	import type { Snippet } from 'svelte';
	import type { Alternate, ArticleMeta, ArticleSummary, TocEntry } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';
	import LanguageSwitcher from '$lib/locale/switcher.svelte';
	import { CARD_HEIGHT, CARD_WIDTH, cardUrl } from '$lib/opengraph';
	import Newsletter from '$lib/newsletter/newsletter.svelte';
	import { createReadsQuery } from '$lib/engagement/reads.svelte';
	import { formatCompact } from './format';
	import HomeLink from './home-link.svelte';
	import Toc from './toc.svelte';
	import TranslationNotice from './translation-notice.svelte';
	import IconXai from './xai-icon.svelte';
	import { shortDate } from '$lib/format';

	type ArticleLocale = {
		code: LocaleCode;
		languageTag: string;
		canonical: string;
		alternates: Alternate[];
		translationAvailable: boolean;
	};

	let {
		slug,
		meta,
		toc,
		chars,
		summary,
		locale,
		children,
	}: {
		/** The article's path, which is what the read counter is keyed by. */
		slug: string;
		meta: ArticleMeta;
		toc: TocEntry[];
		chars: number;
		/** The selected locale, or its English fallback. Absent only when neither exists. */
		summary?: ArticleSummary;
		locale: ArticleLocale;
		children: Snippet;
	} = $props();

	const urls = pickUrls(dev);
	const SUMMARY_PROVIDERS = {
		anthropic: { icon: IconClaude, name: 'Anthropic' },
		google: { icon: IconGemini, name: 'Google Gemini' },
		openai: { icon: IconOpenAi, name: 'OpenAI' },
		xai: { icon: IconXai, name: 'xAI' },
	} as const;

	const reads = createReadsQuery(() => slug);
	const readCount = $derived(reads.data?.read_count);

	let summaryOpen = $state(false);
	const summaryProvider = $derived(
		summary ? SUMMARY_PROVIDERS[summary.provider as keyof typeof SUMMARY_PROVIDERS] : undefined,
	);
	const SummaryProviderIcon = $derived(summaryProvider?.icon);
	// One call only -- `$props.id()` may not be used twice in a component -- so the pair is
	// derived from a single stable base.
	const summaryId = $props.id();
	const summaryTrigger = `${summaryId}-trigger`;
	const summaryPanel = `${summaryId}-panel`;

	/**
	 * The card for this article, at a URL nothing had to be told.
	 *
	 * `cms og` writes one card per article under the same path the article has, so the address
	 * follows from the route and no reference is stored anywhere. The cost is that the name is
	 * mutable -- an edited title reuses this URL -- which is why the CDN serves these for a
	 * week rather than a year. See spec/architecture/media.md.
	 */
	const card = $derived(cardUrl(urls.cdn, page.url.pathname, locale.code));

	$effect(() => {
		document.documentElement.lang = locale.languageTag;
	});

	/**
	 * A JSON-LD block, safe to drop into markup.
	 *
	 * Every `<` in the payload becomes `\u003c`, and the closing tag is assembled rather than
	 * written, so no `</script` sequence exists anywhere here. A tokenizer scanning for one does
	 * not care that it sits inside a string, and neither case stays hypothetical once this data
	 * includes text written by something other than us.
	 */
	function ldJson(data: unknown): string {
		const json = JSON.stringify(data).replaceAll('<', String.raw`\u003c`);
		return `<script type="application/ld+json">${json}</${'script'}>`;
	}

	/**
	 * Follow the last letter on the rendered reference line, not its trailing punctuation.
	 *
	 * A wrapped line can stop short by a word or several CJK glyphs, and CSS exposes no value for
	 * that ink width. Pretext applies the browser's line-breaking rules to exact canvas metrics;
	 * the mark stays in flow and measurement only supplies the inset wrapping cannot express.
	 * See spec/styling.md.
	 */
	function alignSummaryProvider(node: HTMLParagraphElement) {
		let frame = 0;
		let prepared: ReturnType<typeof prepareWithSegments> | undefined;
		let preparedText = '';
		let preparedFont = '';
		let preparedLetterSpacing = 0;
		const align = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => {
				const mark = node.querySelector<HTMLElement>('[data-summary-provider]');
				const text = Array.from(node.childNodes)
					.filter((child) => child.nodeType === Node.TEXT_NODE)
					.map((child) => child.textContent ?? '')
					.join('')
					.trim();
				if (!mark || !text) return;

				const style = getComputedStyle(node);
				const parsedLetterSpacing = Number.parseFloat(style.letterSpacing);
				const letterSpacing = Number.isFinite(parsedLetterSpacing) ? parsedLetterSpacing : 0;
				if (
					!prepared ||
					preparedText !== text ||
					preparedFont !== style.font ||
					preparedLetterSpacing !== letterSpacing
				) {
					prepared = prepareWithSegments(text, style.font, { letterSpacing });
					preparedText = text;
					preparedFont = style.font;
					preparedLetterSpacing = letterSpacing;
				}

				const width = node.clientWidth;
				const lineHeight = Number.parseFloat(style.lineHeight);
				const lines = layoutWithLines(prepared, width, lineHeight).lines;
				const last = lines.at(-1);
				if (!last) return;

				const markWidth = mark.getBoundingClientRect().width;
				const startMargin = Number.parseFloat(getComputedStyle(mark).marginInlineStart) || 0;
				const widthThroughLastLetter = (line: (typeof lines)[number]) => {
					const throughLastLetter = line.text.match(/^.*\p{L}\p{M}*/u)?.[0];
					return throughLastLetter
						? measureNaturalWidth(
								prepareWithSegments(throughLastLetter, style.font, { letterSpacing }),
							)
						: line.width;
				};
				const lastWidth = widthThroughLastLetter(last);
				const preceding = lines.at(-2);
				const precedingWidth = preceding ? widthThroughLastLetter(preceding) : undefined;
				const sharesLastLine =
					precedingWidth !== undefined && last.width + startMargin <= precedingWidth - markWidth;
				const anchorWidth = sharesLastLine ? precedingWidth : lastWidth;
				const inset = Math.min(Math.max(0, width - anchorWidth), Math.max(0, width - markWidth));
				mark.style.marginInlineEnd = remFromMeasuredPixels(inset);
			});
		};

		const resize = new ResizeObserver(align);
		const content = new MutationObserver(align);
		resize.observe(node);
		content.observe(node, { childList: true, characterData: true, subtree: true });
		align();
		return {
			destroy() {
				cancelAnimationFrame(frame);
				resize.disconnect();
				content.disconnect();
			},
		};
	}

	/** What this page is, for a reader that parses rather than renders. */
	const article = $derived({
		'@context': 'https://schema.org',
		'@type': 'Article',
		headline: meta.title,
		description: meta.description,
		image: card,
		datePublished: meta.created,
		dateModified: meta.lastmod,
		inLanguage: locale.languageTag,
		mainEntityOfPage: locale.canonical,
		author: { '@type': 'Person', name: site.author.name },
	});

	// Pin UTC so the shown day matches the authored frontmatter date everywhere it
	// renders, mirroring the article list (see card.svelte).
	const date = $derived(shortDate(meta.created));
</script>

<svelte:head>
	<title>{meta.title}: {meta.subtitle}</title>
	<meta name="description" content={meta.description} />

	<meta property="og:type" content="article" />
	<meta property="og:title" content={meta.title} />
	<meta property="og:description" content={meta.description} />
	<meta property="og:url" content={locale.canonical} />
	<meta property="og:locale" content={locale.languageTag} />
	<meta property="og:image" content={card} />
	<!-- Stated because a crawler that reserves the box before fetching draws it right. -->
	<meta property="og:image:width" content={CARD_WIDTH} />
	<meta property="og:image:height" content={CARD_HEIGHT} />
	<meta property="og:image:alt" content={meta.title} />
	<meta property="article:published_time" content={meta.created} />

	<!-- `summary_large_image` is what makes X render the card at full width rather than as a
	     thumbnail beside the text, which is the only shape this layout is drawn for. -->
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={meta.title} />
	<meta name="twitter:description" content={meta.description} />
	<meta name="twitter:image" content={card} />

	<!-- Safe despite the raw insertion: ldJson escapes what it serialises. Stated rather than
	     suppressed; see +layout.svelte and spec/lint-format.md. -->
	{@html ldJson(article)}
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<Toc {toc} />
	<HomeLink locale={locale.code} />
	<div class="article-column px-6 py-24">
		<article>
			<header>
				<h1 class="text-text-strong">{meta.title}</h1>
				<div class="mt-2 flex flex-wrap items-center gap-2 text-sm text-text-soft">
					<time datetime={meta.created}>{date}</time>
					<span
						class="inline-flex items-center gap-1"
						title="{chars} characters"
						aria-label="{chars.toLocaleString('en-US')} characters"
					>
						<Type class="size-3.5" aria-hidden="true" />
						{formatCompact(chars)}
					</span>
					<!-- Absent until the count arrives, rather than held open at a guessed width.
					     The server cannot know this number -- see spec/engagement.md -- and the
					     width it would need is the rendered width of a figure nobody has yet. A
					     returning reader is served the previous count out of the persisted query
					     cache and sees no movement at all. -->
					{#if readCount != null}
						<span
							class="inline-flex items-center gap-1"
							title="{readCount} reads"
							aria-label="{readCount.toLocaleString('en-US')} reads"
						>
							<BookOpenText class="size-3.5" aria-hidden="true" />
							{formatCompact(readCount)}
						</span>
					{/if}
					<!-- A disclosure, not a menu: it is deliberately not dismissed by clicking
					     elsewhere, because a reader comparing the summary against the article is
					     doing exactly that -- clicking elsewhere. Only the trigger closes it. The
					     disabled control keeps this metadata row stable while generated copy is absent. -->
					<button
						type="button"
						id={summaryTrigger}
						disabled={!summary}
						aria-expanded={summary ? summaryOpen : false}
						aria-controls={summary ? summaryPanel : undefined}
						onclick={() => {
							if (summary) summaryOpen = !summaryOpen;
						}}
						class="quiet-control disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent disabled:hover:text-text-soft"
					>
						<span class="focus-link-inner inline-flex items-center gap-1">
							<Sparkles class="size-3.5" aria-hidden="true" />
							<span>{m['article.summary']({}, { locale: locale.code })}</span>
						</span>
					</button>
					<LanguageSwitcher code={locale.code} sourceLanguage={meta.lang} />
				</div>
				{#if locale.code !== 'mw'}
					<TranslationNotice
						code={locale.code}
						sourceLanguage={meta.lang}
						available={locale.translationAvailable}
					/>
				{/if}
				{#if summary}
					<!-- Rows collapse to 0fr rather than the box to height 0, which is the one way to
					     animate to a height nobody measured. See spec/architecture/media.md on motion. -->
					<div class="summary-shell" data-open={summaryOpen}>
						<div class="overflow-hidden">
							<div
								id={summaryPanel}
								role="region"
								aria-labelledby={summaryTrigger}
								class="mt-3 border-l-2 border-border-strong pr-3 pl-3 text-sm leading-relaxed text-text-soft"
							>
								<p use:alignSummaryProvider>
									{summary.text}
									{#if SummaryProviderIcon && summaryProvider}
										<span
											data-summary-provider
											class="float-right mt-0.75 ml-2 block h-4"
											aria-label={summaryProvider.name}
											title={summaryProvider.name}
										>
											<SummaryProviderIcon class="h-4 w-auto" aria-hidden="true" />
										</span>
									{/if}
								</p>
							</div>
						</div>
					</div>
				{/if}
			</header>

			<div class="article-body mt-8 leading-relaxed">
				{@render children()}
			</div>
		</article>

		<Newsletter locale={locale.code} class="border-t border-border pt-12" />
	</div>
</main>

<style>
	/* Animating to `height: auto` is not possible, so the grid row is animated instead: 0fr to
	   1fr resolves against the content's own height without anyone measuring it. The child
	   needs `overflow: hidden` for the clip to happen. */
	.summary-shell {
		display: grid;
		grid-template-rows: 0fr;
		transition: grid-template-rows 320ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	.summary-shell[data-open='true'] {
		grid-template-rows: 1fr;
	}

	@media (prefers-reduced-motion: reduce) {
		.summary-shell {
			transition: none;
		}
	}

	.article-body {
		font-size: 0.9375rem;
	}

	.article-body :global(strong) {
		font-weight: 500;
		color: var(--color-text-strong);
	}

	.article-body :global(s) {
		color: var(--color-text-soft);
	}

	/* A quoted source is a prose inset, not an authored callout. See spec/styling.md. */
	.article-body :global(blockquote) {
		padding: 1rem 1.125rem 1rem 1.375rem;
		border-left: 0.125rem solid var(--color-border-strong);
		border-radius: 0 0.625rem 0.625rem 0;
		background: var(--color-paper-hover);
		color: var(--color-text-strong);
	}

	.article-body :global(blockquote > p + p) {
		margin-top: 0.75rem;
	}

	.article-body :global(hr) {
		width: 18.75%;
		height: 0.125rem;
		margin: 2.5rem auto;
		border: 0;
		background: linear-gradient(
			to right,
			var(--color-border-strong) 0 12%,
			transparent 12% 22%,
			var(--color-border-strong) 22% 34%,
			transparent 34% 44%,
			var(--color-border-strong) 44% 56%,
			transparent 56% 66%,
			var(--color-border-strong) 66% 78%,
			transparent 78% 88%,
			var(--color-border-strong) 88% 100%
		);
	}

	.article-body :global(code:not(pre code)) {
		box-shadow: inset 0 0 0 0.0625rem var(--color-border-strong);
		border-radius: 0.375rem;
		background: var(--color-paper);
		padding: 0.125rem 0.375rem;
		font-size: 0.875rem;
	}
</style>
