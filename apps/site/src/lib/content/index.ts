import { URLS } from '@canmi/urls';
import { compile, compilePage } from './compile';
import type { Article, CompiledPage, Page } from './types';

// Globbed at build; compilation is memoized and, since every consuming route is
// prerendered, runs only at build time. Posts are category-nested
// (contents/<category>/<slug>.md); top-level files (contents/index.md) are pages
// and never enter the article stream (feed, sitemap, llms.txt index).
const postRaws = import.meta.glob('$contents/*/*.md', {
	query: '?raw',
	import: 'default',
	eager: true,
}) as Record<string, string>;

const pageRaws = import.meta.glob('$contents/*.md', {
	query: '?raw',
	import: 'default',
	eager: true,
}) as Record<string, string>;

function pathOf(file: string): string {
	return file.replace(/^.*\/contents\//, '').replace(/\.md$/, '');
}

let cache: Promise<Article[]> | undefined;

function build(): Promise<Article[]> {
	if (!cache) {
		cache = Promise.all(
			Object.entries(postRaws).map(async ([file, raw]) => {
				const path = pathOf(file);
				const url = `${URLS.apps.production.site}/${path}`;
				return { ...(await compile(raw, url)), path, url };
			}),
		).then((list) =>
			list.toSorted((a, b) => Date.parse(b.meta.created) - Date.parse(a.meta.created)),
		);
	}
	return cache;
}

// Article list (publish-date desc), shared by the sitemap, /llms.txt and
// per-article markdown.
export async function getArticles(): Promise<Article[]> {
	return build();
}

export async function getArticle(path: string): Promise<Article | undefined> {
	return (await build()).find((article) => article.path === path);
}

const pages = new Map<string, Page>();

// Map a compiled page to its standalone /<slug>.md document, mirroring the
// /llms.txt shape so the page reads as a small self-contained index:
//   # <Slug>                       capitalized slug (homepage -> Homepage)
//   > <summary>                    frontmatter `summary`, the page's purpose
//   ## <title>                     frontmatter `title` as an identity section,
//   <description>                  carrying frontmatter `description`
//   ## Bio                         the prose body
//   <body>
function pageDocument(path: string, { meta, body }: Pick<CompiledPage, 'meta' | 'body'>): string {
	const name = path.charAt(0).toUpperCase() + path.slice(1);
	const lines = [`# ${name}`, ''];
	if (meta.summary) lines.push(`> ${meta.summary}`, '');
	if (meta.title) {
		lines.push(`## ${meta.title}`, '');
		if (meta.description) lines.push(meta.description, '');
	}
	if (body) lines.push('## Bio', '', body, '');
	return `${lines.join('\n').trim()}\n`;
}

// Compiled standalone page by path (e.g. 'homepage'). Memoized; the underlying
// glob is build-time and consuming routes are prerendered.
export function getPage(path: string): Page | undefined {
	const cached = pages.get(path);
	if (cached) return cached;
	const raw = pageRaws[Object.keys(pageRaws).find((k) => k.endsWith(`/contents/${path}.md`)) ?? ''];
	if (raw === undefined) return undefined;
	const { meta, blocks, body } = compilePage(raw);
	const page: Page = { meta, blocks, markdown: pageDocument(path, { meta, body }) };
	pages.set(path, page);
	return page;
}
