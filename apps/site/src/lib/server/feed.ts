import { localeCode, type LocaleCode } from '../locale.ts';

/** A shared-cache feed may vary only by its URL, never by request headers or cookies. */
export function feedLocale(request: Pick<Request, 'url'>): LocaleCode {
	return localeCode(new URL(request.url).searchParams.get('lang')) ?? 'mw';
}
