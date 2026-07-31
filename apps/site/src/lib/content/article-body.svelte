<script lang="ts">
	import CodeBlock from '$lib/codeblock.svelte';
	import Image from '$lib/image.svelte';
	import LinkCard from '$lib/linkcard.svelte';
	import Placeholder from '$lib/placeholder.svelte';
	import Section from '$lib/section.svelte';
	import SvgCanvas from '$lib/svg-canvas.svelte';
	import type { Block } from '$lib/content/types';

	let { blocks }: { blocks: Block[] } = $props();
</script>

{#each blocks as block, i (i)}
	{#if block.type === 'prose'}
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html block.html}
	{:else if block.type === 'heading'}
		<Section slug={block.slug}>{block.text}</Section>
	{:else if block.type === 'code'}
		<CodeBlock lang={block.lang} html={block.html} />
	{:else if block.type === 'image'}
		<Image src={block.src} alt={block.alt} />
	{:else if block.type === 'linkcard'}
		<LinkCard src={block.src} url={block.url} title={block.title} tone={block.tone} />
	{:else if block.type === 'placeholder'}
		<Placeholder kind={block.kind} meta={block.meta} />
	{:else if block.type === 'svgCanvas'}
		<SvgCanvas svg={block.svg} />
	{/if}
{/each}
