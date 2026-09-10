import { inlineScriptString } from './inline-script';

const SYSTEM_DARK_QUERY = '(prefers-color-scheme:dark)';

export type Theme = 'light' | 'dark';

/**
 * How the theme cookie is written, wherever it is written.
 *
 * The script below settles the first visit and a control settles every one after it, so both
 * write this cookie -- and a control that wrote a shorter life, or a different path, would let
 * the two disagree about a preference the reader only set once. One string, interpolated into
 * the script and returned to the control.
 */
const COOKIE_ATTRIBUTES = ';path=/;max-age=31536000;SameSite=Lax';

export const themeScript = `(function(){var mm=document.cookie.match(/\\btheme=(light|dark)\\b/);var pm=document.cookie.match(/\\bpalette=(nord|contrast)\\b/);var m=mm?mm[1]:window.matchMedia(${inlineScriptString(SYSTEM_DARK_QUERY)}).matches?"dark":"light";var h=document.documentElement;if(m==="dark")h.classList.add("dark");if(pm)h.classList.add(pm[1]);if(!mm)document.cookie="theme="+m+${inlineScriptString(COOKIE_ATTRIBUTES)}})()`;

/** The cookie a control writes when the reader picks a theme. */
export function themeCookie(theme: Theme): string {
	return `theme=${theme}${COOKIE_ATTRIBUTES}`;
}

/**
 * Which theme is on screen right now, read off the document rather than off the cookie.
 *
 * The class is what the page is actually painted from, and the script above sets it before the
 * first frame whether or not a cookie existed. Reading the cookie instead would answer nothing
 * on a first visit, which is exactly when the two can differ.
 */
export function currentTheme(root: HTMLElement = document.documentElement): Theme {
	return root.classList.contains('dark') ? 'dark' : 'light';
}

/** Paint a theme. Toggling the class is the whole of it; every colour is a token beneath it. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
	root.classList.toggle('dark', theme === 'dark');
}

export function followSystemTheme(
	root: HTMLElement = document.documentElement,
	media: MediaQueryList = window.matchMedia(SYSTEM_DARK_QUERY),
): () => void {
	const apply = () => root.classList.toggle('dark', media.matches);
	apply();
	media.addEventListener('change', apply);
	return () => media.removeEventListener('change', apply);
}
