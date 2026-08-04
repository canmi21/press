import { defineCustomClientStrategy, defineCustomServerStrategy } from '../paraglide/runtime';
import { localeCode } from './index';

/**
 * The name Paraglide knows our negotiation by. Its `custom-` prefix is required by the compiler.
 */
export const STRATEGY = 'custom-negotiated';

/**
 * Hand Paraglide the locale this request already resolved to, and let it decide nothing.
 *
 * `resolveLocale` reads a query parameter, a cookie, `Accept-Language` and finally the article
 * itself; the last of those is content-dependent and no library strategy can see it. Rather
 * than approximate that with `cookie` plus `preferredLanguage` and have two negotiations
 * disagree in the cases that matter, the strategy array holds this alone -- no built-in
 * fallback -- and the answer is read back off the request. See spec/locale.md.
 */
export function registerServerStrategy(): void {
	defineCustomServerStrategy(STRATEGY, {
		getLocale: (request) => {
			const header = request?.headers.get('cookie') ?? '';
			const match = /(?:^|;)\s*language=([^;]*)/.exec(header);
			return localeCode(match?.[1] && decodeURIComponent(match[1]));
		},
	});
}

/**
 * The same answer on the client, read from the document the server just rendered.
 *
 * The server stamps its resolved code onto `<html data-locale>`, the same way it stamps the
 * theme class. Although the preference cookie is client-readable, it is only one input to
 * negotiation; the document carries the final answer that hydration is guaranteed to match.
 *
 * `setLocale` is a no-op: switching content language writes the preference and reloads the full
 * document, changing the article and the interface together. Letting Paraglide move the
 * interface alone would leave it describing an article that had not changed.
 */
export function registerClientStrategy(): void {
	defineCustomClientStrategy(STRATEGY, {
		getLocale: () => localeCode(document.documentElement.dataset.locale),
		setLocale: () => {},
	});
}
