import { URLS } from '@canmi/urls';
import { HEADER, TEXT_HEADERS, licenseOf, packages } from '$lib/licenses';
import type { RequestHandler } from './$types';

export const prerender = true;

/**
 * The index: every dependency on one line, and nothing that needs fetching.
 *
 * Tab separated rather than aligned into columns. The purls run to fifty characters and the
 * author lists longer, so padding would push the interesting fields off the side of a
 * terminal; a tab is what `cut` and `awk` already expect.
 */
export const GET: RequestHandler = () => {
	const web = URLS.apps.production.site;
	const rows = packages().map(([purl, entry]) => {
		const authors = entry.authors?.join(', ') ?? '';
		const texts = entry.texts?.length ?? 0;
		const shipped = texts === 0 ? 'no text' : `${texts} text${texts === 1 ? '' : 's'}`;
		return [purl, licenseOf(entry), shipped, authors].join('\t');
	});

	const body = [
		HEADER,
		'',
		`${rows.length} third-party packages: everything the deployed Workers ship, plus every crate`,
		"this repository's own tooling is built from. Workspace packages are not listed -- they are",
		'this project rather than something it credits.',
		'',
		`Every license text in full:  ${web}/licenses/full.txt`,
		`One package:                 ${web}/licenses/{type}/{name}@{version}.txt`,
		'',
		'Columns are tab separated: package, license, whether a text is published, authors.',
		'A license marked "asserted" was not declared by the package; it was read off what the',
		'package ships and recorded in data/licenses.yaml, with the evidence.',
		'',
		...rows,
	].join('\n');

	return new Response(`${body}\n`, { headers: TEXT_HEADERS });
};
