<script lang="ts">
	import CodeBlock from '$lib/codeblock.svelte';
	import Image from '$lib/image.svelte';
	import LinkCard from '$lib/linkcard.svelte';
	import { MESSAGES } from '$lib/messages';
	import Placeholder from '$lib/placeholder.svelte';
	import Section from '$lib/section.svelte';
	import SvgCanvas from '$lib/svg-canvas.svelte';
	import Tokei from '$lib/tokei.svelte';
	import Cargo from '$lib/cargo.svelte';
	import GitHub from '$lib/github.svelte';
	import PopoverContent from '$lib/ui/popover-content.svelte';
	import Info from '@lucide/svelte/icons/info';
	import X from '@lucide/svelte/icons/x';
	import { Popover } from 'bits-ui';
	import type { Block } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';

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
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html block.html}
		{:else if block.type === 'heading'}
			<Section slug={block.slug}>{block.text}</Section>
		{:else if block.type === 'code'}
			<CodeBlock lang={block.lang} html={block.html} />
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
		{:else if block.type === 'placeholder'}
			<Placeholder kind={block.kind} meta={block.meta} />
		{:else if block.type === 'svgCanvas'}
			<SvgCanvas svg={block.svg} />
		{:else if block.type === 'tokei'}
			<Tokei source={block.source} title={block.title} view={block.view} />
		{:else if block.type === 'cargo'}
			<Cargo crate={block.crate} view={block.view} />
		{:else if block.type === 'github'}
			<GitHub
				repo={block.repo}
				gitRef={block.gitRef}
				title={block.title}
				align={block.align}
			/>
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
				>{MESSAGES[locale].translatorNote}</span
			>
			<button
				type="button"
				onclick={() => closeNote(true)}
				class="-m-1 cursor-pointer rounded-sm p-1 text-text-soft hover:bg-paper-hover hover:text-text-strong"
				aria-label={MESSAGES[locale].closeTranslatorNote}
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
		line-height: inherit;
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

	:global(.tn-trigger:focus-visible) {
		border-radius: 0.125rem;
		outline: 0.125rem solid var(--color-border-strong);
		outline-offset: 0.125rem;
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
