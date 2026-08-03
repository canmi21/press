<script lang="ts">
	import { page } from '$app/state';
	import type { LocaleCode } from '$lib/locale';
	import { SCRIPT_NOTICE, TRANSLATION_NOTICE, type PolishedCopy } from '$lib/locale/messages';
	import { contentLanguageHref, sourceCode, sourceLanguageName } from '$lib/locale/switcher';

	type TranslationCode = Exclude<LocaleCode, 'mw'>;

	let { code, sourceLanguage }: { code: TranslationCode; sourceLanguage: string } = $props();

	const copy = $derived(TRANSLATION_NOTICE[code]);
	const originalLanguage = $derived(sourceLanguageName(sourceLanguage, code));
	const originalHref = $derived(contentLanguageHref('mw', page.url));

	const source = $derived(sourceCode(sourceLanguage));

	const sameLanguage = $derived(source === code);

	/** Same language, other script. Compared rather than cast, so the key is known to exist. */
	const scriptCopy = $derived(
		code === 'zh' && source === 'tw'
			? SCRIPT_NOTICE.zh
			: code === 'tw' && source === 'zh'
				? SCRIPT_NOTICE.tw
				: undefined,
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
		<!-- Script sibling first, or a Simplified article read at `tw` falls through and gets
		     announced as a translation. -->
		{#if scriptCopy}
			{@render recommendation(scriptCopy)}
		{:else if sameLanguage}
			{@render recommendation(copy.polished)}
		{:else}
			{copy.translated.beforeLanguage}<a
				href={originalHref}
				data-sveltekit-reload
				class="font-medium underline-offset-3 hover:underline"
				>{originalLanguage}</a
			>{copy.translated.afterLanguage}
		{/if}
	</p>
</div>

<!-- Both recommending states link the word for the original, not the language name. -->
{#snippet recommendation(text: PolishedCopy)}
	{text.beforeLanguage}{originalLanguage}{text.beforeLink}<a
		href={originalHref}
		data-sveltekit-reload
		class="font-medium underline-offset-3 hover:underline"
		>{text.linkLabel}</a
	>{text.afterLink}
{/snippet}

<style>
	.notice {
		/* The one knob. Flat across the whole strip; past roughly 20% the tint stops reading as
		   tinted paper and becomes a coloured box. */
		--wash: 10%;
		background: color-mix(in oklab, var(--color-blue) var(--wash), transparent);
	}
</style>
