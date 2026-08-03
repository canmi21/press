<script lang="ts">
	import { Hash } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let { slug, children }: { slug: string; children: Snippet } = $props();

	function copyHash(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		history.replaceState(null, '', `#${slug}`);
	}
</script>

<h2 id={slug} class="group relative mt-12 font-semibold text-text-strong">
	<button
		type="button"
		aria-label="Copy link to section"
		onclick={copyHash}
		class="absolute top-1/2 -left-7 hidden -translate-y-1/2 cursor-pointer py-1 pr-2 pl-1 opacity-0 transition-opacity duration-200 ease-out group-hover:opacity-100 focus-visible:opacity-100 lg:block"
	>
		<span class="block text-text-soft transition-colors duration-200 hover:text-text-strong">
			<Hash class="h-4 w-4" aria-hidden="true" />
		</span>
	</button>
	{@render children()}
</h2>

<style>
	/* The button pads out its hit area beyond the glyph (py-1 pr-2 pl-1), but the
	focus ring should track just the hash icon: suppress the button's own ring
	(unlayered, beating the @layer base default) and redraw it on the icon span
	with a 0.125rem (2px) gap and radius. */
	button:focus-visible {
		outline: none;
		box-shadow: none;
	}

	button:focus-visible span {
		--focus-ring-offset: 0.125rem;
		--focus-ring-radius: 0.125rem;
		outline: var(--focus-ring-width) solid transparent;
		outline-offset: var(--focus-ring-offset);
		border-radius: var(--focus-ring-radius);
		box-shadow:
			0 0 0 var(--focus-ring-offset) var(--focus-ring-gap),
			0 0 0 calc(var(--focus-ring-offset) + var(--focus-ring-width)) var(--focus-ring-color);
	}
</style>
