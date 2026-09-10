<script lang="ts">
	import Moon from '@lucide/svelte/icons/moon';
	import Sun from '@lucide/svelte/icons/sun';
	import { applyTheme, currentTheme, themeCookie, type Theme } from '@canmi/theme';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let { locale }: { locale: LocaleCode } = $props();

	/**
	 * What is painted, not what was stored.
	 *
	 * The inline script in `app.html` has already put the class on `<html>` before this component
	 * exists, cookie or no cookie, so the document is the one place both a returning reader and a
	 * first-time one are described correctly. See @canmi/theme.
	 *
	 * The server cannot render this: it knows the cookie but not the system preference a first
	 * visit resolves from. So the button starts on the theme the page is already wearing, which
	 * the effect below reads once mounted; until then it renders neither icon rather than guessing
	 * one and swapping it a frame later.
	 */
	let theme = $state<Theme | undefined>();

	$effect(() => {
		theme = currentTheme();
	});

	/**
	 * Whether the icons have a change to play.
	 *
	 * Set only by a press. Arriving on a page is not a change of theme, and spinning the icon on
	 * every load would announce something that did not happen -- the same line the newsletter's
	 * confirmation draws between what the reader just did and what they are.
	 */
	let pressed = $state(false);

	const next = $derived<Theme>(theme === 'dark' ? 'light' : 'dark');

	function toggle() {
		if (theme === undefined) return;
		pressed = true;
		theme = next;
		applyTheme(theme);
		// Path and lifetime come from the library, so this and the first-visit script cannot
		// disagree about a preference the reader set once.
		document.cookie = themeCookie(theme);
	}
</script>

<!-- One cell holding both glyphs, so the row's height is the taller of the two in every state and
     nothing below the button moves as it changes. The label names what the press will do rather
     than what is showing: a control in a row of controls is read for its effect. -->
<button
	type="button"
	onclick={toggle}
	aria-label={m['theme.switch']({}, { locale })}
	aria-pressed={theme === 'dark'}
	class="quiet-control"
>
	<span class="dial focus-link-inner" class:turning={pressed}>
		<Sun class="size-3.5" data-shown={theme === 'light'} aria-hidden="true" />
		<Moon class="size-3.5" data-shown={theme === 'dark'} aria-hidden="true" />
	</span>
</button>

<style>
	/* Both glyphs in one grid cell. `visibility` rather than `display` so the box still measures
	   in either state and the two can cross without the row reflowing. */
	.dial {
		display: inline-grid;
		place-items: center;
	}

	.dial :global(svg) {
		grid-area: 1 / 1;
		visibility: hidden;
		opacity: 0;
		/* The turn is the whole animation: one leaves the way the other arrives, so the press
		   reads as one dial rotating rather than two icons swapping. */
		rotate: -90deg;
		scale: 0.6;
	}

	.dial :global(svg[data-shown='true']) {
		visibility: visible;
		opacity: 1;
		rotate: 0deg;
		scale: 1;
	}

	/* Only after a press. A page that loads already dark has not just changed. */
	.turning :global(svg) {
		transition:
			opacity 200ms ease,
			rotate 320ms var(--ease-spring),
			scale 320ms var(--ease-spring),
			visibility 320ms;
	}

	@media (prefers-reduced-motion: reduce) {
		.turning :global(svg) {
			transition: none;
		}
	}
</style>
