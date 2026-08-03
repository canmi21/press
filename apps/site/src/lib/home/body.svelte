<script lang="ts">
	import Icon from './icons.svelte';
	import type { PageBlock } from '$lib/content/types';

	let { blocks }: { blocks: PageBlock[] } = $props();
</script>

{#each blocks as block, i (i)}
	{#if block.type === 'p'}
		<p>
			{#each block.segments as seg, j (j)}
				{#if seg.type === 'html'}
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html seg.html}
				{:else}
					<a
						href={seg.href}
						class="inline-flex items-center gap-1 align-middle leading-tight text-text-strong"
						{...seg.newTab ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
					>
						{#if seg.icon}<Icon name={seg.icon} />{/if}
						<span class="underline decoration-border underline-offset-4">{seg.label}</span>
						{#if seg.newTab}<span class="sr-only"> (opens in new tab)</span>{/if}
					</a>
				{/if}
			{/each}
		</p>
	{:else}
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html block.html}
	{/if}
{/each}
