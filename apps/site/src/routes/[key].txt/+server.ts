import { site } from '$lib/site';
import { error } from '@sveltejs/kit';
import type { EntryGenerator, RequestHandler } from './$types';

export const prerender = true;

/**
 * The IndexNow ownership proof: `/<key>.txt`, containing that key and nothing else.
 *
 * **The path is derived from the config rather than written as a directory name.** The key is a
 * value in `site.config.yaml`, and a folder named after it would be a second copy that no test
 * compares -- rotate the key and the file would still be served under the old name, which fails
 * as a 403 on every submission and says nothing about why. A dynamic segment checked against the
 * config cannot drift: one edit moves both the address and the contents.
 *
 * A search engine fetches this on **every** submission, not once. There is no verification step
 * that completes; the file is the proof and has to stay up for as long as the key is in use. See
 * spec/indexing.md.
 *
 * Prerendered to exactly one entry. Any other `<something>.txt` is not this file and must not
 * answer as if it were -- the static routes beside this one (`robots.txt`, `llms.txt`,
 * `licenses.txt`) take precedence on their own names, and nothing else is claimed.
 */
export const entries: EntryGenerator = () => [{ key: site.indexnow }];

export const GET: RequestHandler = ({ params }) => {
	if (params.key !== site.indexnow) error(404, 'Not Found');
	return new Response(site.indexnow, {
		headers: { 'Content-Type': 'text/plain; charset=utf-8' },
	});
};
