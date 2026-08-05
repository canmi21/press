<script lang="ts">
	type Props = {
		lang?: string;
		code?: string;
		html?: string;
	};
	let { lang, code, html }: Props = $props();
</script>

<div class="codeblock relative">
	{#if lang && lang !== 'text' && lang !== 'plaintext'}
		<span
			class="pointer-events-none absolute top-3 right-3 z-10 text-xs tracking-wider text-text-soft uppercase select-none"
		>
			{lang}
		</span>
	{/if}
	<!-- Shiki tags its <pre> with tabindex=0 so keyboard users can focus and scroll the
	code, so focus lands on the inner code area, not this box. The ring is redirected
	out to this bordered box via :has (see <style>) so it wraps the whole code block. -->
	<div
		class="code-scroll focus-ring-within overflow-x-auto rounded-xl border border-border bg-paper p-4 pr-16 text-sm leading-snug"
	>
		{#if html}
			{@html html}
		{:else if code}
			<pre><code>{code}</code></pre>
		{/if}
	</div>
</div>

<style>
	.codeblock :global(pre) {
		background: transparent;
		margin: 0;
		padding: 0;
	}

	/* Shiki's <pre> takes focus, while the shared within utility draws on this box. */
	.codeblock :global(pre:focus-visible) {
		outline: none;
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
</style>
