<script lang="ts">
	import { URLS } from '@canmi/urls';
	import FileText from '@lucide/svelte/icons/file-text';
	import Scale from '@lucide/svelte/icons/scale';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import { PUBLIC_LANGUAGE } from '$lib/locale';
	import { spaceScriptBoundaries } from '$lib/locale/spacing';
	import * as m from '$lib/paraglide/messages';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const locale = $derived(data.locale.code);
	const numberLocale = $derived(locale === 'mw' ? 'en-US' : PUBLIC_LANGUAGE[locale]);
	const count = $derived(new Intl.NumberFormat(numberLocale).format(data.total));

	/** What each registry is called where readers know it, and where it lives. */
	const REGISTRY_NAMES: Record<string, string> = { npm: 'npm', cargo: 'crates.io' };

	/**
	 * The registries, written as a list in the reader's language with each one a link.
	 *
	 * `Intl.ListFormat` supplies the connectives -- "npm and crates.io", "npm と crates.io" --
	 * which is the one part of this sentence that is neither a proper noun nor a number, and the
	 * one part no message can carry because the list length is data. `formatToParts` keeps the
	 * names separable from the words between them, so each stays a link.
	 *
	 * A registry with no entry above is still listed, under the identifier the record uses and
	 * without a link. Dropping it would understate where the code came from.
	 */
	const registryParts = $derived.by(() => {
		const parts = new Intl.ListFormat(numberLocale, {
			style: 'long',
			type: 'conjunction',
		}).formatToParts(data.registries.map((id) => REGISTRY_NAMES[id] ?? id));
		// Chinese joins with a bare `和`, which leaves `crates.io和npm` reading as one run.
		// Authored copy carries these spaces already; text assembled here has nobody to type
		// them. See $lib/locale/spacing.
		// Carried as a flag rather than folded into the value, because the space belongs outside
		// the anchor -- inside it, the link's underline would be drawn under the gap too.
		const spaced = spaceScriptBoundaries(parts.map((part) => part.value));
		return parts.map((part, index) => ({
			type: part.type,
			value: part.value,
			gapBefore: (spaced[index] ?? '').length > part.value.length,
		}));
	});
	const registryUrl = $derived((name: string) => {
		const id = data.registries.find((candidate) => (REGISTRY_NAMES[candidate] ?? candidate) === name);
		return id ? URLS.external.registries[id as keyof typeof URLS.external.registries] : undefined;
	});

	/**
	 * Scroll to a section without writing its id into the address bar.
	 *
	 * The contents list is a way around one long page, not a set of addresses worth collecting
	 * in history: a reader walking six licences would otherwise leave six entries behind and
	 * have to press Back six times to get out. Arriving *with* a hash still works -- the
	 * browser jumps natively on load, and restores the reader's own position on a reload rather
	 * than jumping again -- so nothing is lost by not adding one here.
	 *
	 * The element stays an `<a href="#...">`, so without JavaScript the native jump happens
	 * instead, hash and all. See spec/styling.md.
	 */
	function jumpToSection(event: MouseEvent, anchor: string) {
		// A modified click is the reader asking for a new tab or window, which is the browser's
		// to answer.
		if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button !== 0) {
			return;
		}
		const target = document.getElementById(anchor);
		if (!target) return;
		event.preventDefault();
		const still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		target.scrollIntoView({ behavior: still ? 'instant' : 'smooth', block: 'start' });
	}
</script>

<svelte:head>
	<title>{m['licenses.title']({}, { locale })}</title>
	<meta name="description" content={m['licenses.description']({}, { locale })} />
	<!-- A credits list has nothing to rank for and every row of it belongs to somebody else. -->
	<meta name="robots" content="noindex, follow" />
</svelte:head>

<main class="min-h-screen bg-page text-text">
	<article class="mx-auto max-w-180 px-6 py-24">
		<header>
			<h1 class="text-text-strong">{m['licenses.title']({}, { locale })}</h1>
			<!-- The thanks come before the accounting. What follows is a list of other people's
			     work, and leading with the count would make the page read as an inventory of
			     assets rather than as credit. -->
			<div class="mt-4 space-y-4 leading-relaxed text-pretty">
				<p>{m['licenses.built']({}, { locale })}</p>
				<p>{m['licenses.thanks']({}, { locale })}</p>
				<p class="text-text-soft">
					<ParaglideMessage message={m['licenses.below']} inputs={{}} options={{ locale }}>
						<!-- The project's own terms, pointing at the canonical text rather than at a
						     copy of it. SPDX because that is the address every licence on this page
						     has, this one included. -->
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

		<!-- The census sits after the thanks rather than before them: the number is what the list
		     amounts to, not what the page is about. Plain figures, not the boxed cells the
		     newsletter uses -- those mark a number somebody just changed, and this one is a
		     standing fact about the tree. -->
		<p class="mt-8 text-pretty text-text-soft">
			<ParaglideMessage
				message={m['licenses.census']}
				inputs={{ count, licenses: data.groups.length }}
				options={{ locale }}
			>
				{#snippet registries()}{#each registryParts as part, i (i)}{#if part.gapBefore}{' '}{/if}{#if part.type === 'element'}<a
								href={registryUrl(part.value)}
								target="_blank"
								rel="noopener noreferrer"
								class="focus-link spring-underline article-link text-text">{part.value}</a
							>{:else}{part.value}{/if}{/each}{/snippet}
			</ParaglideMessage>
		</p>

		<!-- The same three documents the plain-text routes serve. A page is easier to read and a
		     text file is easier to keep, so both exist and neither is the fallback. -->
		<nav aria-label={m['licenses.plaintext']({}, { locale })} class="mt-4 flex flex-wrap gap-4">
			<!-- Named for what each one holds -- one line per package, against every licence text
			     in full -- rather than for the file it happens to be. The section below is already
			     called Index, so neither reuses that word. -->
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
		</nav>

		<!-- Contents. Every licence in the tree with what it covers, which is both the summary
		     worth having and the way into a page this long. -->
		<nav aria-labelledby="licenses-contents" class="mt-16">
			<h2 id="licenses-contents" class="mb-3 font-medium text-text-strong">
				{m['licenses.contents']({}, { locale })}
			</h2>
			{#each data.groups as group (group.anchor)}
				<a
					href="#{group.anchor}"
					onclick={(event) => jumpToSection(event, group.anchor)}
					class="focus-ring-within -mx-2 flex items-center gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
				>
					<span class="focus-link-inner min-w-0 truncate">{group.license}</span>
					<span class="h-0 flex-1 border-t border-dashed border-border-strong"></span>
					<span class="shrink-0 font-mono text-[0.9375rem] tabular-nums text-text-soft"
						>{group.entries.length}</span
					>
				</a>
			{/each}
			<!-- A footnote rather than a sentence under the heading: what it explains is an oddity
			     in the column of numbers above -- they add up to more than the number of packages
			     -- so it belongs after them, at the weight of an aside. -->
			<p class="mt-3 text-[0.8125rem] text-pretty text-text-soft">
				<span aria-hidden="true">*&nbsp;</span>{m['licenses.multiple']({}, { locale })}
			</p>
		</nav>

		{#each data.groups as group (group.anchor)}
			<section aria-labelledby={group.anchor} class="mt-16">
				<!-- `jump-target` holds a share of the viewport above the heading, so a section
				     jumped to starts in the band people read from instead of against the top edge.
				     See styles/utilities.css and spec/styling.md. -->
				<h2 id={group.anchor} class="jump-target mb-3 font-medium text-text-strong">

					{group.license}
				</h2>
				{#each group.entries as entry (entry.href)}
					<a
						href={entry.href}
						data-sveltekit-reload
						class="focus-ring-within -mx-2 flex items-baseline gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
					>
						<span class="focus-link-inner shrink-0 truncate text-text-strong">{entry.name}</span>
						<span class="shrink-0 font-mono text-[0.8125rem] text-text-soft">{entry.version}</span>
						<!-- The package declared nothing and the licence above was read off what it
						     ships. Marked on the row rather than in the heading, because it is a fact
						     about how this is known and not about which terms apply. -->
						{#if entry.asserted}
							<span
								class="shrink-0 rounded-[0.25rem] border border-border px-1 text-[0.75rem] text-text-soft"
								>{m['licenses.asserted']({}, { locale })}</span
							>
						{/if}
						<!-- Only when the heading is part of a longer expression. A package filed
						     under MIT because it offers `MIT OR Apache-2.0` must say so on the row,
						     or the grouping would read as a claim that plain MIT is the whole of it.
						     A package whose terms are exactly the heading repeats nothing. -->
						{#if entry.spdx !== group.license}
							<span class="hidden shrink-0 text-[0.8125rem] text-text-soft sm:inline"
								>{entry.spdx}</span
							>
						{/if}
						<!-- `min-w` so a long expression beside a long author list cannot squeeze the
						     leader out of existence; it is what ties the two ends of the row together
						     and a row that loses it stops matching the ones above. -->
						<span
							class="h-0 min-w-6 flex-1 self-center border-t border-dashed border-border-strong"
						></span>
						<!-- Blank when the package named nobody. Two registries carry an author field
						     that a third of packages leave empty, and inventing attribution is worse
						     than leaving the space. -->
						<span class="min-w-0 shrink truncate text-right text-[0.9375rem] text-text-soft"
							>{entry.authors}</span
						>
					</a>
				{/each}
			</section>
		{/each}
	</article>
</main>
