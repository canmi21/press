<script lang="ts">
	import { page } from '$app/state';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import type { LocaleCode } from '$lib/locale';
	import { contentLanguageHref, sourceCode, sourceLanguageName } from '$lib/locale/switcher';
	import * as m from '$lib/paraglide/messages';

	type TranslationCode = Exclude<LocaleCode, 'mw'>;

	let { code, sourceLanguage }: { code: TranslationCode; sourceLanguage: string } = $props();

	const language = $derived(sourceLanguageName(sourceLanguage, code));
	const originalHref = $derived(contentLanguageHref('mw', page.url));
	const source = $derived(sourceCode(sourceLanguage));

	/**
	 * Which of the three things this view is, to the article.
	 *
	 * Script sibling is tested first: a Simplified article read at `tw` is also not the same
	 * code, and would otherwise be announced as a translation. See spec/locale.md.
	 */
	const message = $derived(
		(code === 'zh' && source === 'tw') || (code === 'tw' && source === 'zh')
			? m['notice.script']
			: source === code
				? m['notice.polished']
				: m['notice.translated'],
	);
</script>

<!--
	Square on the left so the bar reads as an edge rather than a lozenge; `text-soft` because
	this belongs to the metadata row above it, not to the article. See spec/locale.md.
-->
<div
	role="note"
	class="notice mt-4 rounded-r-md border-l-2 border-blue-ink py-1.5 pr-3 pl-3 text-sm leading-snug text-text-soft"
>
	<p>
		<ParaglideMessage {message} inputs={{ language }} options={{ locale: code }}>
			{#snippet link({ children })}
				<a
					href={originalHref}
					data-sveltekit-reload
					class="original-link font-medium">{@render children?.()}</a
				>
			{/snippet}
		</ParaglideMessage>
	</p>
</div>

<style>
	.notice {
		/* The one knob. Flat across the whole strip; past roughly 20% the tint stops reading as
		   tinted paper and becomes a coloured box. */
		--wash: 10%;
		background: color-mix(in oklab, var(--color-blue) var(--wash), transparent);
	}

	/* Drawn as a background rather than an underline because `text-decoration-line` does not
	   animate -- it is on or off, and no easing reaches it. A gradient sized to nothing and
	   grown to full width is the same one pixel, and it is a property that tweens.

	   The easing is a real spring, sampled from motion's own generator at stiffness 1200,
	   damping 70 and baked to `linear()`: overdamped, so it settles without overshoot. The
	   overshoot would be clipped by the element box anyway, which is the reason for damping it
	   out rather than spending it. Sampling here rather than animating through the library
	   keeps a hover off the main thread entirely. */
	.original-link {
		background-image: linear-gradient(currentColor, currentColor);
		background-repeat: no-repeat;
		background-position: 0 100%;
		background-size: 0% 1px;
		-webkit-box-decoration-break: clone;
		box-decoration-break: clone;
		transition: background-size 315ms
			linear(
				0,
				0.149,
				0.393,
				0.602,
				0.752,
				0.85,
				0.911,
				0.948,
				0.97,
				0.983,
				0.99,
				0.994,
				0.997,
				0.998,
				0.999,
				0.999,
				1
			);
	}

	.original-link:hover,
	.original-link:focus-visible {
		background-size: 100% 1px;
	}

	@media (prefers-reduced-motion: reduce) {
		.original-link {
			transition: none;
		}
	}
</style>
