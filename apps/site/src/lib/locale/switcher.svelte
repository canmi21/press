<script lang="ts">
	// Mingcute rather than Lucide for the language marks: it distinguishes machine translation
	// from translation in general, which is the distinction this menu is about. Iconify icons
	// carry their own viewBox, so they are sized by height with an automatic width -- forcing a
	// square scales them inconsistently against each other. See spec/naming.md for the sizes.
	import IconTranslate from '~icons/mingcute/translate-line';
	import IconTranslateAi from '~icons/mingcute/translate-2-ai-line';
	import IconTranslateSimplified from '~icons/mingcute/translate-2-line';
	import IconWorld from '~icons/mingcute/world-2-line';
	import IconUpSmall from '~icons/mingcute/up-small-line';
	import Check from '@lucide/svelte/icons/check';
	import Compass from '@lucide/svelte/icons/compass';
	import { DropdownMenu } from 'bits-ui';
	import MenuContent from '$lib/components/menu-content.svelte';
	import {
		languageChoices,
		MARK_SIZE,
		selectContentLanguage,
		triggerLabel,
		type LanguageChoice,
		type MarkName,
	} from './switcher';
	import { acceptedLocale, contentLanguageCookie, SITE_LANGUAGE, type LocaleCode } from './index';
	import * as m from '$lib/paraglide/messages';

	// The language of the thing being read. An article passes its own; a page passes nothing and
	// takes the site's, which is what its `<html lang>` already declares. See languageChoices.
	let { code, sourceLanguage = SITE_LANGUAGE }: { code: LocaleCode; sourceLanguage?: string } =
		$props();
	let open = $state(false);

	/**
	 * The mark a language carries, assigned rather than derived.
	 *
	 * Three marks across eight languages, chosen per language: there is no property of a locale
	 * that produces this grouping, so it is written out instead of computed from one. The
	 * original stands apart with a globe, being the one view nothing was done to.
	 */
	const MARKS = {
		en: 'translate',
		es: 'translate',
		zh: 'translate-simplified',
		tw: 'translate-ai',
		ja: 'translate-ai',
		ko: 'translate-ai',
		de: 'translate-ai',
		fr: 'translate-ai',
		mw: 'world',
	} as const satisfies Record<LocaleCode, MarkName>;

	/**
	 * Named rather than imported straight into the table above, because a mark is now two things:
	 * the glyph, and the height that glyph needs to read the same size as the rest. The name is
	 * what joins them, and it is what `MARK_SIZE` in switcher.ts is keyed by.
	 */
	const MARK_ICON = {
		translate: IconTranslate,
		'translate-simplified': IconTranslateSimplified,
		'translate-ai': IconTranslateAi,
		world: IconWorld,
	} as const satisfies Record<MarkName, unknown>;

	function markFor(choice: LanguageChoice): MarkName {
		return MARKS[choice.code];
	}

	/**
	 * What this browser would have asked for, run through the same parser the worker uses.
	 *
	 * `navigator.languages` is already in descending preference, which is the shape an
	 * Accept-Language header has, so joining it feeds the server's own negotiation rather than a
	 * second reading of the same preferences. Two implementations would eventually disagree, and
	 * the disagreement would show up as a marker pointing at the wrong row.
	 */
	const preferred = $derived(
		acceptedLocale(globalThis.navigator?.languages?.join(',') ?? globalThis.navigator?.language) ??
			'en',
	);

	// Ordered by the reader's own language rather than by the view, so the sequence settles once
	// per reader instead of shifting as they move between translations.
	const choices = $derived(languageChoices(code, sourceLanguage, preferred));
	const current = $derived(choices.find((choice) => choice.current) ?? choices[0]);

	/**
	 * The closed control names the language; the menu names the choices.
	 *
	 * So this is not the current row's label. A row has the whole list beside it for context and
	 * can afford to read `Original`; the trigger stands alone in a metadata row and has to answer
	 * what is being read without one. See switcher.ts.
	 */
	const label = $derived(triggerLabel(code, sourceLanguage));

	/**
	 * The trigger says where the reader stands; the menu says what each language is.
	 *
	 * So when the view already matches what this browser asked for, the trigger carries the
	 * compass rather than a translation mark -- the one thing worth saying at a glance is that
	 * nothing needs changing. Inside the menu the marks keep naming languages, because there the
	 * compass has the other job: pointing at a row worth moving to.
	 */
	const currentMark = $derived(
		code === preferred || current === undefined ? undefined : markFor(current),
	);
	const CurrentMark = $derived(currentMark ? MARK_ICON[currentMark] : Compass);

	/**
	 * The one slot that holds either icon set, so the one place their difference is spelled out.
	 *
	 * Iconify marks are set by height with an automatic width, and every Lucide glyph in the
	 * metadata row this sits in is `size-3.5`. The compass was a step under that, on the grounds
	 * that a larger Lucide glyph lifted the row -- measured at 3.25, 3.5 and 4, the row is 24rem
	 * high at all three and nothing moves. That reason is gone.
	 *
	 * What is left is the glyph's own shape, and it is the reason this is not simply `size-3.5`.
	 * Lucide does not fill its box consistently: at the same 14px, `Type` inks 10.5px and
	 * `Sparkles` 12.8px. The compass inks 12.8px there too, but it reaches its box on every side
	 * because it is a circle, and a circle that reaches its box reads smaller than a glyph that
	 * only reaches it at the corners. So it is set one step above the row's Lucide size -- about
	 * 7%, which is the usual correction for a round mark among angular ones.
	 */
	const COMPASS_SIZE = 'size-3.75';

	const markSize = $derived(currentMark ? MARK_SIZE[currentMark] : COMPASS_SIZE);

	function choose(nextCode: string) {
		open = false;
		const choice = choices.find(({ code: choiceCode }) => choiceCode === nextCode);
		if (!choice) return;
		const navigated = selectContentLanguage(code, choice.code, (selectedCode) => {
			document.cookie = contentLanguageCookie(selectedCode, window.location.protocol === 'https:');
			window.location.reload();
		});
		if (!navigated) open = false;
	}
</script>

<DropdownMenu.Root {open} onOpenChange={(next) => (open = next)}>
	<DropdownMenu.Trigger
		aria-label={m['language.switcher']({ name: label }, { locale: code })}
		class="quiet-control"
	>
		<span class="focus-link-inner inline-flex items-center gap-1">
			<CurrentMark class={markSize} aria-hidden="true" />
			<span>{label}</span>
			<!-- Pulled back into the gap: the glyph carries its own padding inside the viewBox, so
			     the 0.25rem gap reads as noticeably more than it does beside the mark on the left. -->
			<IconUpSmall
				class="-ml-0.5 h-4 w-auto transition-transform duration-200 ease-out motion-reduce:transition-none {open
					? ''
					: 'rotate-180'}"
				aria-hidden="true"
			/>
		</span>
	</DropdownMenu.Trigger>

	<MenuContent id="article-language-menu">
		<DropdownMenu.RadioGroup value={code} onValueChange={choose}>
			{#each choices as choice (choice.code)}
				{@const mark = markFor(choice)}
				{@const Mark = MARK_ICON[mark]}
				<DropdownMenu.RadioItem
					data-language-option
					value={choice.code}
					aria-label={!choice.current && choice.code === preferred
						? `${choice.name}, your browser's preference`
						: undefined}
					class="group flex w-full cursor-pointer items-center gap-2 px-2 py-1 text-left text-sm whitespace-nowrap outline-none data-[highlighted]:bg-paper-hover"
				>
					{#snippet children({ checked })}
						<Mark
							class="{MARK_SIZE[mark]} shrink-0 text-text-soft group-data-[highlighted]:text-text-strong"
							aria-hidden="true"
						/>
						<span class="flex-1 {checked ? 'text-text-strong' : 'text-text-soft'}"
							>{choice.name}</span
						>
						<!-- One marker at most: being the current view outranks being the browser's
						     preference, and showing both on one row would say the same thing twice. -->
						{#if checked}
							<Check class="size-3.25 shrink-0 text-text-strong" aria-hidden="true" />
						{:else if choice.code === preferred}
							<Compass class="size-3.25 shrink-0 text-text-soft" aria-hidden="true" />
						{/if}
					{/snippet}
				</DropdownMenu.RadioItem>
			{/each}
		</DropdownMenu.RadioGroup>
	</MenuContent>
</DropdownMenu.Root>
