<script lang="ts">
	import CodeBlock from '$lib/codeblock.svelte';
	import Image from '$lib/image.svelte';
	import LinkCard from '$lib/linkcard.svelte';
	import Placeholder from '$lib/placeholder.svelte';
	import Section from '$lib/section.svelte';
	import SvgCanvas from '$lib/svg-canvas.svelte';
	import X from '@lucide/svelte/icons/x';
	import { tick } from 'svelte';
	import { fade } from 'svelte/transition';
	import type { Block } from '$lib/content/types';

	let { blocks }: { blocks: Block[] } = $props();
	let root = $state<HTMLElement>();
	let panel = $state<HTMLElement>();
	let trigger: HTMLButtonElement | undefined;
	let note = $state('');
	let top = $state(0);
	let left = $state(0);

	function noteTrigger(target: EventTarget | null): HTMLButtonElement | undefined {
		if (!(target instanceof Element)) return undefined;
		const found = target.closest<HTMLButtonElement>('button[data-tn-note]');
		return found && root?.contains(found) ? found : undefined;
	}

	function positionNote() {
		if (!trigger || !panel) return;
		const anchor = trigger.getBoundingClientRect();
		const box = panel.getBoundingClientRect();
		const gutter = 12;
		const below = anchor.bottom + 10;
		const above = anchor.top - box.height - 10;
		top = below + box.height <= window.innerHeight - gutter || above < gutter ? below : above;
		left = Math.min(
			Math.max(anchor.left + anchor.width / 2 - box.width / 2, gutter),
			window.innerWidth - box.width - gutter,
		);
	}

	function closeNote(restoreFocus: boolean) {
		if (!trigger) return;
		trigger.setAttribute('aria-expanded', 'false');
		trigger.removeAttribute('aria-describedby');
		const previous = trigger;
		trigger = undefined;
		note = '';
		if (restoreFocus) previous.focus();
	}

	async function openNote(next: HTMLButtonElement) {
		if (trigger === next) {
			closeNote(true);
			return;
		}
		closeNote(false);
		trigger = next;
		note = next.dataset.tnNote ?? '';
		next.setAttribute('aria-expanded', 'true');
		next.setAttribute('aria-describedby', 'translator-note-description');
		await tick();
		positionNote();
	}

	function handleClick(event: MouseEvent) {
		const next = noteTrigger(event.target);
		if (next) void openNote(next);
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

	function fadeMs(): number {
		return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 0 : 120;
	}

	$effect(() => {
		if (!note) return;
		const pointerDown = (event: PointerEvent) => {
			if (!(event.target instanceof Node)) return;
			if (!panel?.contains(event.target) && !trigger?.contains(event.target)) closeNote(false);
		};
		const keyDown = (event: KeyboardEvent) => {
			if (event.key === 'Escape') closeNote(true);
		};
		document.addEventListener('pointerdown', pointerDown, true);
		document.addEventListener('keydown', keyDown);
		window.addEventListener('resize', positionNote);
		window.addEventListener('scroll', positionNote, true);
		return () => {
			document.removeEventListener('pointerdown', pointerDown, true);
			document.removeEventListener('keydown', keyDown);
			window.removeEventListener('resize', positionNote);
			window.removeEventListener('scroll', positionNote, true);
		};
	});
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
		{/if}
	{/each}
</div>

{#if note}
	<aside
		bind:this={panel}
		id="translator-note"
		role="note"
		transition:fade={{ duration: fadeMs() }}
		style="--tn-top: {top}px; --tn-left: {left}px"
		class="tn-popover fixed z-40 rounded-lg border border-border bg-paper p-3 text-sm leading-relaxed text-text shadow-lg"
	>
		<div class="mb-1 flex items-center justify-between gap-4">
			<span class="text-xs font-medium tracking-wide text-text-soft uppercase">TN</span>
			<button
				type="button"
				onclick={() => closeNote(true)}
				class="-m-1 cursor-pointer rounded-sm p-1 text-text-soft hover:bg-paper-hover hover:text-text-strong"
				aria-label="Close translator's note"
			>
				<X class="size-3.5" aria-hidden="true" />
			</button>
		</div>
		<p id="translator-note-description">{note}</p>
	</aside>
{/if}

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

	.tn-popover {
		top: var(--tn-top);
		left: var(--tn-left);
		width: min(26rem, calc(100vw - 1.5rem));
	}

	@media (max-width: 40rem) {
		.tn-popover {
			top: auto;
			right: 0.75rem;
			bottom: 0.75rem;
			left: 0.75rem;
			width: auto;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.tn-popover {
			transition: none;
		}
	}
</style>
