import * as m from '../paraglide/messages';
import { PUBLIC_LANGUAGE, type LocaleCode } from './index';

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
 * reads the same whichever view rendered it. Only the pairs that need telling apart carry a
 * qualifier -- there is one English here and two Chinese.
 */
export const LANGUAGE_ENDONYMS = {
	en: 'English',
	zh: '中文 (简体)',
	tw: '中文 (繁體)',
	ja: '日本語',
	ko: '한국어',
	de: 'Deutsch',
	fr: 'Français',
	es: 'Español',
} as const satisfies Record<TranslationCode, string>;

/**
 * Two orders, chosen by what the reader's own language is rather than by the view.
 *
 * Names never change; only their sequence does, and it settles once per reader rather than
 * shifting as they move between views. Someone reading in Japanese should not have to walk past
 * four European languages to reach Chinese, and someone reading in French should not have to do
 * the reverse.
 *
 * Written out rather than derived from the endonym table, because there are now two of them and
 * an implicit order cannot express two. Both must name all eight; the tests hold them to it.
 */
const ORDER_CJK = ['en', 'zh', 'tw', 'ja', 'ko', 'de', 'fr', 'es'] as const;
const ORDER_LATIN = ['en', 'es', 'fr', 'ja', 'zh', 'tw', 'ko', 'de'] as const;

/** Reader languages that get the first order. Not `COMPACT_SCRIPT`: that one is about labels. */
const CJK_READER = new Set<LocaleCode>(['zh', 'tw', 'ja', 'ko']);

export function orderFor(preferred: LocaleCode): readonly TranslationCode[] {
	return CJK_READER.has(preferred) ? ORDER_CJK : ORDER_LATIN;
}

/**
 * Views whose script keeps a language name short enough to spell out.
 *
 * The test is what the label is written in, not which language the article happens to be. `mw`
 * is not among them: its own label reads `Original`, so the parenthetical beside it is English
 * too, and `Original (Chinese)` crowds a row that exists to be scanned in exactly the way the
 * other Latin views do.
 */
const COMPACT_SCRIPT = new Set<LocaleCode>(['zh', 'tw', 'ja', 'ko']);

/** Subtags that mark a Chinese tag as Traditional. Script wins; the regions are the legacy spelling. */
function isTraditional(subtags: string[]): boolean {
	return subtags.some((part) => part === 'hant' || ['tw', 'hk', 'mo'].includes(part));
}

/**
 * The view whose language is the one the article was written in, if this site publishes it.
 *
 * Resolved against `PUBLIC_LANGUAGE` rather than kept as a second table, so a locale that
 * changes its tag changes this with it. Traditional Chinese is matched by script before the
 * language falls through, since `zh-Hant` and `zh` differ in exactly the way the codes do.
 *
 * `undefined` means the article is in a language with no view of its own -- the eight are not a
 * promise about what may be written, only about what may be read.
 */
export function sourceCode(sourceLanguage: string): TranslationCode | undefined {
	const [primary = '', ...rest] = sourceLanguage.toLowerCase().split('-');
	const traditional = isTraditional(rest);
	return (Object.keys(PUBLIC_LANGUAGE) as TranslationCode[]).find((candidate) => {
		const [language, region] = PUBLIC_LANGUAGE[candidate].toLowerCase().split('-');
		if (language !== primary) return false;
		return primary === 'zh' ? (traditional ? region === 'tw' : region === 'cn') : true;
	});
}

/** The region of the locale this site publishes a language under. */
function regionOf(sourceLanguage: string): string | undefined {
	const code = sourceCode(sourceLanguage);
	return code ? PUBLIC_LANGUAGE[code].split('-')[1] : undefined;
}

/**
 * The tag handed to `Intl.DisplayNames`, with the script restored when it carries meaning.
 *
 * Frontmatter writes `lang: zh`, and `DisplayNames.of('zh')` answers "中文" / "Chinese" -- a name
 * covering both scripts, which names neither. Chinese is the only language here whose display
 * name splits by script, and every interface language already spells that split out (`简体中文`,
 * `Simplified Chinese`, `簡体中国語`, `중국어(간체)`), so the script is added to the tag rather than
 * eight names being written by hand. See spec/locale.md.
 */
function displayTag(sourceLanguage: string): string {
	const [primary = sourceLanguage, ...rest] = sourceLanguage.toLowerCase().split('-');
	if (primary !== 'zh') return primary;
	return isTraditional(rest) ? 'zh-Hant' : 'zh-Hans';
}

/**
 * Which language the original is in, named as briefly as the reading view allows.
 *
 * A CJK view spells it out, because `中文` and `한국어` are already as short as an abbreviation.
 * Every other view gets the region code, since `Original (Chinese)` crowds a row that exists to
 * be scanned.
 */
export function sourceLabel(sourceLanguage: string, currentCode: LocaleCode): string {
	const [primary = sourceLanguage] = sourceLanguage.split('-');
	if (!COMPACT_SCRIPT.has(currentCode)) return regionOf(sourceLanguage) ?? primary.toUpperCase();

	const tag = currentCode === 'mw' ? sourceLanguage : PUBLIC_LANGUAGE[currentCode];
	try {
		return new Intl.DisplayNames([tag], { type: 'language' }).of(primary) ?? primary.toUpperCase();
	} catch {
		return primary.toUpperCase();
	}
}

/** The source language spelled out in full, in the current interface language. */
export function sourceLanguageName(sourceLanguage: string, currentCode: LocaleCode): string {
	const tag = displayTag(sourceLanguage);
	const displayLanguage = currentCode === 'mw' ? sourceLanguage : PUBLIC_LANGUAGE[currentCode];
	const fallback = tag.split('-')[0]?.toUpperCase() ?? sourceLanguage.toUpperCase();
	try {
		return new Intl.DisplayNames([displayLanguage], { type: 'language' }).of(tag) ?? fallback;
	} catch {
		return fallback;
	}
}

/**
 * The eight, then the original last, as one entry among them rather than a section of its own.
 *
 * `mw` is labelled in whichever language is being read rather than in the article's own, because
 * it names a state and not a language. The endonyms above stay fixed for the opposite reason.
 */
export function languageChoices(
	currentCode: LocaleCode,
	sourceLanguage: string,
	preferred: LocaleCode = 'en',
): LanguageChoice[] {
	return [
		...orderFor(preferred).map((code) => ({
			code,
			name: LANGUAGE_ENDONYMS[code],
			original: false,
			current: currentCode === code,
		})),
		{
			code: 'mw' as const,
			name: `${m['language.original']({}, { locale: currentCode })} (${sourceLabel(sourceLanguage, currentCode)})`,
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
	navigate(contentLanguageHref(selectedCode, currentUrl));
	return true;
}

/** Preserve unrelated query state while asking the worker for another article view. */
export function contentLanguageHref(selectedCode: LocaleCode, currentUrl: URL): string {
	const target = new URL(currentUrl);
	target.searchParams.set('lang', selectedCode);
	return `${target.pathname}${target.search}${target.hash}`;
}
