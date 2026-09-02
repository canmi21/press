/**
 * Tell the IndexNow participants which of this site's pages changed.
 *
 * The protocol is for URLs that were added, updated or deleted -- not for a periodic replay of
 * the whole sitemap, which is what its `429 Too Many Requests (potential Spam)` exists to stop.
 * So this reads the live sitemap for the set of addresses, fingerprints the source each one is
 * built from, and submits only what a fingerprint says has moved. An unchanged site sends
 * nothing.
 *
 * See spec/indexing.md.
 */
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { parse as parseYaml } from 'yaml';
import { URLS } from '@canmi/urls';

const SITE = URLS.apps.production.site;
const HOST = new URL(SITE).hostname;
const ENDPOINT = URLS.external.indexnow;

const CONFIG = fileURLToPath(new URL('../site.config.yaml', import.meta.url));
const ROOT = new URL('../../../', import.meta.url);
const RECORD = fileURLToPath(new URL('data/indexnow.json', ROOT));

/** One request may carry 10,000 URLs. This site is nowhere near it; the cap is enforced anyway
 *  so that a future corpus splits into batches rather than being silently truncated. */
const BATCH = 10_000;

/** What was last submitted: path and query (no origin, one site) to the fingerprint sent with
 *  it. */
type Submitted = Record<string, string>;

/** A sitemap address paired with the fingerprint of the source behind it. */
type Entry = { path: string; fingerprint: string };

function key(): string {
	const config = parseYaml(readFileSync(CONFIG, 'utf8')) as { indexnow?: string };
	const value = config.indexnow;
	if (!value) throw new Error('site.config.yaml has no indexnow key');
	return value;
}

function submitted(): Submitted {
	try {
		return JSON.parse(readFileSync(RECORD, 'utf8')) as Submitted;
	} catch {
		// No record yet is the first run, not a failure: everything in the sitemap is new.
		return {};
	}
}

/**
 * Hash the files a page is built from, tolerating the ones that do not exist.
 *
 * Truncated the same way segment and asset ids are: this answers "same or different", and the
 * full digest would be thirty-two more characters of noise in a file a person occasionally
 * opens.
 */
function fingerprint(paths: string[]): string {
	const hash = createHash('sha256');
	for (const path of paths) {
		try {
			hash.update(readFileSync(fileURLToPath(new URL(path, ROOT))));
		} catch {
			// A sidecar that has not been generated yet is a real state rather than an error:
			// its absence is part of what this fingerprint describes, and the value changes on
			// its own the day the file appears.
			hash.update('\0');
		}
	}
	return hash.digest('hex').slice(0, 16);
}

/**
 * The source files behind one address, or nothing if it is not announced.
 *
 * **Not the sitemap's `lastmod`.** That is a display value the author owns -- there are edits
 * where the date shown to a reader should deliberately stay put -- so reading it here would tie
 * "tell the search engines" to a decision about what a page claims about itself. It is also
 * wrong in both directions: a rebuilt but unchanged page carries a fresh build timestamp and
 * would be announced for nothing, while an article whose translations were rewritten keeps its
 * old frontmatter date and would never be announced at all.
 *
 * **Not the file's mtime either.** That was the first instinct and does not survive: mtime is
 * recorded by neither jj nor git, so a fresh clone dates every file to the checkout and the next
 * run announces the whole site. A content hash is what actually answers the question, and it is
 * the primitive segment ids and asset ids already use.
 */
function sources(path: string, articles: string[]): string[] | undefined {
	// `split` always yields a first element; the fallback is what says so to the checker.
	const [route = path, query] = path.split('?');

	// The licence directories are in the sitemap so they can be crawled, and that is all they
	// need. They are derived pages nobody is waiting on, and the only timestamp they have is the
	// build's -- announcing them would mean announcing thirty URLs on every deploy.
	if (route.startsWith('/licenses')) return undefined;

	// The home page is a list of the articles, so it changes when any of them does.
	if (route === '/') return articles.map((slug) => `contents${slug}.md`);

	if (!articles.includes(route)) return undefined;

	// The source view shows the article and its summary; a translated view additionally shows
	// whatever the sidecar holds for it. Hashed whole rather than per locale: the sidecar is one
	// file, and reading a single locale out of it would buy a distinction that only matters on
	// the days translations change anyway, at the cost of teaching this script the sidecar's
	// shape.
	return query
		? [`contents${route}.md`, `contents${route}.i18n.yaml`, `contents${route}.summary.yaml`]
		: [`contents${route}.md`, `contents${route}.summary.yaml`];
}

/**
 * Every address in the live sitemap.
 *
 * Fetched over HTTP rather than read from the build, because the sitemap route is generated per
 * request -- its `changefreq` and `priority` reflect staleness at crawl time -- so there is no
 * build artifact to read. What is live is also what a search engine would see, which is the
 * thing being reconciled.
 *
 * Parsed with a regex rather than an XML library. The document is emitted by a route in this
 * repository a few lines away, one `<url>` per line group, and a dependency to re-read a shape
 * we ourselves wrote would be a dependency to keep current for no gain.
 */
async function addresses(): Promise<string[]> {
	const response = await fetch(`${SITE}/sitemap.xml`);
	if (!response.ok) throw new Error(`sitemap.xml returned ${response.status}`);
	const xml = await response.text();

	const found: string[] = [];
	for (const block of xml.matchAll(/<url>([\s\S]*?)<\/url>/g)) {
		const inner = block[1];
		const loc = inner ? /<loc>(.*?)<\/loc>/.exec(inner)?.[1] : undefined;
		if (!loc) continue;
		// Stored without the origin: this record describes one site, and carrying the host in
		// every row would be the same string repeated a few hundred times. The query survives,
		// though -- `?lang=` is what separates one article's nine views, and dropping it
		// collapses them onto one key and submits the same address nine times.
		const address = new URL(loc);
		found.push(`${address.pathname}${address.search}`);
	}
	if (found.length === 0) throw new Error('sitemap.xml parsed to no entries');
	return found;
}

async function submit(urls: string[], indexnow: string): Promise<void> {
	const response = await fetch(ENDPOINT, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json; charset=utf-8' },
		body: JSON.stringify({
			host: HOST,
			key: indexnow,
			keyLocation: `${SITE}/${indexnow}.txt`,
			urlList: urls,
		}),
	});
	// 200 and 202 are both acceptance; everything else names what to fix, and the body is worth
	// printing because the status alone does not say which URL or which key it objected to.
	if (response.status !== 200 && response.status !== 202) {
		const body = await response.text();
		throw new Error(`IndexNow returned ${response.status}: ${body.trim() || '(no body)'}`);
	}
}

const dry = process.argv.includes('--dry');
// Record what is live without announcing it. For the case where the engines already have these
// URLs but this record does not agree -- a submission made by hand, or a change to how the
// fingerprint is computed, which makes every page look new while none of them is. Sending them
// again would be the exact thing this script exists to avoid.
const seed = process.argv.includes('--seed');
const indexnow = key();
const found = await addresses();

// Article routes, taken from the addresses rather than from a directory listing: the sitemap is
// what decides which pages exist, and a file with no address in it is not a page yet.
const articles = [
	...new Set(
		found
			.map((path) => path.split('?')[0] ?? path)
			.filter((route) => route !== '/' && !route.startsWith('/licenses')),
	),
];

const entries: Entry[] = [];
let skipped = 0;
for (const path of found) {
	const files = sources(path, articles);
	if (files) entries.push({ path, fingerprint: fingerprint(files) });
	else skipped += 1;
}

const seen = submitted();
const pending = entries.filter(({ path, fingerprint: current }) => seen[path] !== current);

console.log(
	`${found.length} URLs in the sitemap, ${skipped} not announced, ${entries.length} tracked`,
);

if (pending.length === 0) {
	console.log('nothing to submit: every tracked URL matches its recorded fingerprint');
	process.exit(0);
}

console.log(`\n${pending.length} changed since the last submission:`);
for (const { path } of pending) console.log(`  ${path}`);

if (dry) {
	console.log('\n--dry: nothing sent, record untouched');
	process.exit(0);
}

if (!seed) {
	// Sequential on purpose: these are submissions to one endpoint that rate-limits, and firing
	// the batches at once is the shape its 429 is looking for. Only reached above 10,000 URLs.
	for (let at = 0; at < pending.length; at += BATCH) {
		const batch = pending.slice(at, at + BATCH);
		// eslint-disable-next-line no-await-in-loop
		await submit(
			batch.map(({ path }) => `${SITE}${path}`),
			indexnow,
		);
	}
}

// Written only after the requests were accepted. Recording first would make a failed submission
// look sent, and the next run would skip exactly the URLs that never arrived.
//
// Rebuilt from what is tracked now rather than merged into what was there before, so an address
// that leaves the sitemap leaves the record with it instead of accumulating forever.
const next: Submitted = {};
for (const { path } of entries) if (seen[path]) next[path] = seen[path];
for (const { path, fingerprint: current } of pending) next[path] = current;
mkdirSync(fileURLToPath(new URL('data/', ROOT)), { recursive: true });
writeFileSync(RECORD, `${JSON.stringify(next, null, '\t')}\n`);

console.log(
	seed
		? `\n--seed: recorded ${pending.length} URLs without announcing them; ${Object.keys(next).length} now on record`
		: `\nsubmitted ${pending.length} URLs; ${Object.keys(next).length} now on record`,
);
