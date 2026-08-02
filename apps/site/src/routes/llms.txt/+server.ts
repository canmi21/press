import { getArticles } from '$lib/content';
import { URLS } from '@canmi/urls';
import { site } from '$lib/site';
import type { RequestHandler } from './$types';

export const prerender = false;

// The site's nature, distinct from site.tagline (which is the RSS description).
const DESCRIPTION =
	"A developer's personal space for daily life, engineering, and research — part FAQ, part archive, a place to record and collect what's worth keeping.";

// Collapse YAML-folded whitespace so a subtitle stays on one line.
function oneline(value: string): string {
	return value.replace(/\s+/g, ' ').trim();
}

// LLM entry point: the site name, a one-line nature blurb, then link sections,
// each link as [name](url) with a short note. Site collects the homepage,
// sitemap and feed; Writing lists every article as title -> clean markdown with
// the subtitle. See https://llmstxt.org/.
export const GET: RequestHandler = async ({ locals }) => {
	const web = URLS.apps.production.site;
	const articles = await getArticles();
	const code = locals.locale?.code ?? 'mw';
	const body = [
		`# ${site.name}`,
		'',
		`> ${DESCRIPTION}`,
		'',
		'## Site',
		'',
		`- [Homepage](${web}/homepage.md): The landing page as clean markdown — a short introduction and the way into the site.`,
		`- [Sitemap](${web}/sitemap.xml): Every page and article with last-modified hints, for crawlers.`,
		`- [Atom feed](${web}/atom.xml): Full-text feed of all writing, newest first, for subscribers.`,
		'',
		'## Writing',
		'',
		...articles.map((article) => {
			const meta = article.views[code].meta;
			return `- [${meta.title}](${article.url}.md): ${oneline(meta.subtitle)}`;
		}),
	].join('\n');
	return new Response(`${body}\n`, {
		headers: {
			'Content-Type': 'text/plain; charset=utf-8',
			'Cache-Control': 'private, no-store',
		},
	});
};
