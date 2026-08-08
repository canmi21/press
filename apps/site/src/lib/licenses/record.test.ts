import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { describe, expect, it } from 'vitest';
import { HEADER, licenseOf, routePath, routeTable, textUrl, type LicenseRecord } from './record.ts';

// The committed record itself, read from disk rather than through the virtual module: these
// are claims about the real dependency tree, and the point is that they hold for it.
const RECORD = fileURLToPath(new URL('../../../../../data/build/licenses.json', import.meta.url));
const record = JSON.parse(readFileSync(RECORD, 'utf8')) as LicenseRecord;

describe('route paths', () => {
	it('drops the scheme and the escaping a purl only needs to stay one string', () => {
		expect(routePath('pkg:cargo/serde@1.0.219')).toBe('cargo/serde@1.0.219');
		expect(routePath('pkg:npm/%40sveltejs/kit@2.0.0')).toBe('npm/@sveltejs/kit@2.0.0');
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

describe('the record', () => {
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

	it('fans a text out over the same two levels the store writes', () => {
		expect(textUrl('https://cdn.example', 'ad4a608d8ded9e7ead3dcad841f25be0')).toBe(
			'https://cdn.example/license/ad/4a/ad4a608d8ded9e7ead3dcad841f25be0.txt',
		);
	});

	it('names the repository in the line every route opens with', () => {
		expect(HEADER).toContain('MIT License');
		expect(HEADER).toContain(URLS.source);
	});
});
