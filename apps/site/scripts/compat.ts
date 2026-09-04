/**
 * What the shipped bundle needs that the declared baseline does not have.
 *
 * The question a polyfill list cannot answer for itself: which of core-js's several hundred
 * modules does this site actually reach for, given the browsers it says it supports? Asking
 * core-js alone gives the wrong answer -- `core-js-compat` reports everything the baseline
 * lacks, which is seventy-odd modules nobody here calls. What matters is the intersection with
 * real code, and only Babel can compute that half.
 *
 * So this runs babel-plugin-polyfill-corejs3 in the mode that decides where an injected import
 * would go, and collects those decisions instead of writing them out. Babel is never in the
 * build: Vite compiles with esbuild and this reads what Vite produced. See spec/compat.md.
 *
 * What it prints is the delta against a reviewed snapshot, not the whole set. The whole set is
 * mostly noise and always will be: `usage-global` has no types, so a bare `x.map(...)` is
 * attributed to Array and to Iterator alike, and core-js additionally patches spec corners in
 * methods that have existed for a decade. Forty of those are not a finding. One that appeared
 * since the last review is. `--update` accepts the current set as the new snapshot, which is a
 * deliberate act and shows up in a diff.
 *
 * `--json` prints the sets as data. Reports and never fails the run; see spec/compat.md.
 */

import { transformAsync } from '@babel/core';
// Imported rather than named as a string. Babel resolves a plugin name against its `root`, which
// is the directory the task is run from -- the workspace root, where pnpm has not put this. The
// module graph of this file has, so let it do the resolving.
import polyfillCorejs3 from 'babel-plugin-polyfill-corejs3';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const SITE = new URL('../', import.meta.url);
const CLIENT = new URL('.svelte-kit/output/client/', SITE);
const COMPATIBILITY = new URL('src/lib/client/compatibility.ts', SITE);
const PACKAGE = new URL('package.json', SITE);
const SNAPSHOT = new URL('scripts/compat-snapshot.json', SITE);

/** core-js's own name for a module, as it appears in a `core-js/modules/<id>.js` specifier. */
type ModuleId = string;

async function baseline(): Promise<string> {
	const manifest = JSON.parse(await readFile(fileURLToPath(PACKAGE), 'utf8'));
	const queries: string[] = manifest.browserslist;
	if (!Array.isArray(queries) || queries.length === 0) {
		throw new Error('apps/site/package.json declares no browserslist, so there is no baseline');
	}
	return queries.join(', ');
}

/** Every built JavaScript chunk, which is the code readers actually run. */
async function chunks(dir: URL): Promise<URL[]> {
	let entries;
	try {
		entries = await readdir(dir, { withFileTypes: true });
	} catch {
		throw new Error(
			`no client build at ${fileURLToPath(dir)} -- run \`pnpm --filter @canmi/site build\` first`,
		);
	}

	const found: URL[] = [];
	for (const entry of entries) {
		const child = new URL(`${entry.name}${entry.isDirectory() ? '/' : ''}`, dir);
		// Depth-first over a directory tree that is a few levels deep; nothing to overlap.
		// eslint-disable-next-line no-await-in-loop
		if (entry.isDirectory()) found.push(...(await chunks(child)));
		else if (entry.name.endsWith('.js')) found.push(child);
	}
	return found;
}

/**
 * The modules one chunk would have had injected.
 *
 * `usage-global` is the mode that reads a call site and decides which polyfill covers it. The
 * transformed output is thrown away -- what is wanted is the list of specifiers it reached for,
 * which is the only place that decision is visible.
 */
async function needs(file: URL, targets: string): Promise<Set<ModuleId>> {
	const source = await readFile(fileURLToPath(file), 'utf8');
	const result = await transformAsync(source, {
		filename: fileURLToPath(file),
		babelrc: false,
		configFile: false,
		sourceType: 'unambiguous',
		compact: true,
		sourceMaps: false,
		plugins: [[polyfillCorejs3, { method: 'usage-global', version: '3.50', proposals: false, targets }]],
	});

	const found = new Set<ModuleId>();
	for (const match of (result?.code ?? '').matchAll(/["']core-js\/modules\/([^"']+)\.js["']/g)) {
		found.add(match[1]!);
	}
	return found;
}

/** What `compatibility.ts` already loads, read from the file rather than kept as a second list. */
async function installed(): Promise<Set<ModuleId>> {
	const source = await readFile(fileURLToPath(COMPATIBILITY), 'utf8');
	const found = new Set<ModuleId>();
	for (const match of source.matchAll(/["']core-js\/modules\/([^"']+)\.js["']/g)) {
		found.add(match[1]!);
	}
	return found;
}

/** The set accepted at the last review, so a report is a delta rather than a wall. */
async function snapshot(): Promise<Set<ModuleId>> {
	try {
		return new Set(JSON.parse(await readFile(fileURLToPath(SNAPSHOT), 'utf8')));
	} catch {
		// Absent on the first run, which is not an error: everything is new, and `--update`
		// is how that set becomes the thing later runs are measured against.
		return new Set();
	}
}

const targets = await baseline();
const files = await chunks(CLIENT);

const required = new Set<ModuleId>();
for (const file of files) {
	// One at a time on purpose. Each transform holds a whole minified chunk's AST, and starting
	// a hundred and thirty of those together trades a memory spike for wall-clock nobody is
	// waiting on -- this runs after a build, not in front of a person.
	// eslint-disable-next-line no-await-in-loop
	for (const id of await needs(file, targets)) required.add(id);
}

const present = await installed();
const reviewed = await snapshot();

// Three questions, and only the first two are about drift. The third is the one that pays for
// this whole task: a polyfill nobody needs any more is bytes every reader downloads forever,
// and nothing else in the repo would ever mention it again.
const appeared = [...required].filter((id) => !reviewed.has(id)).toSorted();
const gone = [...reviewed].filter((id) => !required.has(id)).toSorted();
const surplus = [...present].filter((id) => !required.has(id)).toSorted();

if (process.argv.includes('--update')) {
	await writeFile(fileURLToPath(SNAPSHOT), `${JSON.stringify([...required].toSorted(), null, '\t')}\n`);
	console.log(`snapshot updated: ${required.size} modules accepted as reviewed`);
} else if (process.argv.includes('--json')) {
	console.log(JSON.stringify({ targets, chunks: files.length, appeared, gone, surplus }, null, '\t'));
} else {
	console.log(`baseline : ${targets}`);
	console.log(`scanned  : ${files.length} built chunks`);
	console.log(`reviewed : ${reviewed.size} modules in the snapshot, ${required.size} needed now`);

	if (appeared.length > 0) {
		console.log('\nnew since the snapshot -- check whether the site really calls these:');
		for (const id of appeared) console.log(`  ${id}`);
		console.log('  (real ones go in compatibility.ts; the rest are accepted with --update)');
	}
	if (gone.length > 0) {
		console.log('\ngone since the snapshot -- the code stopped reaching for them:');
		for (const id of gone) console.log(`  ${id}`);
	}
	if (surplus.length > 0) {
		console.log('\nloaded by compatibility.ts and not needed -- the baseline caught up, delete:');
		for (const id of surplus) console.log(`  ${id}`);
	}
	if (appeared.length === 0 && gone.length === 0 && surplus.length === 0) {
		console.log('\nno drift: the snapshot, the baseline and compatibility.ts agree.');
	}
}
