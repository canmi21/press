import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';
import { pickUrls } from '@canmi/urls';
import { HEADER, TEXT_HEADERS, find, licenseOf, textUrl } from '$lib/licenses';
import type { RequestHandler } from './$types';

export const prerender = false;

/**
 * One package: what it is, who wrote it, and the license text it shipped.
 *
 * The path is the purl with its scheme and escaping removed -- `/licenses/npm/@sveltejs/kit@2.0.0.txt`
 * -- so a package name that contains a slash needs no encoding of its own. The rest parameter
 * is what makes that work, and is the same shape the article routes use for the same reason.
 */
export const GET: RequestHandler = async ({ params }) => {
	const route = params.package.replace(/\.txt$/, '');
	const found = find(route);
	if (!found) error(404, 'Not found');

	const preamble = [
		HEADER,
		'',
		found.purl,
		`License: ${licenseOf(found.package)}`,
		...(found.package.authors?.length
			? [`Authors: ${found.package.authors.map(({ name }) => name).join(', ')}`]
			: []),
		...(found.package.asserted
			? [
					'',
					'This package declares no license of its own. The expression above was read off',
					'what it ships and recorded in data/licenses.yaml, with the evidence.',
				]
			: []),
	].join('\n');

	const texts = found.package.texts ?? [];
	if (texts.length === 0) {
		return new Response(`${preamble}\n\nNo license text is distributed with this package.\n`, {
			headers: TEXT_HEADERS,
		});
	}

	// At most a handful per package -- three is the widest seen, for a crate offering a choice
	// of three licenses -- so these are fetched together rather than in sequence.
	const cdn = pickUrls(dev).cdn;
	const bodies = await Promise.all(
		texts.map(async (text) => {
			const upstream = await fetch(textUrl(cdn, text.cid));
			const body = upstream.ok ? await upstream.text() : 'This text has not been published yet.\n';
			return `--- ${text.name} ---\n\n${body}`;
		}),
	);

	return new Response(`${preamble}\n\n${bodies.join('\n')}`, { headers: TEXT_HEADERS });
};
