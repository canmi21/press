import { inlineScriptString } from './inline-script';

const SYSTEM_DARK_QUERY = '(prefers-color-scheme:dark)';

export const themeScript = `(function(){var mm=document.cookie.match(/\\btheme=(light|dark)\\b/);var pm=document.cookie.match(/\\bpalette=(nord|contrast)\\b/);var m=mm?mm[1]:window.matchMedia(${inlineScriptString(SYSTEM_DARK_QUERY)}).matches?"dark":"light";var h=document.documentElement;if(m==="dark")h.classList.add("dark");if(pm)h.classList.add(pm[1]);if(!mm)document.cookie="theme="+m+";path=/;max-age=31536000;SameSite=Lax"})()`;

export function followSystemTheme(
	root: HTMLElement = document.documentElement,
	media: MediaQueryList = window.matchMedia(SYSTEM_DARK_QUERY),
): () => void {
	const apply = () => root.classList.toggle('dark', media.matches);
	apply();
	media.addEventListener('change', apply);
	return () => media.removeEventListener('change', apply);
}
