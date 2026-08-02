import { similarity } from './assemble.ts';
import { languageTag, localeUrl, LOCALE_CODES, type LocaleCode } from '../locale.ts';
import type { Alternate } from './types.ts';

export const CANONICAL_SIMILARITY_THRESHOLD = 0.9;

/** Canonicals and alternates are built together so an hreflang can never disagree with its view. */
export function indexingMetadata(
	url: string,
	sourceLanguage: string,
	raws: Readonly<Record<LocaleCode, string>>,
): { canonical: Record<LocaleCode, string>; canonicalUrls: string[]; alternates: Alternate[] } {
	const canonical = Object.fromEntries(
		LOCALE_CODES.map((code) => [
			code,
			code === 'mw' || similarity(raws.mw, raws[code]) >= CANONICAL_SIMILARITY_THRESHOLD
				? url
				: localeUrl(url, code),
		]),
	) as Record<LocaleCode, string>;

	return {
		canonical,
		canonicalUrls: [...new Set(LOCALE_CODES.map((code) => canonical[code]))],
		alternates: [
			...LOCALE_CODES.map((code) => ({
				code,
				languageTag: languageTag(code, sourceLanguage),
				href: canonical[code],
			})),
			{ code: 'x-default', languageTag: 'x-default', href: url },
		],
	};
}
