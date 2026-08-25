<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { animate } from 'motion';
	import { onDestroy, untrack } from 'svelte';
	import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';

	type Props = {
		label?: string;
		title?: string;
		collapsible?: boolean;
		defaultExpanded?: boolean;
		code?: string;
		html?: string;
	};
	let { label, title, collapsible, defaultExpanded = true, code, html }: Props = $props();

	type AnimationControl = { stop: () => void };
	type CollapsePhase = 'collapsed' | 'collapsing' | 'expanded' | 'expanding';

	const COLLAPSE_SPRING = {
		type: 'spring' as const,
		stiffness: 420,
		damping: 38,
		mass: 0.9,
	};
	const instanceId = $props.id();
	const panelId = `${instanceId}-panel`;
	// This prop is an initial state, not a command that reopens a disclosure after interaction.
	const initiallyExpanded = untrack(() => !title || collapsible === false || defaultExpanded);
	let expanded = $state(initiallyExpanded);
	let phase = $state<CollapsePhase>(initiallyExpanded ? 'expanded' : 'collapsed');
	let collapseEl = $state<HTMLElement>();
	let collapseMotion: AnimationControl | undefined;
	const canCollapse = $derived(Boolean(title) && (collapsible ?? true));
	const panelHidden = $derived(canCollapse && !expanded);
	const dividerVisible = $derived(!canCollapse || phase !== 'collapsed');

	function settle(nextExpanded: boolean) {
		if (collapseEl) collapseEl.style.height = nextExpanded ? 'auto' : '0rem';
		phase = nextExpanded ? 'expanded' : 'collapsed';
		collapseMotion = undefined;
	}

	function setExpanded(nextExpanded: boolean) {
		if (!canCollapse || nextExpanded === expanded || !collapseEl) return;

		const currentHeight = collapseEl.getBoundingClientRect().height;
		collapseMotion?.stop();
		collapseMotion = undefined;
		collapseEl.style.height = remFromMeasuredPixels(currentHeight);
		expanded = nextExpanded;
		phase = nextExpanded ? 'expanding' : 'collapsing';

		const targetHeight = nextExpanded ? collapseEl.scrollHeight : 0;
		if (
			window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
			Math.abs(currentHeight - targetHeight) < 0.5
		) {
			settle(nextExpanded);
			return;
		}

		const rootPixels =
			Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
			DEFAULT_PIXELS_PER_REM;
		let control: AnimationControl;
		control = animate(currentHeight, targetHeight, {
			...COLLAPSE_SPRING,
			onUpdate: (height) => {
				collapseEl?.style.setProperty(
					'height',
					remFromMeasuredPixels(Math.max(0, height), rootPixels),
				);
			},
			onComplete: () => {
				if (collapseMotion !== control) return;
				settle(nextExpanded);
			},
		});
		collapseMotion = control;
	}

	onDestroy(() => collapseMotion?.stop());
</script>

{#snippet codeLabel()}
	{#if label}
		<span
			class="pointer-events-none absolute top-3 right-3 z-10 text-xs tracking-wider text-text-soft select-none"
		>
			{label}
		</span>
	{/if}
{/snippet}

{#snippet source()}
	{#if html}
		{@html html}
	{:else if code}
		<pre><code>{code}</code></pre>
	{/if}
{/snippet}

<div class="codeblock relative">
	{#if title}
		<div
			class="code-frame focus-ring-within overflow-hidden rounded-xl border border-border bg-paper"
		>
			{#if canCollapse}
				<button
					type="button"
					class="code-title flex w-full cursor-pointer items-center justify-between gap-3 border-border bg-paper-hover px-4 py-2.5 text-left text-sm font-medium text-text transition-colors duration-150 hover:text-text-strong"
					class:border-b={dividerVisible}
					aria-expanded={expanded}
					aria-controls={panelId}
					onclick={() => setExpanded(!expanded)}
				>
					<span>{title}</span>
					<span
						class="code-chevron shrink-0 text-text-soft transition-transform duration-200"
						class:rotate-180={expanded}
					>
						<ChevronDown class="size-3.5" aria-hidden="true" />
					</span>
				</button>
			{:else}
				<div
					class="border-b border-border bg-paper-hover px-4 py-2.5 text-sm font-medium text-text"
				>
					{title}
				</div>
			{/if}

			<div
				bind:this={collapseEl}
				class="code-collapse"
				data-phase={canCollapse ? phase : 'expanded'}
			>
				<div id={panelId} class="code-panel relative" aria-hidden={panelHidden} inert={panelHidden}>
					{@render codeLabel()}
					<div class="code-scroll overflow-x-auto bg-paper p-4 pr-16 text-sm leading-snug">
						{@render source()}
					</div>
				</div>
			</div>
		</div>
	{:else}
		<div class="code-panel relative">
			{@render codeLabel()}
			<!-- Shiki tags its <pre> with tabindex=0 so keyboard users can focus and scroll the
			code, so focus lands on the inner code area, not this box. The ring is redirected
			out to this bordered box via :has (see <style>) so it wraps the whole code block. -->
			<div
				class="code-scroll focus-ring-within overflow-x-auto rounded-xl border border-border bg-paper p-4 pr-16 text-sm leading-snug"
			>
				{@render source()}
			</div>
		</div>
	{/if}
</div>

<style>
	.codeblock :global(pre) {
		background: transparent;
		margin: 0;
		padding: 0;
	}

	/* Shiki's <pre> takes focus, while the shared within utility draws on this box. */
	.codeblock :global(pre:focus-visible),
	.code-title:focus-visible {
		outline: none;
	}

	.code-collapse {
		overflow: hidden;
	}

	.code-collapse[data-phase='collapsed'] {
		height: 0;
	}

	.code-collapse[data-phase='collapsing'],
	.code-collapse[data-phase='expanding'] {
		will-change: height;
	}

	.codeblock :global(.shiki),
	.codeblock :global(.shiki span) {
		color: var(--shiki-light);
	}

	:global(.dark) .codeblock :global(.shiki),
	:global(.dark) .codeblock :global(.shiki span),
	:global([data-theme='dark']) .codeblock :global(.shiki),
	:global([data-theme='dark']) .codeblock :global(.shiki span) {
		color: var(--shiki-dark);
	}

	@media (prefers-reduced-motion: reduce) {
		.code-chevron {
			transition: none;
		}
	}
</style>
