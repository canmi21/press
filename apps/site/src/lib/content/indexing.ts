import { similarity } from './assemble.ts';
import { localeUrl, LOCALE_CODES, PUBLIC_LANGUAGE, type LocaleCode } from '../locale.ts';
import type { Alternate } from './types.ts';

export const CANONICAL_SIMILARITY_THRESHOLD = 0.9;

/** Canonicals and alternates are built together so an hreflang can never disagree with its view. */
export function indexingMetadata(
	url: string,
	content: Readonly<Record<LocaleCode, string>>,
): { canonical: Record<LocaleCode, string>; canonicalUrls: string[]; alternates: Alternate[] } {
	const canonical = Object.fromEntries(
		LOCALE_CODES.map((code) => [
			code,
			code === 'mw' || similarity(content.mw, content[code]) >= CANONICAL_SIMILARITY_THRESHOLD
				? url
				: localeUrl(url, code),
		]),
	) as Record<LocaleCode, string>;

	return {
		canonical,
		canonicalUrls: [...new Set(LOCALE_CODES.map((code) => canonical[code]))],
		alternates: [
			...(Object.entries(PUBLIC_LANGUAGE) as [Exclude<LocaleCode, 'mw'>, string][]).map(
				([code, languageTag]) => ({ code, languageTag, href: canonical[code] }),
			),
			{ code: 'x-default', languageTag: 'x-default', href: url },
		],
	};
}
