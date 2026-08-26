<script lang="ts">
	import Cargo from '$lib/blocks/cargo/cargo.svelte';
	import CodeBlock from '$lib/blocks/code-block.svelte';
	import GitHub from '$lib/blocks/github.svelte';
	import Image from '$lib/blocks/image.svelte';
	import ArticleCard from '$lib/blocks/article-card.svelte';
	import LinkCard from '$lib/blocks/link-card.svelte';
	import Mermaid from '$lib/blocks/mermaid/mermaid.svelte';
	import Placeholder from '$lib/blocks/placeholder.svelte';
	import Quadrant from '$lib/blocks/quadrant.svelte';
	import SvgCanvas from '$lib/blocks/svg-canvas.svelte';
	import Tokei from '$lib/blocks/tokei/tokei.svelte';
	import Twitter from '$lib/blocks/twitter.svelte';
	import PopoverContent from '$lib/components/popover-content.svelte';
	import * as m from '$lib/paraglide/messages';
	import Info from '@lucide/svelte/icons/info';
	import X from '@lucide/svelte/icons/x';
	import { Popover } from 'bits-ui';
	import type { Block } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';
	import Section from './section.svelte';

	let { blocks, locale }: { blocks: Block[]; locale: LocaleCode } = $props();
	let root = $state<HTMLElement>();
	let trigger = $state<HTMLButtonElement>();
	let note = $state('');
	let open = $state(false);
	let restoreFocusOnDismiss = false;

	function noteTrigger(target: EventTarget | null): HTMLButtonElement | undefined {
		if (!(target instanceof Element)) return undefined;
		const found = target.closest<HTMLButtonElement>('button[data-tn-note]');
		return found && root?.contains(found) ? found : undefined;
	}

	function closeNote(restoreFocus: boolean) {
		if (!trigger) return;
		trigger.setAttribute('aria-expanded', 'false');
		trigger.removeAttribute('aria-describedby');
		open = false;
		if (restoreFocus) trigger.focus();
	}

	function openNote(next: HTMLButtonElement) {
		if (trigger === next && open) {
			closeNote(true);
			return;
		}
		closeNote(false);
		trigger = next;
		note = next.dataset.tnNote ?? '';
		next.setAttribute('aria-expanded', 'true');
		next.setAttribute('aria-describedby', 'translator-note-description');
		open = true;
	}

	function handleClick(event: MouseEvent) {
		const next = noteTrigger(event.target);
		if (next) openNote(next);
	}

	function noteEvents(node: HTMLElement) {
		root = node;
		node.addEventListener('click', handleClick);
		return {
			destroy() {
				node.removeEventListener('click', handleClick);
				if (root === node) root = undefined;
			},
		};
	}

	function handleOpenChange(next: boolean) {
		if (next) {
			open = true;
			return;
		}
		closeNote(restoreFocusOnDismiss);
		restoreFocusOnDismiss = false;
	}

	function finishOpenChange(next: boolean) {
		if (next) return;
		trigger = undefined;
		note = '';
	}
</script>

<div use:noteEvents class="article-content space-y-4">
	{#each blocks as block, i (i)}
		{#if block.type === 'prose'}
			<!-- Compiled at build time from the tracked corpus, not reader input. Stated rather
			     than suppressed; see spec/lint-format.md. -->
			{@html block.html}
		{:else if block.type === 'heading'}
			<Section slug={block.slug}>{block.text}</Section>
		{:else if block.type === 'code'}
			<CodeBlock
				label={block.label}
				title={block.title}
				collapsible={block.collapsible}
				defaultExpanded={block.defaultExpanded}
				copyLabel={m['code.copy']({}, { locale })}
				copiedLabel={m['code.copied']({}, { locale })}
				copyFailedLabel={m['code.copy-failed']({}, { locale })}
				code={block.code}
				html={block.html}
			/>
		{:else if block.type === 'mermaid'}
			<Mermaid
				source={block.source}
				ratio={block.ratio}
				loadingLabel={m['mermaid.loading']({}, { locale })}
			/>
		{:else if block.type === 'quadrant'}
			<Quadrant
				title={block.title}
				description={block.description}
				axes={block.axes}
				items={block.items}
			/>
		{:else if block.type === 'image'}
			<Image
				src={block.src}
				alt={block.alt}
				width={block.width}
				height={block.height}
				preview={block.preview}
				srcset={block.srcset}
				crop={block.crop}
				align={block.align}
			/>
		{:else if block.type === 'linkcard'}
			<LinkCard
				{locale}
				src={block.src}
				url={block.url}
				title={block.title}
				tone={block.tone}
				width={block.width}
				height={block.height}
				preview={block.preview}
				srcset={block.srcset}
				description={block.description}
			/>
		{:else if block.type === 'article'}
			<ArticleCard
				path={block.path}
				title={block.title}
				subtitle={block.subtitle}
				description={block.description}
				created={block.created}
				chars={block.chars}
			/>
		{:else if block.type === 'placeholder'}
			<Placeholder kind={block.kind} meta={block.meta} />
		{:else if block.type === 'svgCanvas'}
			<SvgCanvas svg={block.svg} />
		{:else if block.type === 'tokei'}
			<Tokei source={block.source} title={block.title} view={block.view} />
		{:else if block.type === 'cargo'}
			<Cargo crate={block.crate} view={block.view} />
		{:else if block.type === 'twitter'}
			<Twitter tweet={block.tweet} />
		{:else if block.type === 'github'}
			<GitHub repo={block.repo} gitRef={block.gitRef} title={block.title} align={block.align} />
		{/if}
	{/each}
</div>

<Popover.Root {open} onOpenChange={handleOpenChange} onOpenChangeComplete={finishOpenChange}>
	<PopoverContent
		anchor={trigger ?? null}
		id="translator-note"
		labelledby="translator-note-label"
		describedby="translator-note-description"
		onEscapeKeydown={() => (restoreFocusOnDismiss = true)}
		onInteractOutside={() => (restoreFocusOnDismiss = false)}
		onOpenAutoFocus={(event) => event.preventDefault()}
		onCloseAutoFocus={(event) => event.preventDefault()}
	>
		<div class="flex items-center gap-2 px-2 py-1 text-text-soft">
			<Info class="size-3.5 shrink-0" aria-hidden="true" />
			<span id="translator-note-label" class="flex-1 text-xs font-medium"
				>{m['article.translator-note']({}, { locale })}</span
			>
			<button
				type="button"
				onclick={() => closeNote(true)}
				class="focus-ring -m-1 cursor-pointer rounded-sm p-1 text-text-soft hover:bg-paper-hover hover:text-text-strong focus-visible:text-text-strong"
				aria-label={m['article.translator-note.close']({}, { locale })}
			>
				<X class="size-3.5" aria-hidden="true" />
			</button>
		</div>
		<p id="translator-note-description" class="border-t border-border px-3 py-2">{note}</p>
	</PopoverContent>
</Popover.Root>

<style>
	:global(.tn-trigger) {
		margin: 0;
		border: 0;
		background: transparent;
		padding: 0;
		color: inherit;
		font: inherit;
		line-height: var(--focus-link-height);
		text-decoration-line: underline;
		text-decoration-style: dotted;
		text-decoration-color: var(--color-border-strong);
		text-underline-offset: 0.25rem;
		cursor: help;
	}

	:global(.tn-trigger:hover),
	:global(.tn-trigger:focus-visible),
	:global(.tn-trigger[aria-expanded='true']) {
		color: var(--color-text-strong);
		text-decoration-color: currentColor;
	}

	:global(.tn-trigger .tn-icon) {
		display: inline;
		width: 0.78em;
		height: 0.78em;
		margin-inline-start: 0.22em;
		margin-inline-end: 0.12em;
		vertical-align: 0.08em;
		color: var(--color-text-soft);
		text-decoration: none;
	}
</style>
