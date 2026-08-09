import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import {
	HEADER,
	coordinates,
	dependentsOf,
	githubRepository,
	licenseOf,
	licenseSlug,
	licenseTerms,
	packagePagePath,
	registryPackageUrl,
	routePath,
	routeTable,
	textUrl,
	type LicenseRecord,
} from './record.ts';

// The committed record itself, read from disk rather than through the virtual module: these
// are claims about the real dependency tree, and the point is that they hold for it.
const RECORD = fileURLToPath(new URL('../../../../../data/build/licenses.json', import.meta.url));
const record = JSON.parse(readFileSync(RECORD, 'utf8')) as LicenseRecord;

describe('route paths', () => {
	it('drops the scheme and the escaping a purl only needs to stay one string', () => {
		expect(routePath('pkg:cargo/serde@1.0.219')).toBe('cargo/serde@1.0.219');
		expect(routePath('pkg:npm/%40sveltejs/kit@2.0.0')).toBe('npm/@sveltejs/kit@2.0.0');
	});

	it('builds the separate browser and registry addresses from one purl', () => {
		const purl = 'pkg:npm/%40sveltejs/kit@2.70.2';
		const parts = coordinates(purl);
		expect(parts).toEqual({ registry: 'npm', name: '@sveltejs/kit', version: '2.70.2' });
		expect(packagePagePath(purl)).toBe('/licenses/pkgs/npm/@sveltejs/kit@2.70.2');
		expect(registryPackageUrl(parts)).toBe(
			`${URLS.external.registries.npm}/package/@sveltejs/kit/v/2.70.2`,
		);
	});

	it('turns a license into one readable route segment', () => {
		expect(licenseSlug('Apache-2.0 WITH LLVM-exception')).toBe('apache-2-0-with-llvm-exception');
	});

	it('recognises a repository without treating deeper GitHub URLs as one', () => {
		const repository = `${URLS.external.github.web}/sveltejs/kit`;
		expect(githubRepository(`${repository}.git`)).toEqual({
			owner: 'sveltejs',
			name: 'kit',
			url: repository,
		});
		expect(githubRepository(`${repository}/tree/main`)).toBeUndefined();
		const elsewhere = new URL(repository);
		elsewhere.hostname = ['gitlab', 'com'].join('.');
		expect(githubRepository(elsewhere.href)).toBeUndefined();
	});

	// The reverse lookup is a table rather than a parser, so this is what proves the two
	// spellings agree for every real package -- including the scoped names whose embedded
	// slash a parser would have to guess at.
	it('reaches every package back from its own route path', () => {
		const table = routeTable(record);
		const purls = Object.keys(record.packages);
		expect(purls.length).toBeGreaterThan(0);
		for (const purl of purls) {
			expect(table.get(routePath(purl))).toBe(purl);
		}
		// One route path per package: a collision would silently serve one package's terms
		// under another's name.
		expect(table.size).toBe(purls.length);
	});
});

describe('dependents', () => {
	const tree: LicenseRecord = {
		version: 0,
		packages: {
			'pkg:npm/leaf@1.0.0': { dependents: ['pkg:npm/middle@1.0.0', 'pkg:npm/other@1.0.0'] },
			'pkg:npm/middle@1.0.0': { dependents: ['workspace:site'] },
			'pkg:npm/other@1.0.0': { dependents: ['pkg:npm/middle@1.0.0', 'workspace:site'] },
		},
	};

	it('splits the packages that name it from the ones that only reach it', () => {
		expect(dependentsOf(tree, 'pkg:npm/leaf@1.0.0')).toEqual({
			direct: ['pkg:npm/middle@1.0.0', 'pkg:npm/other@1.0.0'],
			indirect: ['workspace:site'],
		});
	});

	// `middle` is both: `leaf`'s own parent, and reachable again through `other`. Naming it
	// twice would read as two dependents rather than one arriving by two routes.
	it('keeps a package that is both direct and indirect on the direct side only', () => {
		const { direct, indirect } = dependentsOf(tree, 'pkg:npm/leaf@1.0.0');
		expect(direct).toContain('pkg:npm/middle@1.0.0');
		expect(indirect).not.toContain('pkg:npm/middle@1.0.0');
	});

	it('terminates on a cycle', () => {
		const cycle: LicenseRecord = {
			version: 0,
			packages: {
				'pkg:npm/a@1.0.0': { dependents: ['pkg:npm/b@1.0.0'] },
				'pkg:npm/b@1.0.0': { dependents: ['pkg:npm/a@1.0.0'] },
			},
		};
		expect(dependentsOf(cycle, 'pkg:npm/a@1.0.0')).toEqual({
			direct: ['pkg:npm/b@1.0.0'],
			indirect: [],
		});
	});

	// The claim the whole section rests on: every package in the record is here because
	// something asked for it, so none of them can have an empty direct list.
	it('gives every package in the real record at least one direct dependent', () => {
		const orphans = Object.entries(record.packages)
			.filter(([, entry]) => (entry.dependents ?? []).length === 0)
			.map(([purl]) => purl);
		expect(orphans).toEqual([]);
	});

	// A package can only be reached from a workspace root, so walking back from any of them
	// has to arrive at one.
	it('reaches a workspace label from every package in the real record', () => {
		for (const purl of Object.keys(record.packages)) {
			const { direct, indirect } = dependentsOf(record, purl);
			const reached = [...direct, ...indirect];
			expect(reached.some((label) => label.startsWith('workspace:'))).toBe(true);
		}
	});
});

// Every distinct expression in the tree as it stands, with what each flattens to. Taken from
// the real record rather than invented, and the last test below fails if the tree grows one
// this table has not been updated for -- which is the point of writing them all out.
const EXPRESSIONS: [string, string[]][] = [
	['MIT', ['MIT']],
	['MIT OR Apache-2.0', ['MIT', 'Apache-2.0']],
	['Apache-2.0', ['Apache-2.0']],
	['Apache-2.0 OR MIT', ['Apache-2.0', 'MIT']],
	['Unicode-3.0', ['Unicode-3.0']],
	['ISC', ['ISC']],
	['BSD-2-Clause', ['BSD-2-Clause']],
	['BSD-3-Clause', ['BSD-3-Clause']],
	['Unlicense OR MIT', ['Unlicense', 'MIT']],
	// Cargo's deprecated spelling of OR, with no spaces around it.
	['MIT/Apache-2.0', ['MIT', 'Apache-2.0']],
	['Apache-2.0/MIT', ['Apache-2.0', 'MIT']],
	['MPL-2.0', ['MPL-2.0']],
	// `WITH` binds an exception on; the result is one licence, not two.
	[
		'Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT',
		['Apache-2.0 WITH LLVM-exception', 'Apache-2.0', 'MIT'],
	],
	[
		'CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception',
		['CC0-1.0', 'Apache-2.0', 'Apache-2.0 WITH LLVM-exception'],
	],
	['BlueOak-1.0.0', ['BlueOak-1.0.0']],
	['Zlib OR Apache-2.0 OR MIT', ['Zlib', 'Apache-2.0', 'MIT']],
	['MIT OR Apache-2.0 OR Zlib', ['MIT', 'Apache-2.0', 'Zlib']],
	['BSD-2-Clause OR Apache-2.0 OR MIT', ['BSD-2-Clause', 'Apache-2.0', 'MIT']],
	['BSD-3-Clause OR Apache-2.0', ['BSD-3-Clause', 'Apache-2.0']],
	// An identifier that merely contains a shorter one. Splitting on substrings would tear
	// this into `FSL-1.1-` and `MIT`.
	['FSL-1.1-MIT', ['FSL-1.1-MIT']],
	['CC0-1.0 OR MIT-0 OR Apache-2.0', ['CC0-1.0', 'MIT-0', 'Apache-2.0']],
	// The lowercase `or` inside `-or-later` is part of the identifier, not the operator.
	['MIT OR Apache-2.0 OR LGPL-2.1-or-later', ['MIT', 'Apache-2.0', 'LGPL-2.1-or-later']],
	['LGPL-3.0-or-later', ['LGPL-3.0-or-later']],
	['Zlib', ['Zlib']],
	// Parentheses group a disjunction inside a conjunction; neither may stay glued to an id.
	['(MIT OR Apache-2.0) AND NCSA', ['MIT', 'Apache-2.0', 'NCSA']],
	['(MIT OR Apache-2.0) AND Unicode-3.0', ['MIT', 'Apache-2.0', 'Unicode-3.0']],
	['0BSD', ['0BSD']],
	['0BSD OR MIT OR Apache-2.0', ['0BSD', 'MIT', 'Apache-2.0']],
	[
		'AGPL-3.0-only OR LicenseRef-Imazen-Commercial',
		['AGPL-3.0-only', 'LicenseRef-Imazen-Commercial'],
	],
	['Apache-2.0 AND ISC', ['Apache-2.0', 'ISC']],
	['MIT AND ODbL-1.0', ['MIT', 'ODbL-1.0']],
	['Apache-2.0 OR BSL-1.0', ['Apache-2.0', 'BSL-1.0']],
	['Apache-2.0 OR GPL-2.0-only', ['Apache-2.0', 'GPL-2.0-only']],
	['Apache-2.0 OR ISC OR MIT', ['Apache-2.0', 'ISC', 'MIT']],
	['CC-BY-4.0', ['CC-BY-4.0']],
	['CC0-1.0', ['CC0-1.0']],
	['CC0-1.0 OR Apache-2.0', ['CC0-1.0', 'Apache-2.0']],
	['CDLA-Permissive-2.0', ['CDLA-Permissive-2.0']],
	['MIT OR Zlib OR Apache-2.0', ['MIT', 'Zlib', 'Apache-2.0']],
];

describe('splitting an SPDX expression', () => {
	it.each(EXPRESSIONS)('flattens %s', (expression, expected) => {
		expect(licenseTerms(expression)).toEqual(expected);
	});

	it('names a licence once however often the expression repeats it', () => {
		expect(licenseTerms('MIT OR MIT')).toEqual(['MIT']);
	});

	// The table above is only worth having if it stays complete. A new dependency bringing in
	// an expression nobody has looked at is exactly when this should stop the build.
	it('covers every expression in the record', () => {
		const known = new Set(EXPRESSIONS.map(([expression]) => expression));
		const unknown = [
			...new Set(
				Object.values(record.packages)
					.map((entry) => entry.spdx)
					.filter((spdx): spdx is string => spdx !== undefined && !known.has(spdx)),
			),
		];
		expect(unknown).toEqual([]);
	});

	// Whatever comes out has to be usable as a heading and as an anchor. An operator surviving
	// into a term means something was not split and would file a package under a name that is
	// not a licence.
	it('leaves no operator or bracket inside a term', () => {
		for (const [expression] of EXPRESSIONS) {
			for (const term of licenseTerms(expression)) {
				expect(term).not.toMatch(/[()/]|\bOR\b|\bAND\b/);
				expect(term.trim()).toBe(term);
				expect(term).not.toBe('');
			}
		}
	});
});

describe('the record', () => {
	it('uses the package metadata schema consumed by the directory pages', () => {
		expect(record.version).toBe(4);
	});
	it('says where a license came from when the package did not declare it', () => {
		expect(licenseOf({ spdx: 'MIT' })).toBe('MIT');
		expect(licenseOf({ spdx: 'MIT', asserted: true })).toBe('MIT (asserted)');
		expect(licenseOf({})).toBe('not declared');
	});

	// `cms licenses` refuses to finish while a package has no discoverable terms. This is that
	// guarantee restated where the site would otherwise be the thing publishing the gap.
	it('has a license for every package it lists', () => {
		const missing = Object.entries(record.packages)
			.filter(([, entry]) => !entry.spdx)
			.map(([purl]) => purl);
		expect(missing).toEqual([]);
	});

	it('explains every package with a resolved path that ends at that package', () => {
		for (const [purl, entry] of Object.entries(record.packages)) {
			const origins = Object.entries(entry.origins ?? {});
			expect(origins.length, purl).toBeGreaterThan(0);
			for (const [root, path] of origins) {
				expect(root).not.toBe('');
				expect(path.at(-1)).toBe(purl);
				for (const node of path) {
					if (node.startsWith('workspace:')) continue;
					expect(record.packages[node], `${purl} through ${node}`).toBeDefined();
				}
			}
		}
	});

	// The CDN route puts the fanout back on. Spelling it here instead would publish the bucket's
	// layout as part of every link, which is the thing the image route already avoids.
	it('addresses a text by its content id alone, without the storage fanout', () => {
		const cid = 'ad4a608d8ded9e7ead3dcad841f25be0';
		expect(textUrl('https://cdn.example', cid)).toBe(`https://cdn.example/license/${cid}.txt`);
		expect(textUrl('https://cdn.example', cid)).not.toContain('/ad/4a/');
	});

	it('names the repository in the line every route opens with', () => {
		expect(HEADER).toContain('MIT License');
		expect(HEADER).toContain(URLS.source);
	});
});
