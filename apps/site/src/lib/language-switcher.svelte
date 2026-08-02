<script lang="ts">
	import { page } from '$app/state';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Globe from '@lucide/svelte/icons/globe';
	import Languages from '@lucide/svelte/icons/languages';
	import { tick } from 'svelte';
	import { fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import {
		languageChoices,
		selectContentLanguage,
		type LanguageChoice,
	} from './language-switcher';
	import type { LocaleCode } from './locale';

	let { code }: { code: LocaleCode } = $props();
	let rootEl = $state<HTMLElement | undefined>();
	let triggerEl = $state<HTMLButtonElement | undefined>();
	let open = $state(false);
	const choices = $derived(languageChoices(code));
	const current = $derived(choices.find((choice) => choice.current) ?? choices[0]);

	/** A reader who asked for less motion gets none of it; the menu still opens. */
	function fadeMs(): number {
		return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ? 0 : 150;
	}

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
		{#if code === 'mw'}
			<Globe class="size-3.5" aria-hidden="true" />
		{:else}
			<Languages class="size-3.5" aria-hidden="true" />
		{/if}
		<span>{current?.name}</span>
		<ChevronUp
			class="size-3 transition-transform duration-200 ease-out motion-reduce:transition-none {open
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
				{#if index === 1}
					<div role="separator" class="border-t border-border"></div>
				{/if}
				<button
					data-language-option
					type="button"
					role="menuitemradio"
					aria-checked={choice.current}
					tabindex={choice.current ? 0 : -1}
					onclick={() => choose(choice)}
					onkeydown={(event) => handleOptionKeydown(event, index)}
					class="group flex w-full cursor-pointer items-center justify-between gap-3 px-2 py-1 text-left text-sm whitespace-nowrap hover:bg-paper-hover"
				>
					<span class={choice.current ? 'text-text-strong' : 'text-text-soft'}>{choice.name}</span>
					{#if choice.original}
						<Globe
							class="size-3.5 shrink-0 text-text-soft group-hover:text-text-strong"
							aria-hidden="true"
						/>
					{:else}
						<Languages
							class="size-3.5 shrink-0 text-text-soft group-hover:text-text-strong"
							aria-hidden="true"
						/>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
