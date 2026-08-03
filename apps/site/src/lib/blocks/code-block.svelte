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
		class="code-scroll overflow-x-auto rounded-xl border border-border bg-paper p-4 pr-16 text-sm leading-snug"
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

	/* Shiki's <pre> takes focus (tabindex=0), but the ring should wrap the bordered
	box. Suppress the ring on the focused pre and redraw the token ring on the box
	when it holds focus. Unlayered, so it beats the @layer base :focus-visible. */
	.codeblock :global(pre:focus-visible) {
		outline: none;
		box-shadow: none;
	}

	.codeblock :global(.code-scroll:has(pre:focus-visible)) {
		/* Flush: no gap, so the accent hugs the box's rounded-xl edge. */
		--focus-ring-offset: 0px;
		outline: var(--focus-ring-width) solid transparent;
		outline-offset: var(--focus-ring-offset);
		box-shadow:
			0 0 0 var(--focus-ring-offset) var(--focus-ring-gap),
			0 0 0 calc(var(--focus-ring-offset) + var(--focus-ring-width)) var(--focus-ring-color);
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
