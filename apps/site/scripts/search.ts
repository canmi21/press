/**
 * Align the search index with what this site serves.
 *
 * A sibling of `indexnow.ts` rather than a copy of it: that one announces addresses to other
 * people's engines and can only write, so it keeps a local record of what it sent. This one
 * talks to an index it can also read, which removes the record entirely -- the fingerprint of
 * each record lives on the record itself, in the index, and a run diffs the corpus against what
 * is actually there. Nothing local can be wrong about production because nothing local is kept.
 *
 * See spec/search.md.
 */
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { parse as parseYaml } from 'yaml';
import { URLS } from '@canmi/urls';
import { buildArticles } from '../src/lib/content/build/articles.ts';
import { LOCALE_CODES } from '../src/lib/locale/index.ts';
import type { Article, ArticleView } from '../src/lib/content/types.ts';

/** One index holds every locale; see spec/search.md for why the languages are not declared. */
const INDEX = 'press_articles';

const ROOT = new URL('../../../', import.meta.url);
const SITE = new URL('apps/site/', ROOT);
const CONFIG = fileURLToPath(new URL('site.config.yaml', SITE));

/** A record's shape in the index. `fingerprint` is written but never searched. */
type Record = {
	objectID: string;
	path: string;
	locale: string;
	url: string;
	title: string;
	subtitle: string;
	heading: string;
	text: string;
	fingerprint: string;
};

type Credentials = { appId: string; writeKey: string };

function credentials(config: { algolia?: { appId?: string } }): Credentials {
	const appId = config.algolia?.appId;
	const writeKey = process.env.ALGOLIA_WRITE_KEY;
	if (!appId) throw new Error('site.config.yaml has no algolia.appId');
	// Decrypted out of secrets.json by mise on entering the directory. Absent means the sops key
	// is missing or the shell never entered the repo, and both read the same from here.
	if (!writeKey) throw new Error('ALGOLIA_WRITE_KEY is not set; see spec/search.md');
	return { appId, writeKey };
}

/**
 * The sections of one view, each anchored at the heading it falls under.
 *
 * Sections rather than whole articles for two reasons. A record has a size ceiling that a long
 * article passes on its own, and a result that lands on the paragraph answering the query is
 * worth more than one that lands at the top of the page.
 *
 * The boundaries come from `blocks`, which carries each heading's slug, while the words come
 * from `text`, which the compiler already renders with advanced expressions collapsed to empty.
 * Neither is re-derived: taking the text from the blocks would mean reimplementing the plain
 * text renderer, and taking the slugs from the text would mean guessing at them.
 */
function sections(view: ArticleView): { anchor: string; heading: string; text: string }[] {
	const headings = view.blocks
		.filter((block) => block.type === 'heading')
		.map((block) => ({ slug: block.slug, text: block.text }));

	// The lead paragraphs, before the article reaches a heading of its own.
	let current = { anchor: '', heading: '', text: [] as string[] };
	const out = [current];
	let next = 0;
	for (const paragraph of view.text.split('\n\n')) {
		// Matched in order rather than against a set: a paragraph repeating a heading's words is
		// prose, and only the next heading the article actually reaches can open a section.
		const heading = headings[next];
		if (heading && paragraph.trim() === heading.text) {
			current = { anchor: heading.slug, heading: heading.text, text: [] };
			out.push(current);
			next += 1;
			continue;
		}
		current.text.push(paragraph);
	}

	return out.flatMap((section) => chunk(section.anchor, section.heading, section.text));
}

/**
 * A ceiling on how much text one record carries, in bytes.
 *
 * Not the vendor's limit, which is higher: a section that merely fits is a section one edit
 * away from not fitting, and the failure lands at push time on whoever was publishing rather
 * than at the moment the paragraph was written. CJK costs three bytes a character, so this is a
 * few hundred characters of Chinese with the rest of the record's fields still to come.
 */
const CHUNK_BYTES = 4000;

/** One section's paragraphs, grouped into records that stay under the ceiling. */
function chunk(
	anchor: string,
	heading: string,
	paragraphs: string[],
): { anchor: string; heading: string; text: string }[] {
	let current: string[] = [];
	const out = [current];
	let held = 0;
	for (const paragraph of paragraphs) {
		const size = Buffer.byteLength(paragraph);
		// A single paragraph over the ceiling still gets its own record: splitting mid-sentence
		// would cut a match in half, which is worse than one record running long.
		if (held > 0 && held + size > CHUNK_BYTES) {
			current = [];
			out.push(current);
			held = 0;
		}
		current.push(paragraph);
		held += size;
	}

	return out
		.map((group) => ({ anchor, heading, text: group.join('\n\n').trim() }))
		.filter((section) => section.text.length > 0);
}

/**
 * What a record is built from, hashed.
 *
 * The hash is over the record's own content, not over the files behind it. `indexnow.ts` hashes
 * the sources because it has no way to see what it sent; here the thing being compared is
 * present, so comparing it directly is both simpler and stricter. It also sidesteps a cost that
 * hashing sources would carry: the translation sidecar is one file for eight locales, so a
 * source hash moves for all eight when one of them is rewritten, and eight-ninths of the
 * resulting push would be identical bytes.
 */
function fingerprint(record: Omit<Record, 'fingerprint'>): string {
	const hash = createHash('sha256');
	hash.update(
		JSON.stringify([record.url, record.title, record.subtitle, record.heading, record.text]),
	);
	return hash.digest('hex').slice(0, 16);
}

function records(articles: Article[]): Record[] {
	const out: Record[] = [];
	for (const article of articles) {
		for (const code of LOCALE_CODES) {
			const view = article.views[code];
			for (const [index, section] of sections(view).entries()) {
				const base = {
					objectID: `${article.path}:${code}:${index}`,
					path: article.path,
					locale: code,
					// Taken as-is: `build/indexing.ts` already decided whether this view keeps its
					// own address or collapses onto the source's, and deciding it twice is how the
					// two come to disagree.
					url: section.anchor ? `${view.canonical}#${section.anchor}` : view.canonical,
					title: view.meta.title,
					subtitle: view.meta.subtitle,
					heading: section.heading,
					text: section.text,
				};
				out.push({ ...base, fingerprint: fingerprint(base) });
			}
		}
	}
	return out;
}

class Client {
	readonly #headers: HeadersInit;
	readonly #write: string;

	constructor({ appId, writeKey }: Credentials) {
		this.#headers = {
			'X-Algolia-API-Key': writeKey,
			'X-Algolia-Application-Id': appId,
			'Content-Type': 'application/json',
		};
		this.#write = `https://${appId}.algolia.net`;
	}

	async call<T>(path: string, method: string, payload?: unknown): Promise<T> {
		const response = await fetch(`${this.#write}${path}`, {
			method,
			headers: this.#headers,
			body: payload === undefined ? undefined : JSON.stringify(payload),
		});
		const body = await response.text();
		// The body names which object or which setting was refused; the status alone does not.
		if (!response.ok) {
			const error = new Error(`${method} ${path}: ${response.status} ${body.trim()}`);
			// Carried so a caller can tell a state apart from a failure without matching on prose.
			Object.assign(error, { status: response.status });
			throw error;
		}
		return JSON.parse(body) as T;
	}

	/** Applied asynchronously, so a run that returns before publication reports a stale diff. */
	async settle(taskID: number): Promise<void> {
		for (let attempt = 0; attempt < 120; attempt++) {
			const { status } = await this.call<{ status: string }>(
				`/1/indexes/${INDEX}/task/${taskID}`,
				'GET',
			);
			if (status === 'published') return;
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
		throw new Error(`task ${taskID} never published`);
	}

	/**
	 * Every objectID in the index with the fingerprint it was written with.
	 *
	 * An index that does not exist yet is the first run, not a failure: it holds nothing, which
	 * is exactly what an empty map says. The settings push below creates it.
	 */
	async remote(): Promise<Map<string, string>> {
		const found = new Map<string, string>();
		let cursor: string | undefined;
		do {
			const page = await this.call<{
				hits: { objectID: string; fingerprint?: string }[];
				cursor?: string;
			}>(`/1/indexes/${INDEX}/browse`, 'POST', {
				attributesToRetrieve: ['fingerprint'],
				hitsPerPage: 1000,
				...(cursor ? { cursor } : {}),
			});
			for (const hit of page.hits) found.set(hit.objectID, hit.fingerprint ?? '');
			cursor = page.cursor;
		} while (cursor);
		return found;
	}

	async browsed(): Promise<Map<string, string>> {
		try {
			return await this.remote();
		} catch (error) {
			if ((error as { status?: number }).status === 404) return new Map();
			throw error;
		}
	}

	async batch(requests: unknown[]): Promise<void> {
		for (let i = 0; i < requests.length; i += 100) {
			const { taskID } = await this.call<{ taskID: number }>(`/1/indexes/${INDEX}/batch`, 'POST', {
				requests: requests.slice(i, i + 100),
			});
			await this.settle(taskID);
		}
	}
}

const dry = process.argv.includes('--dry');
const config = parseYaml(await readFile(CONFIG, 'utf8')) as { algolia?: { appId?: string } };
const client = new Client(credentials(config));

const { articles } = await buildArticles({
	contents: fileURLToPath(new URL('contents', ROOT)),
	cdnUrl: URLS.apps.production.cdn,
	messages: fileURLToPath(new URL('messages', SITE)),
	assets: fileURLToPath(new URL('data/metadata.json', ROOT)),
	media: fileURLToPath(new URL('data/media.yaml', ROOT)),
	segments: fileURLToPath(new URL('data/build/segments.json', ROOT)),
	crates: fileURLToPath(new URL('data/build/crates.json', ROOT)),
	repos: fileURLToPath(new URL('data/build/repos.json', ROOT)),
	tweets: fileURLToPath(new URL('data/build/twitter.json', ROOT)),
});

const wanted = records(articles);
const held = await client.browsed();

const changed = wanted.filter((record) => held.get(record.objectID) !== record.fingerprint);
const gone = [...held.keys()].filter(
	(objectID) => !wanted.some((record) => record.objectID === objectID),
);

// A record has a size ceiling, and the section split above is what keeps it clear. Reported
// every run so the margin is visible before an article grows past it rather than after.
const largest = Math.max(...wanted.map((record) => Buffer.byteLength(JSON.stringify(record))));

console.log(
	`${articles.length} articles, ${wanted.length} records across ${LOCALE_CODES.length} locales, ` +
		`largest ${largest} bytes; ${held.size} in the index`,
);

if (changed.length === 0 && gone.length === 0) {
	console.log('nothing to do: every record matches its fingerprint');
	process.exit(0);
}

console.log(`${changed.length} to write, ${gone.length} to delete`);
for (const record of changed.slice(0, 20)) console.log(`  write  ${record.objectID}`);
if (changed.length > 20) console.log(`  ... and ${changed.length - 20} more`);
for (const objectID of gone.slice(0, 20)) console.log(`  delete ${objectID}`);
if (gone.length > 20) console.log(`  ... and ${gone.length - 20} more`);

if (dry) {
	// One record in full, because the counts above say how much would be sent and nothing about
	// whether it is right. The address and the anchor are what a reader lands on.
	if (changed.length > 0) console.log('\nfirst record:\n', changed[0]);
	console.log('\n--dry: nothing was sent');
	process.exit(0);
}

// Settings before records, so a first run never serves an index that has not been told which
// attributes are searchable. Pushed every run because it is one small request and the
// alternative is a setting that lives only in whoever ran it last.
await client.settle(
	(
		await client.call<{ taskID: number }>(`/1/indexes/${INDEX}/settings`, 'PUT', {
			// Ranked in this order. `fingerprint` is deliberately absent: it is written to be
			// compared, never matched. Nothing else is declared -- see spec/search.md for the
			// measurement that found the language settings inert.
			searchableAttributes: ['title', 'heading', 'subtitle', 'text'],
			attributesForFaceting: ['filterOnly(locale)'],
		})
	).taskID,
);

await client.batch([
	...changed.map((body) => ({ action: 'addObject', body })),
	...gone.map((objectID) => ({ action: 'deleteObject', body: { objectID } })),
]);

console.log(`done: ${changed.length} written, ${gone.length} deleted`);
