<script lang="ts">
	import { Hash } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		slug,
		depth = 2,
		notes = [],
		children,
	}: { slug: string; depth?: number; notes?: number[]; children: Snippet } = $props();

	function copyHash(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		history.replaceState(null, '', `#${slug}`);
	}
</script>

<!-- A subsection is the same size, weight and colour as a section, and sits closer to what comes
     before it. The type scale is already spent -- 16px title, 15px heading, 14px prose, one step
     apart and separated by weight rather than size -- so there is no smaller step to give a third
     level without flattening the first two. Space is the signal instead, and the honest one:
     sitting nearer says "this belongs to what is above", which is exactly the relation. It also
     earns the rail's filtering -- a reader who can see that a heading is a subsection reads a
     table of contents that lists only sections as complete, not as missing something. -->
<svelte:element
	this={`h${depth}`}
	id={slug}
	class="group relative font-semibold text-text-strong"
	class:mt-12={depth === 2}
	class:mt-8={depth !== 2}
>
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
	{#each notes as number (number)}<sup class="note-marker"
			><a id="marker-{number}" href="#note-{number}" class="note-marker-link focus-link jump-target"
				>{number}</a
			></sup
		>{/each}
</svelte:element>
