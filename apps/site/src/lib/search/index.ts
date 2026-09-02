import { liteClient } from 'algoliasearch/lite';
import { site } from '$lib/site';
import type { LocaleCode } from '$lib/locale/index.ts';
import { MARK_CLOSE, MARK_OPEN, type SearchHit } from './query.ts';

/** The index every locale shares; see spec/search.md for why it is one and not nine. */
const INDEX = 'press_articles';

/**
 * `liteClient` rather than the full client, for two reasons that both matter here.
 *
 * It carries the retry across the four search hosts, which is the only part of talking to this
 * service that is not a single POST. And it has no write methods at all, so a key pasted into
 * the wrong place in browser code cannot reach anything but search.
 */
const client = liteClient(site.algolia.appId, site.algolia.searchKey);

/**
 * Search one locale.
 *
 * The locale is a filter rather than a separate index, so the caller passes the view the reader
 * is already on and gets back only that language.
 *
 * There is no cancellation here because the client offers none -- its `RequestOptions` carries
 * timeouts and headers and no `AbortSignal`. That costs less than it sounds: the request has
 * already left, so aborting would refund no part of the monthly budget and save only the
 * parsing. What actually has to be right is the ordering, and the caller enforces that by
 * discarding an answer that is no longer the newest.
 */
export async function search(query: string, locale: LocaleCode): Promise<SearchHit[]> {
	const { results } = await client.search<SearchHit>({
		requests: [
			{
				indexName: INDEX,
				query,
				filters: `locale:${locale}`,
				// Grouped into at most five articles below, so the request has to bring enough sections
				// for the grouping to have anything to choose from.
				hitsPerPage: 20,
				attributesToSnippet: ['text:28'],
				snippetEllipsisText: '…',
				highlightPreTag: MARK_OPEN,
				highlightPostTag: MARK_CLOSE,
			},
		],
	});

	const first = results[0];
	return first && 'hits' in first ? first.hits : [];
}

export { groupHits, markup, worthSearching, type SearchGroup, type SearchHit } from './query.ts';
