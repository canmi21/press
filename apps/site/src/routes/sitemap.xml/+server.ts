import { URLS } from '@canmi/urls';
import { getArticles } from '$lib/content';
import type { RequestHandler } from './$types';

// Generated per request so changefreq/priority reflect staleness at crawl time,
// not at build time.

type Entry = {
	loc: string;
	lastmod: string;
	changefreq: string;
	priority: string;
};

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

// Non-article routes have no modification time of their own, so the build
// timestamp (baked via Vite define) stands in for "last changed".
const staticEntries: Entry[] = [
	{
		loc: `${URLS.apps.production.site}/`,
		lastmod: import.meta.env.VITE_BUILD_TIME,
		changefreq: 'daily',
		priority: '1.0',
	},
];

function changefreq(ageMs: number): string {
	if (ageMs < HOUR) return 'hourly';
	if (ageMs < DAY) return 'daily';
	if (ageMs < 7 * DAY) return 'weekly';
	if (ageMs < 30 * DAY) return 'monthly';
	if (ageMs < 365 * DAY) return 'yearly';
	return 'never';
}

function priority(ageMs: number): string {
	if (ageMs < 30 * DAY) return '0.9';
	if (ageMs < 90 * DAY) return '0.8';
	if (ageMs < 180 * DAY) return '0.7';
	if (ageMs < 365 * DAY) return '0.6';
	return '0.5';
}

export const GET: RequestHandler = async () => {
	const now = Date.now();
	// getArticles() is already publish-date desc, the order we want here.
	const articles = await getArticles();

	const entries: Entry[] = [
		...staticEntries,
		...articles.map((article) => {
			const ageMs = now - Date.parse(article.meta.lastmod);
			return {
				loc: article.url,
				lastmod: article.meta.lastmod,
				changefreq: changefreq(ageMs),
				priority: priority(ageMs),
			};
		}),
	];

	const items = entries
		.map((e) => {
			const parts = [
				`\t\t<loc>${e.loc}</loc>`,
				`\t\t<lastmod>${e.lastmod}</lastmod>`,
				`\t\t<changefreq>${e.changefreq}</changefreq>`,
				`\t\t<priority>${e.priority}</priority>`,
			];
			return `\t<url>\n${parts.join('\n')}\n\t</url>`;
		})
		.join('\n');

	const body = `<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/xsl" href="/sitemap.xsl"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${items}
</urlset>
`;
	return new Response(body, {
		headers: {
			'Content-Type': 'application/xml; charset=utf-8',
			'Cache-Control': 'public, max-age=300, s-maxage=300',
		},
	});
};
