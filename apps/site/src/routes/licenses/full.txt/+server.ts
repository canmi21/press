import { dev } from '$app/environment';
import { pickUrls } from '@canmi/urls';
import { HEADER, TEXT_HEADERS, fullUrl } from '$lib/licenses';
import type { RequestHandler } from './$types';

// Not prerendered, because prerendering means fetching the CDN during the build and CI
// compiles rather than derives. See spec/architecture/data.md.
export const prerender = false;

/**
 * The whole attribution notice: every package, with every license text in full.
 *
 * This is the artefact the permissive licenses actually ask for -- the copyright notices and
 * permission texts of everything being distributed, in one fetch. It is assembled by
 * `cms licenses` and published as a single object, because building it here would mean the
 * Worker fetching several hundred objects to concatenate them on every request.
 *
 * Buffered rather than streamed. It is a few megabytes and it is answered from the edge cache
 * almost every time; a hand-assembled stream to prepend one line would be the more delicate
 * code for no gain a reader could notice.
 */
export const GET: RequestHandler = async () => {
	const upstream = await fetch(fullUrl(pickUrls(dev).cdn));
	if (!upstream.ok) {
		// The notice has not been synced to the bucket yet. A missing published object is a
		// known state here rather than a fault, so it is reported as one.
		const status = upstream.status === 404 ? 404 : 502;
		return new Response(`${HEADER}\n\nThe full notice has not been published yet.\n`, {
			status,
			headers: TEXT_HEADERS,
		});
	}

	return new Response(`${HEADER}\n\n${await upstream.text()}`, { headers: TEXT_HEADERS });
};
