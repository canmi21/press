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
	import { languageChoices, selectContentLanguage, type LanguageChoice } from './switcher';
	import { acceptedLocale, contentLanguageCookie, type LocaleCode } from './index';
	import * as m from '$lib/paraglide/messages';

	// `sourceLanguage` is an article's own language, and names the qualifier on the original row.
	// A page has none, and passes nothing; see languageChoices.
	let { code, sourceLanguage }: { code: LocaleCode; sourceLanguage?: string } = $props();
	let open = $state(false);

	/**
	 * The mark a language carries, assigned rather than derived.
	 *
	 * Three marks across eight languages, chosen per language: there is no property of a locale
	 * that produces this grouping, so it is written out instead of computed from one. The
	 * original stands apart with a globe, being the one view nothing was done to.
	 */
	const MARKS = {
		en: IconTranslate,
		es: IconTranslate,
		zh: IconTranslateSimplified,
		tw: IconTranslateAi,
		ja: IconTranslateAi,
		ko: IconTranslateAi,
		de: IconTranslateAi,
		fr: IconTranslateAi,
		mw: IconWorld,
	} as const satisfies Record<LocaleCode, unknown>;

	function markFor(choice: LanguageChoice) {
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
	 * The trigger says where the reader stands; the menu says what each language is.
	 *
	 * So when the view already matches what this browser asked for, the trigger carries the
	 * compass rather than a translation mark -- the one thing worth saying at a glance is that
	 * nothing needs changing. Inside the menu the marks keep naming languages, because there the
	 * compass has the other job: pointing at a row worth moving to.
	 */
	const CurrentMark = $derived(
		code === preferred || current === undefined ? Compass : markFor(current),
	);

	/**
	 * The two icon sets are not sized the same way, and this is the one slot that holds either.
	 *
	 * Iconify marks are set by height with an automatic width; a Lucide glyph at that height
	 * draws taller and lifts the whole metadata row, which is visible as the line shifting the
	 * moment a reader lands on their own language. Everywhere else a slot holds one set only, so
	 * this is the single place the difference has to be spelled out.
	 */
	const markSize = $derived(code === preferred ? 'size-3.25' : 'h-4 w-auto');

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
		aria-label={m['language.switcher']({ name: current?.name ?? '' }, { locale: code })}
		class="quiet-control"
	>
		<span class="focus-link-inner inline-flex items-center gap-1">
			<CurrentMark class={markSize} aria-hidden="true" />
			<span>{current?.name}</span>
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
				{@const Mark = markFor(choice)}
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
							class="h-4 w-auto shrink-0 text-text-soft group-data-[highlighted]:text-text-strong"
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
