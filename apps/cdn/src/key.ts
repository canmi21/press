/**
 * Turning a request path into the object it names, and naming a response about it.
 *
 * Pure string handling, kept apart from the route so it can be read and tested without loading a
 * codec. The route pulls in several megabytes of WebAssembly; none of that is needed to know
 * which object a URL is asking for.
 *
 * Where an object *lives* is not here. That is a fact about the bucket and belongs to
 * `@canmi/store`, which is what reads it -- it was written out once per worker until three
 * copies had to agree about which prefixes fan out.
 */

import { isContentId } from '@canmi/store';

/** The source view: what `?lang=` absent, or naming something unknown, resolves to. */
export const SOURCE_VIEW = 'mw';

/**
 * Every view `cms og` writes. A code outside this set is a typo rather than a language.
 *
 * Listed rather than accepted blindly, because the code becomes a path segment: an unchecked
 * one is a way to ask the bucket for an arbitrary prefix.
 */
export const VIEWS = new Set([SOURCE_VIEW, 'de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw']);

/**
 * The keys to try for an OpenGraph card, best first.
 *
 * A card is addressed by the slug of the page it belongs to plus `?lang=`; it is stored under
 * `opengraph/{view}/{slug}.png`. Returns the asked-for view followed by the source view, or
 * `null` when the path is not a card address at all.
 */
export function cardKeys(pathname: string, lang: string | null): string[] | null {
	const slug = pathname.replace(/^\/opengraph\/+/, '').replace(/\.png$/, '');
	// `..` would climb out of the prefix, and an empty slug names the directory rather than a card.
	if (!slug || slug.includes('..')) return null;

	const view = lang && VIEWS.has(lang) ? lang : SOURCE_VIEW;
	const keys = [`opengraph/${view}/${slug}.png`];
	if (view !== SOURCE_VIEW) keys.push(`opengraph/${SOURCE_VIEW}/${slug}.png`);
	return keys;
}

/** Split `{cid}.{ext}`, or null if it is not that shape. */
export function parseName(name: string): { cid: string; extension: string } | null {
	const dot = name.lastIndexOf('.');
	if (dot <= 0) return null;
	const cid = name.slice(0, dot).toLowerCase();
	const extension = name.slice(dot + 1).toLowerCase();
	return isContentId(cid) ? { cid, extension } : null;
}

/**
 * The spelling a request should have used, or `null` when it already has it.
 *
 * `jpg` is JPEG written for an eight-character filename limit that outlived the system imposing
 * it -- the same history that leaves `yml` beside `yaml`. Treating it as a second format would
 * put two of everything downstream: two validators, two edge cache entries and two transcodes,
 * over identical bytes. Normalising at the door leaves one of each.
 *
 * Here rather than beside the codecs because it is a fact about how a name is written, not about
 * what can be decoded.
 */
export function canonicalSpelling(extension: string): string | null {
	return extension === 'jpg' ? 'jpeg' : null;
}

/** One id serves several formats, so the format is part of what the tag identifies. */
export function validatorFor(cid: string, extension: string): string {
	return `"${cid}.${extension}"`;
}
