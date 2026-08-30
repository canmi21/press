/**
 * Tell the IndexNow participants which of this site's pages changed.
 *
 * The protocol is for URLs that were added, updated or deleted -- not for a periodic replay of
 * the whole sitemap, which is what its `429 Too Many Requests (potential Spam)` exists to stop.
 * So this reads the live sitemap, compares it against what was submitted last time, and sends
 * only the difference. An unchanged site sends nothing and exits.
 *
 * See spec/indexing.md.
 */
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { parse as parseYaml } from 'yaml';
import { URLS } from '@canmi/urls';

const SITE = URLS.apps.production.site;
const HOST = new URL(SITE).hostname;
const ENDPOINT = URLS.external.indexnow;

const CONFIG = fileURLToPath(new URL('../site.config.yaml', import.meta.url));
const RECORD = fileURLToPath(new URL('../../../data/indexnow.json', import.meta.url));

/** One request may carry 10,000 URLs. This site is nowhere near it; the cap is enforced anyway
 *  so that a future corpus splits into batches rather than being silently truncated. */
const BATCH = 10_000;

/** What was last submitted: path and query (no origin, one site) to the `lastmod` sent with
 *  it. */
type Submitted = Record<string, string>;

/** A sitemap entry reduced to the two things that decide whether to resubmit. */
type Entry = { path: string; lastmod: string };

function key(): string {
	const config = parseYaml(readFileSync(CONFIG, 'utf8')) as { indexnow?: string };
	const value = config.indexnow;
	if (!value) throw new Error(`site.config.yaml has no indexnow key`);
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
 * Read `<loc>` and `<lastmod>` out of the live sitemap.
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
async function sitemap(): Promise<Entry[]> {
	const response = await fetch(`${SITE}/sitemap.xml`);
	if (!response.ok) throw new Error(`sitemap.xml returned ${response.status}`);
	const xml = await response.text();

	const entries: Entry[] = [];
	for (const block of xml.matchAll(/<url>([\s\S]*?)<\/url>/g)) {
		const loc = /<loc>(.*?)<\/loc>/.exec(block[1])?.[1];
		const lastmod = /<lastmod>(.*?)<\/lastmod>/.exec(block[1])?.[1];
		if (!loc || !lastmod) continue;
		// Stored without the origin: this record describes one site, and carrying the host in
		// every row would be the same string repeated a few hundred times. The query survives,
		// though -- `?lang=` is what separates one article's nine views, and dropping it
		// collapses them onto one key and submits the same address nine times.
		const address = new URL(loc);
		entries.push({ path: `${address.pathname}${address.search}`, lastmod });
	}
	if (entries.length === 0) throw new Error('sitemap.xml parsed to no entries');
	return entries;
}

/** New, or changed since it was last sent. Anything else is what the protocol asks us not to
 *  resend. */
function changed(entries: Entry[], seen: Submitted): Entry[] {
	return entries.filter(({ path, lastmod }) => seen[path] !== lastmod);
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
const indexnow = key();
const entries = await sitemap();
const seen = submitted();
const pending = changed(entries, seen);

if (pending.length === 0) {
	console.log(`nothing to submit: ${entries.length} URLs in the sitemap, all already sent`);
	process.exit(0);
}

console.log(`${pending.length} of ${entries.length} URLs changed since the last submission:`);
for (const { path } of pending) console.log(`  ${path}`);

if (dry) {
	console.log('\n--dry: nothing sent, record untouched');
	process.exit(0);
}

// Sequential on purpose: these are submissions to one endpoint that rate-limits, and firing
// the batches at once is the shape its 429 is looking for. Only reached above 10,000 URLs.
// eslint-disable-next-line no-await-in-loop
for (let at = 0; at < pending.length; at += BATCH) {
	const batch = pending.slice(at, at + BATCH);
	// eslint-disable-next-line no-await-in-loop
	await submit(
		batch.map(({ path }) => `${SITE}${path}`),
		indexnow,
	);
}

// Written only after the request was accepted. Recording first would make a failed submission
// look sent, and the next run would skip exactly the URLs that never arrived.
const next: Submitted = { ...seen };
for (const { path, lastmod } of pending) next[path] = lastmod;
mkdirSync(fileURLToPath(new URL('../../../data/', import.meta.url)), { recursive: true });
writeFileSync(RECORD, `${JSON.stringify(next, null, '\t')}\n`);

console.log(`\nsubmitted ${pending.length} URLs; ${Object.keys(next).length} now on record`);
