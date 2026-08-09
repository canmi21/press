import { localeUrl, type LocaleCode } from '$lib/locale';

/**
 * Addressing the card `cms og` rendered for a page.
 *
 * One card per page per language. The slug is the page's own path, so nothing stores a
 * reference and the address follows from the route; the language rides on `?lang=`, the same
 * parameter that selects the page itself. The CDN turns the pair into a key, so the layout the
 * cards are stored under stays private. See spec/architecture.md.
 */

/**
 * The size every consumer crops to, and what the renderer draws.
 *
 * Strings because their only use is a meta attribute. Carried as numbers they would be
 * stringified at each of them, which is the same value written two ways.
 */
export const CARD_WIDTH = '1200';
export const CARD_HEIGHT = '630';

/** The home page's card. Its route is `/`, which is not a slug anything can be filed under. */
export const HOME_SLUG = 'homepage';

/**
 * The card for one slug in one language.
 *
 * `localeUrl` rather than a template, so a card and the page it belongs to can never disagree
 * about how a language is named in a URL -- including that the source view carries no
 * parameter at all.
 */
export function cardUrl(cdn: string, slug: string, code: LocaleCode): string {
	const path = slug.replace(/^\/+/, '').replace(/\/+$/, '');
	return localeUrl(`${cdn}/opengraph/${path}.png`, code);
}
