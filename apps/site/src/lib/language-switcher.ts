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
/**
 * Reading order, not alphabetical: the languages this site is actually read in come first, then
 * the rest. Object order is the rendered order, so the two cannot drift apart.
 */
export const LANGUAGE_ENDONYMS = {
	en: 'English (US)',
	zh: '中文 (简体)',
	tw: '中文 (繁體)',
	ja: '日本語',
	ko: '한국어',
	de: 'Deutsch',
	fr: 'Français',
	es: 'Español',
} as const satisfies Record<TranslationCode, string>;

const TRANSLATION_CODES = Object.keys(LANGUAGE_ENDONYMS) as TranslationCode[];

/**
 * The eight, then the original last, as one entry among them rather than a section of its own.
 *
 * `mw` is labelled in whichever language is being read rather than in the article's own, because
 * it names a state and not a language. The endonyms above stay fixed for the opposite reason.
 */
export function languageChoices(currentCode: LocaleCode): LanguageChoice[] {
	return [
		...TRANSLATION_CODES.map((code) => ({
			code,
			name: LANGUAGE_ENDONYMS[code],
			original: false,
			current: currentCode === code,
		})),
		{
			code: 'mw' as const,
			name: MESSAGES[currentCode].original,
			original: true,
			current: currentCode === 'mw',
		},
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
