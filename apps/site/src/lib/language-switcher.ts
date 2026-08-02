import { MESSAGES } from './messages';
import type { LocaleCode } from './locale';

type TranslationCode = Exclude<LocaleCode, 'mw'>;

export type LanguageChoice = {
	code: LocaleCode;
	name: string;
	original: boolean;
	current: boolean;
};

/**
 * Each language named as its own readers write it, and never translated.
 *
 * A reader who cannot read the interface still has to find their language in this list, so it
 * reads the same whichever view rendered it.
 */
export const LANGUAGE_ENDONYMS = {
	en: 'English (US)',
	zh: '中文 (简体)',
	ja: '日本語',
	de: 'Deutsch',
	ko: '한국어',
	fr: 'Français',
	es: 'Español',
	tw: '中文 (繁體)',
} as const satisfies Record<TranslationCode, string>;

const TRANSLATION_CODES = Object.keys(LANGUAGE_ENDONYMS) as TranslationCode[];

/**
 * The original first, then the eight, and the first one is not one of them.
 *
 * `mw` is labelled in whichever language is being read rather than in the article's own, because
 * it names a state and not a language. The endonyms below stay fixed for the opposite reason.
 */
export function languageChoices(currentCode: LocaleCode): LanguageChoice[] {
	return [
		{
			code: 'mw',
			name: MESSAGES[currentCode].original,
			original: true,
			current: currentCode === 'mw',
		},
		...TRANSLATION_CODES.map((code) => ({
			code,
			name: LANGUAGE_ENDONYMS[code],
			original: false,
			current: currentCode === code,
		})),
	];
}

/** Switch through the worker even for `mw`; the existing cookie may name another view. */
export function selectContentLanguage(
	currentCode: LocaleCode,
	selectedCode: LocaleCode,
	currentUrl: URL,
	navigate: (href: string) => void,
): boolean {
	if (currentCode === selectedCode) return false;
	const target = new URL(currentUrl);
	target.searchParams.set('lang', selectedCode);
	navigate(`${target.pathname}${target.search}${target.hash}`);
	return true;
}
