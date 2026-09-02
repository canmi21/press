/**
 * The parts of search that are decisions rather than plumbing.
 *
 * Separated from the client so they can be tested without standing up a vendor connection, and
 * because they are the half that is actually about this site: what counts as a query worth
 * sending, and how a service's answer is made safe to render.
 */

/**
 * Sentinels the service wraps matches in, replaced with real markup after escaping.
 *
 * Algolia does not escape the attribute values it returns -- it hands back the stored text with
 * the tags inserted -- and this corpus is about web development, so its prose genuinely contains
 * `<title>` and `<div>`. Rendering the service's answer directly would let an article's own words
 * become markup. Control characters cannot occur in the source text, so swapping these out
 * afterwards is unambiguous in a way `<em>` never is.
 */
export const MARK_OPEN = '\u0001';
export const MARK_CLOSE = '\u0002';

/** One section of one view, as the index holds it. */
export type SearchHit = {
	objectID: string;
	path: string;
	locale: string;
	url: string;
	title: string;
	subtitle: string;
	heading: string;
	text: string;
	_highlightResult?: Partial<Record<'title' | 'heading', { value: string }>>;
	_snippetResult?: Partial<Record<'text', { value: string }>>;
};

const ESCAPES: Record<string, string> = {
	'&': '&amp;',
	'<': '&lt;',
	'>': '&gt;',
	'"': '&quot;',
	"'": '&#39;',
};

/**
 * A highlighted value, safe to render.
 *
 * Everything is escaped first and the sentinels become `<mark>` after, so the only markup that
 * survives is the markup this function put there.
 */
export function markup(value: string | undefined, fallback: string): string {
	return (value ?? fallback)
		.replace(/[&<>"']/g, (character) => ESCAPES[character] ?? character)
		.replaceAll(MARK_OPEN, '<mark>')
		.replaceAll(MARK_CLOSE, '</mark>');
}

const HAN = /\p{Script=Han}/u;

/**
 * Whether a query is worth a request yet.
 *
 * The usual rule is "at least two characters", and it is half wrong for this corpus. A single
 * Latin letter is noise -- `a` matches nearly everything and means nearly nothing -- but a
 * single Han character is a word: `渲`, `锈` and `码` are each a real thing to look for. Holding
 * Chinese to the Latin rule would cost a reader of the language this site is mostly written in
 * one search every time they used it.
 *
 * Kana and Hangul stay on the two-character rule rather than joining Han. A lone `の` is a
 * particle and a lone `이` usually is one too -- they are those scripts' equivalent of `a`, not
 * of `渲`. So the test is the script, not merely "is it CJK".
 *
 * Counted in code points rather than UTF-16 units, so a character outside the basic plane is one
 * character here as it is to the person who typed it.
 */
export function worthSearching(query: string): boolean {
	const characters = [...query];
	return characters.length >= 2 || (characters.length === 1 && HAN.test(query));
}

/** One article, with the sections of it that matched. */
export type SearchGroup = { path: string; title: string; sections: SearchHit[] };

/**
 * Collapse a flat result list into one entry per article.
 *
 * A record is a section, so an article whose subject is the query matches in many of them and
 * the raw list is the same title repeated down the panel -- which spends the reader's attention
 * on a fact they learned from the first row. Grouping says the title once and lets the sections
 * under it be the thing being chosen between.
 *
 * Order is relevance order: a group takes the position of its best section, and sections keep
 * theirs within it. Nothing is re-scored here, because the service already did that and a second
 * opinion computed from a truncated list would be a worse one.
 *
 * Both caps exist to keep the panel a glance rather than a page. They discard the tail of a long
 * answer on purpose: a reader who needs the fourth section of the sixth article is not being
 * served by a longer list, they are being served by a better query.
 */
export function groupHits(hits: SearchHit[], maxGroups = 5, maxPerGroup = 3): SearchGroup[] {
	const groups: SearchGroup[] = [];
	const byPath = new Map<string, SearchGroup>();

	const seen = new Map<string, Set<string>>();

	for (const hit of hits) {
		let group = byPath.get(hit.path);
		if (!group) {
			if (groups.length >= maxGroups) continue;
			group = { path: hit.path, title: hit.title, sections: [] };
			byPath.set(hit.path, group);
			seen.set(hit.path, new Set());
			groups.push(group);
		}
		// One row per destination. A section longer than the record ceiling is stored as several
		// records that share a heading and an anchor, so listing each would offer the reader a
		// choice between rows that go to the same place -- and the heading would appear twice
		// under a title that appears once, which is the repetition this grouping exists to end.
		// The first is kept because the service ranked it first.
		const anchors = seen.get(hit.path);
		const anchor = new URL(hit.url).hash;
		if (anchors?.has(anchor)) continue;
		anchors?.add(anchor);
		if (group.sections.length < maxPerGroup) group.sections.push(hit);
	}

	return groups;
}
