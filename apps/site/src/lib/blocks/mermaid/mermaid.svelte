<script lang="ts">
	import './palette.css';
	import { renderMermaid } from './mermaid';

	let { source, ratio, loadingLabel }: { source: string; ratio?: number; loadingLabel: string } =
		$props();
	let root = $state<HTMLElement>();
	let svg = $state('');
	let failed = $state(false);

	$effect(() => {
		const host = root;
		const definition = source;
		if (!host) return;

		let current = true;
		svg = '';
		failed = false;
		void renderMermaid(definition, host).then(
			(result) => {
				if (current) svg = result.svg;
			},
			(error: unknown) => {
				if (!current) return;
				failed = true;
				console.error('Could not render Mermaid diagram', error);
			},
		);

		return () => {
			current = false;
		};
	});
</script>

<div
	bind:this={root}
	class="mermaid-block overflow-hidden rounded-xl border border-border bg-paper"
>
	<div
		class="mermaid-stage focus-ring-within relative overflow-x-auto rounded-xl p-5"
		aria-busy={!svg && !failed}
	>
		{#if svg}
			<!-- Mermaid sanitises tracked diagram source in strict mode before returning this SVG.
			     Stated rather than suppressed; see spec/lint-format.md. -->
			<div class="mermaid-result">{@html svg}</div>
		{:else if failed}
			<!-- svelte-ignore a11y_no_noninteractive_tabindex (the source fallback can overflow
			     horizontally and therefore needs to be reachable by a keyboard) -->
			<pre tabindex="0"><code>{source}</code></pre>
		{:else}
			<div
				class="mermaid-loading"
				class:mermaid-intrinsic={ratio !== undefined}
				style:aspect-ratio={ratio}
				role="status"
			>
				<div class="mermaid-placeholder" aria-hidden="true">
					<span class="mermaid-path"></span>
					<span class="mermaid-node mermaid-node-start"></span>
					<span class="mermaid-node mermaid-node-end"></span>
				</div>
				<span class="mermaid-loading-label">{loadingLabel}</span>
			</div>
		{/if}
	</div>
</div>

<style>
	.mermaid-stage {
		display: grid;
		min-block-size: 13rem;
		align-items: center;
	}

	.mermaid-placeholder {
		position: absolute;
		inset: 1.25rem;
		filter: blur(0.3rem);
		opacity: 0.48;
		animation: mermaid-breathe 1.6s ease-in-out infinite alternate;
	}

	.mermaid-loading.mermaid-intrinsic {
		position: relative;
		min-inline-size: 30rem;
	}

	.mermaid-loading-label {
		position: absolute;
		z-index: 1;
		display: grid;
		inset: 0;
		place-items: center;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		line-height: 1;
		color: var(--color-text-soft);
	}

	.mermaid-node {
		position: absolute;
		top: 50%;
		width: 5.5rem;
		height: 2.75rem;
		translate: 0 -50%;
		border: 0.0625rem solid var(--color-border-strong);
		border-radius: 0.5rem;
		background: var(--color-paper);
	}

	.mermaid-node-start {
		left: 12%;
	}

	.mermaid-node-end {
		right: 12%;
	}

	.mermaid-path {
		position: absolute;
		top: 50%;
		left: calc(12% + 5.5rem);
		right: calc(12% + 5.5rem);
		border-block-start: 0.125rem solid var(--color-border-strong);
	}

	.mermaid-result {
		min-inline-size: 30rem;
		animation: mermaid-reveal 260ms var(--ease-spring) both;
	}

	.mermaid-result :global(svg) {
		display: block;
		max-inline-size: 100%;
		height: auto;
		margin-inline: auto;
	}

	pre {
		min-inline-size: max-content;
		margin: 0;
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.4;
		color: var(--color-text-soft);
	}

	pre:focus-visible {
		outline: none;
	}

	@keyframes mermaid-breathe {
		to {
			opacity: 0.72;
		}
	}

	@keyframes mermaid-reveal {
		from {
			opacity: 0;
			filter: blur(0.25rem);
			transform: translateY(0.25rem);
		}
		to {
			opacity: 1;
			filter: blur(0);
			transform: translateY(0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.mermaid-placeholder,
		.mermaid-result {
			animation: none;
		}
	}
</style>
