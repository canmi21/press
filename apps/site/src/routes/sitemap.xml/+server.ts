import { URLS } from '@canmi/urls';
import { getArticles } from '$lib/content';
import { sitemapViews } from '$lib/content/sitemap';
import type { Alternate } from '$lib/content/types';
import { licenseDirectory, packageRows } from '$lib/licenses/directory';
import type { RequestHandler } from './$types';

// Generated per request so changefreq/priority reflect staleness at crawl time,
// not at build time.

type Entry = {
	loc: string;
	lastmod: string;
	changefreq: string;
	priority: string;
	alternates?: Alternate[];
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

/**
 * The licence surface, down to the directories and no further.
 *
 * The line is drawn at the individual package. A directory answers a question somebody asks --
 * what is Apache-licensed here, what comes from crates.io -- while one package page answers a
 * question nobody searches for and there are several hundred of them, which would make the
 * dependency tree the bulk of this site's sitemap. Package pages stay `noindex, follow`: still
 * walked, so the links out of them count, never entered on their own.
 *
 * Derived from the record rather than written out. The set of licence terms is whatever the
 * tree currently resolves to, so a hand-kept list would silently miss the page created by the
 * twenty-sixth licence to appear.
 *
 * The build time is the right lastmod for all of them: the record is baked into the bundle, so
 * a rebuild is exactly when any of these pages last changed.
 */
function licenseEntries(): Entry[] {
	const site = URLS.apps.production.site;
	const at = (path: string, weight: string): Entry => ({
		loc: `${site}${path}`,
		lastmod: import.meta.env.VITE_BUILD_TIME,
		changefreq: 'monthly',
		priority: weight,
	});

	const registries = [...new Set(packageRows().map(({ registry }) => registry))].toSorted();

	return [
		at('/licenses', '0.3'),
		at('/licenses/pkgs', '0.3'),
		...registries.map((registry) => at(`/licenses/pkgs/${registry}`, '0.2')),
		...licenseDirectory().map(({ slug }) => at(`/licenses/${slug}`, '0.2')),
	];
}

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
		...licenseEntries(),
		...articles.flatMap((article) => {
			const ageMs = now - Date.parse(article.meta.lastmod);
			return sitemapViews(article).map(({ loc, alternates }) => ({
				loc,
				lastmod: article.meta.lastmod,
				changefreq: changefreq(ageMs),
				priority: priority(ageMs),
				alternates,
			}));
		}),
	];

	const items = entries
		.map((e) => {
			const parts = [
				`\t\t<loc>${e.loc}</loc>`,
				...(e.alternates ?? []).map(
					(alternate) =>
						`\t\t<xhtml:link rel="alternate" hreflang="${alternate.languageTag}" href="${alternate.href}" />`,
				),
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
