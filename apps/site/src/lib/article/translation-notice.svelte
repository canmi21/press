<script lang="ts">
	import { page } from '$app/state';
	import { ParaglideMessage } from '@inlang/paraglide-js-svelte';
	import { contentLanguageCookie, type LocaleCode } from '$lib/locale';
	import {
		contentLanguageHref,
		languageName,
		publishedLabel,
		sourceCode,
		sourceLanguageName,
	} from '$lib/locale/switcher';
	import { fillSlot } from '$lib/locale/spacing';
	import * as m from '$lib/paraglide/messages';

	type TranslationCode = Exclude<LocaleCode, 'mw'>;

	let {
		code,
		sourceLanguage,
		available,
	}: { code: TranslationCode; sourceLanguage: string; available: boolean } = $props();

	const language = $derived(sourceLanguageName(sourceLanguage, code));
	const originalHref = $derived(contentLanguageHref('mw', page.url));
	const source = $derived(sourceCode(sourceLanguage));
	// The folded name, not the endonym: this sits inside a sentence, and `中文 (简体)版本` puts a
	// bracket between the language and the noun it qualifies. See spec/locale.md.
	const requestedLanguage = $derived(languageName(code));

	/**
	 * The language the reader is being shown, named the way the closed switcher names it.
	 *
	 * An article written in a language this site publishes no view of has no such label, so it
	 * falls back to the language spelled out in the reading view -- the same thing the notice
	 * said before, rather than a bracket with nothing to put in it.
	 */
	const shown = $derived(
		source ? publishedLabel(source) : sourceLanguageName(sourceLanguage, code),
	);

	/** Stands in for the language name while the sentence around it is measured. See fillSlot. */
	const SLOT = '\u0000';

	const unavailable = $derived(
		fillSlot(
			m['notice.unavailable']({ language: requestedLanguage, source: SLOT }, { locale: code }),
			SLOT,
			shown,
		),
	);

	function showOriginal(event: MouseEvent) {
		if (
			event.defaultPrevented ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.shiftKey ||
			event.altKey
		) {
			return;
		}
		event.preventDefault();
		document.cookie = contentLanguageCookie('mw', window.location.protocol === 'https:');
		window.location.reload();
	}

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
	{#if available}
		<p>
			<ParaglideMessage {message} inputs={{ language }} options={{ locale: code }}>
				{#snippet link({ children })}
					<a
						href={originalHref}
						data-sveltekit-reload
						onclick={showOriginal}
						class="focus-link spring-underline font-medium">{@render children?.()}</a
					>
				{/snippet}
			</ParaglideMessage>
		</p>
	{:else}
		<p>{unavailable}</p>
	{/if}
</div>

<style>
	.notice {
		/* The one knob. Flat across the whole strip; past roughly 20% the tint stops reading as
		   tinted paper and becomes a coloured box. */
		--wash: 10%;
		background: color-mix(in oklab, var(--color-blue) var(--wash), transparent);
	}
</style>
