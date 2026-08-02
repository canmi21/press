import { generateAtomFeed } from 'feedsmith';
import { URLS } from '@canmi/urls';
import { getArticles } from '$lib/content';
import { site } from '$lib/site';
import type { RequestHandler } from './$types';

export const prerender = false;

const SITE = URLS.apps.production.site;
const RES = URLS.apps.production.cdn;

export const GET: RequestHandler = async ({ locals }) => {
	const code = locals.locale?.code ?? 'mw';
	const prepared = (await getArticles())
		.map((article) => {
			const view = article.views[code];
			return {
				id: article.url,
				title: view.meta.title,
				updated: new Date(view.meta.lastmod),
				published: new Date(view.meta.created),
				summary: view.meta.description,
				content: view.feed,
				links: [{ href: view.canonical }],
				lang: view.languageTag,
			};
		})
		.toSorted((a, b) => b.updated.getTime() - a.updated.getTime());

	let xml = generateAtomFeed({
		id: site.feed.id,
		title: site.name,
		subtitle: site.tagline,
		updated: prepared[0]?.updated ?? new Date(),
		authors: [{ name: site.author.name, email: site.author.email }],
		icon: `${RES}/favicon.svg`,
		links: [
			{ href: `${SITE}/atom.xml`, rel: 'self' },
			{ href: `${SITE}/`, rel: 'alternate' },
		],
		generator: { text: 'feedsmith', uri: URLS.external.feedsmith },
		entries: prepared.map(({ lang: _lang, ...rest }) => rest),
	});

	xml = xml.replace(
		'<feed xmlns="http://www.w3.org/2005/Atom">',
		`<feed xmlns="http://www.w3.org/2005/Atom">
  <description>${site.feed.followDescription}</description>`,
	);

	xml = xml.replace(/<content>/g, '<content type="html">');

	let entryIdx = 0;
	xml = xml.replace(/<entry>/g, () => {
		const lang = prepared[entryIdx]?.lang ?? 'zh';
		entryIdx += 1;
		return `<entry xml:lang="${lang}">`;
	});

	return new Response(xml, {
		headers: {
			'Content-Type': 'application/atom+xml; charset=utf-8',
			'Cache-Control': 'private, no-store',
		},
	});
};
