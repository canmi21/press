import { articles as builtArticles } from 'virtual:articles';
import { compilePage } from './compile';
import type { Article, CompiledPage, Page } from './types';

// Articles arrive already compiled through virtual:articles. Top-level files
// (contents/index.md) are standalone pages and never enter the article stream.
const pageRaws = import.meta.glob('$contents/*.md', {
	query: '?raw',
	import: 'default',
	eager: true,
}) as Record<string, string>;

const articlesByPath = new Map(builtArticles.map((article) => [article.path, article]));

// Article list (publish-date desc), shared by the sitemap, /llms.txt and
// per-article markdown.
export function getArticles(): Article[] {
	return builtArticles;
}

export function getArticle(path: string): Article | undefined {
	return articlesByPath.get(path);
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
