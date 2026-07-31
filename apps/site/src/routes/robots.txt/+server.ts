import { robotsTxt } from '@canmi/robots';
import { URLS } from '@canmi/urls';
import type { RequestHandler } from './$types';

export const prerender = true;

export const GET: RequestHandler = () =>
	new Response(
		robotsTxt({
			// `/@/` is this site's internal namespace; the other two are infrastructure paths
			// Cloudflare answers on every zone and that have nothing to index.
			disallow: ['/@/', '/cgi-bin/', '/cdn-cgi/'],
			sitemap: `${URLS.apps.production.site}/sitemap.xml`,
		}),
		{ headers: { 'Content-Type': 'text/plain; charset=utf-8' } },
	);
