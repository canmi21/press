import { similarity } from './assemble.ts';
import { languageTag, LOCALE_CODES, type LocaleCode } from '../locale.ts';
import type { Alternate } from './types.ts';

export const CANONICAL_SIMILARITY_THRESHOLD = 0.9;

/** Canonicals and alternates are built together so an hreflang can never disagree with its view. */
export function indexingMetadata(
	url: string,
	sourceLanguage: string,
	raws: Readonly<Record<LocaleCode, string>>,
): { canonical: Record<LocaleCode, string>; alternates: Alternate[] } {
	const canonical = Object.fromEntries(
		LOCALE_CODES.map((code) => [
			code,
			code === 'mw' || similarity(raws.mw, raws[code]) >= CANONICAL_SIMILARITY_THRESHOLD
				? url
				: `${url}?lang=${code}`,
		]),
	) as Record<LocaleCode, string>;

	return {
		canonical,
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
