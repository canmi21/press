<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { untrack } from 'svelte';

	type Props = {
		label?: string;
		title?: string;
		collapsible?: boolean;
		defaultExpanded?: boolean;
		code?: string;
		html?: string;
	};
	let { label, title, collapsible, defaultExpanded = true, code, html }: Props = $props();

	const instanceId = $props.id();
	const panelId = `${instanceId}-panel`;
	// This prop is an initial state, not a command that reopens a disclosure after interaction.
	let expanded = $state(untrack(() => defaultExpanded));
	const canCollapse = $derived(Boolean(title) && (collapsible ?? true));
	const shown = $derived(!canCollapse || expanded);
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
					class="code-title flex w-full cursor-pointer items-center justify-between gap-3 bg-paper-hover px-4 py-2.5 text-left text-sm font-medium text-text transition-colors duration-150 hover:text-text-strong"
					class:border-b={shown}
					class:border-border={shown}
					aria-expanded={expanded}
					aria-controls={panelId}
					onclick={() => (expanded = !expanded)}
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

			<div class="code-collapse" data-open={shown}>
				<div class="min-h-0 overflow-hidden">
					<div id={panelId} class="code-panel relative" aria-hidden={!shown} inert={!shown}>
						{@render codeLabel()}
						<div class="code-scroll overflow-x-auto bg-paper p-4 pr-16 text-sm leading-snug">
							{@render source()}
						</div>
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
		display: grid;
		grid-template-rows: 0fr;
		transition: grid-template-rows 300ms var(--ease-spring);
	}

	.code-collapse[data-open='true'] {
		grid-template-rows: 1fr;
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
		.code-collapse,
		.code-chevron {
			transition: none;
		}
	}
</style>
