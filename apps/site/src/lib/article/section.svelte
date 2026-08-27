<script lang="ts">
	import { Hash } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		slug,
		notes = [],
		children,
	}: { slug: string; notes?: number[]; children: Snippet } = $props();

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
		class="absolute top-1/2 -left-7 hidden -translate-y-1/2 cursor-pointer py-1 pr-2 pl-1 opacity-0 transition-opacity duration-200 ease-out group-hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-none lg:block"
	>
		<span
			class="focus-ring-inner block text-text-soft transition-colors duration-200 hover:text-text-strong"
		>
			<Hash class="h-4 w-4" aria-hidden="true" />
		</span>
	</button>
	{@render children()}
	<!-- After the words rather than above them: a heading's note belongs to the heading, and a
	     marker floating off the cap line reads as belonging to the page. See spec/styling.md. -->
	{#each notes as number (number)}<sup class="fn-ref"
			><a id="fnref-{number}" href="#fn-{number}" class="fn-ref-link focus-link">{number}</a></sup
		>{/each}
</h2>
