<script lang="ts">
	import { page } from '$app/state';
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
	import { tick } from 'svelte';
	import { fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import {
		languageChoices,
		selectContentLanguage,
		type LanguageChoice,
	} from './language-switcher';
	import { acceptedLocale, type LocaleCode } from './locale';

	let { code, sourceLanguage }: { code: LocaleCode; sourceLanguage: string } = $props();
	let rootEl = $state<HTMLElement | undefined>();
	let triggerEl = $state<HTMLButtonElement | undefined>();
	let open = $state(false);
	const choices = $derived(languageChoices(code, sourceLanguage));
	const current = $derived(choices.find((choice) => choice.current) ?? choices[0]);

	/** A reader who asked for less motion gets none of it; the menu still opens. */
	function fadeMs(): number {
		return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 0 : 150;
	}

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

	const CurrentMark = $derived(markFor(current));

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

	function optionButtons(): HTMLButtonElement[] {
		return rootEl
			? Array.from(rootEl.querySelectorAll<HTMLButtonElement>('[data-language-option]'))
			: [];
	}

	async function openMenu(focusIndex: number) {
		open = true;
		await tick();
		optionButtons()[focusIndex]?.focus();
	}

	function closeMenu(restoreFocus: boolean) {
		open = false;
		if (restoreFocus) triggerEl?.focus();
	}

	function choose(choice: LanguageChoice) {
		open = false;
		const navigated = selectContentLanguage(code, choice.code, page.url, (href) => {
			window.location.assign(href);
		});
		if (!navigated) triggerEl?.focus();
	}

	function handleTriggerKeydown(event: KeyboardEvent) {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		event.preventDefault();
		const currentIndex = Math.max(
			0,
			choices.findIndex((choice) => choice.current),
		);
		void openMenu(event.key === 'ArrowUp' ? choices.length - 1 : currentIndex);
	}

	function handleOptionKeydown(event: KeyboardEvent, index: number) {
		const buttons = optionButtons();
		let next: number | undefined;
		switch (event.key) {
			case 'ArrowDown':
				next = (index + 1) % buttons.length;
				break;
			case 'ArrowUp':
				next = (index - 1 + buttons.length) % buttons.length;
				break;
			case 'Home':
				next = 0;
				break;
			case 'End':
				next = buttons.length - 1;
				break;
			case 'Escape':
				event.preventDefault();
				closeMenu(true);
				return;
			case 'Tab':
				open = false;
				return;
			default:
				return;
		}
		event.preventDefault();
		if (next != null) buttons[next]?.focus();
	}

	function handleFocusOut(event: FocusEvent) {
		if (!rootEl || !open) return;
		const next = event.relatedTarget;
		if (!(next instanceof Node) || !rootEl.contains(next)) open = false;
	}

	$effect(() => {
		if (!open || !rootEl) return;
		const closeOutside = (event: PointerEvent) => {
			if (event.target instanceof Node && !rootEl?.contains(event.target)) open = false;
		};
		document.addEventListener('pointerdown', closeOutside, true);
		return () => document.removeEventListener('pointerdown', closeOutside, true);
	});
</script>

<div bind:this={rootEl} class="relative inline-flex" onfocusout={handleFocusOut}>
	<button
		bind:this={triggerEl}
		type="button"
		aria-haspopup="menu"
		aria-expanded={open}
		aria-controls="article-language-menu"
		aria-label="Content language: {current?.name}"
		onclick={() => (open ? closeMenu(false) : void openMenu(choices.findIndex((c) => c.current)))}
		onkeydown={handleTriggerKeydown}
		class="-mx-1 inline-flex cursor-pointer items-center gap-1 rounded-sm px-1 py-0.5 hover:bg-paper-hover hover:text-text-strong"
	>
		<CurrentMark class="h-4 w-auto" aria-hidden="true" />
		<span>{current?.name}</span>
		<IconUpSmall
			class="h-4 w-auto transition-transform duration-200 ease-out motion-reduce:transition-none {open
				? ''
				: 'rotate-180'}"
			aria-hidden="true"
		/>
	</button>

	{#if open}
		<div
			id="article-language-menu"
			role="menu"
			aria-label="Article language"
			in:fade={{ duration: fadeMs(), easing: cubicOut }}
			out:fade={{ duration: fadeMs(), easing: cubicOut }}
			class="absolute top-[calc(100%+0.5rem)] left-0 z-30 min-w-36 overflow-hidden rounded-md border border-border bg-paper shadow-sm"
		>
			{#each choices as choice, index (choice.code)}
				{@const Mark = markFor(choice)}
				<button
					data-language-option
					type="button"
					role="menuitemradio"
					aria-checked={choice.current}
					aria-label={!choice.current && choice.code === preferred
						? `${choice.name}, your browser's preference`
						: undefined}
					tabindex={choice.current ? 0 : -1}
					onclick={() => choose(choice)}
					onkeydown={(event) => handleOptionKeydown(event, index)}
					class="group flex w-full cursor-pointer items-center gap-2 px-2 py-1 text-left text-sm whitespace-nowrap hover:bg-paper-hover"
				>
					<Mark
						class="h-4 w-auto shrink-0 text-text-soft group-hover:text-text-strong"
						aria-hidden="true"
					/>
					<span class="flex-1 {choice.current ? 'text-text-strong' : 'text-text-soft'}"
						>{choice.name}</span
					>
					<!-- One marker at most: being the current view outranks being the browser's
					     preference, and showing both on one row would say the same thing twice. -->
					{#if choice.current}
						<Check class="size-3.25 shrink-0 text-text-strong" aria-hidden="true" />
					{:else if choice.code === preferred}
						<Compass class="size-3.25 shrink-0 text-text-soft" aria-hidden="true" />
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
