<script lang="ts">
	import Icon from './icons.svelte';
	import { m } from '$lib/paraglide/messages';
	import type { PageBlock } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';

	/** `locale` is the view being rendered. Passed rather than read: see spec/locale.md. */
	let { blocks, locale }: { blocks: PageBlock[]; locale: LocaleCode } = $props();
</script>

{#each blocks as block, i (i)}
	{#if block.type === 'p'}
		<p>
			{#each block.segments as seg, j (j)}
				{#if seg.type === 'html'}
					<!-- Compiled at build time from the tracked corpus, not reader input. Stated
					     rather than suppressed; see spec/lint-format.md. -->
					{@html seg.html}
				{:else}
					<a
						href={seg.href}
						class="focus-link inline-flex items-center gap-1 align-middle leading-tight text-text-strong"
						{...seg.newTab ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
					>
						{#if seg.icon}<Icon name={seg.icon} />{/if}
						<span class="underline decoration-border underline-offset-4">{seg.label}</span>
						{#if seg.newTab}<span class="sr-only">
								({m['support.new-tab']({}, { locale })})</span
							>{/if}
					</a>
				{/if}
			{/each}
		</p>
	{:else}
		<!-- Compiled at build time from the tracked corpus, not reader input. Stated rather
		     than suppressed; see spec/lint-format.md. -->
		{@html block.html}
	{/if}
{/each}
