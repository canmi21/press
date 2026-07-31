import { toHtml } from 'hast-util-to-html';
import { toHast } from 'mdast-util-to-hast';
import { toString as mdastToString } from 'mdast-util-to-string';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import { unified } from 'unified';
import { parse as parseYaml, stringify as stringifyYaml } from 'yaml';
import { highlight } from '$lib/server/highlight';
import type { ArticleMeta } from '$lib/article.svelte';
import type { Block, Compiled, CompiledPage, InlineSegment, PageBlock, TocEntry } from './types';
import type { TextDirective } from 'mdast-util-directive';
import type { Heading, Image as MdImage, Paragraph, Root, RootContent } from 'mdast';

// Mirrors the host in image.svelte so feed/markdown image URLs resolve the same
// way the rendered page does.
const IMAGE_CDN = 'https://cdn.canmi.net/image/';

const parser = unified()
	.use(remarkParse)
	.use(remarkFrontmatter, ['yaml'])
	.use(remarkGfm)
	.use(remarkDirective);

const stringifier = unified().use(remarkStringify, { bullet: '-', fences: true }).use(remarkGfm);

function escapeHtml(value: string): string {
	return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// DLC directives extend markdown with display-only semantics that compile
// differently per target: rich HTML for the page, plain markdown for /llms.txt.
// `:t[text]{...}` is a styled span; `:link[platform]{to=handle}` a social link.
type DirectiveAttrs = Record<string, string | null | undefined>;

const SOCIAL: Record<
	string,
	{ href: (handle: string) => string; follow?: (handle: string) => string; newTab: boolean }
> = {
	twitter: {
		href: (h) => `https://x.com/${h}`,
		follow: (h) => `https://twitter.com/intent/follow?screen_name=${h}`,
		newTab: true,
	},
	github: { href: (h) => `https://github.com/${h}`, newTab: true },
	email: { href: (h) => `mailto:${h}`, newTab: false },
};

// `:link[Twitter]{to=canmi21}` resolves to the profile; add the `follow` flag for
// the intent-follow prompt. Unknown platforms fall back to the raw `to` value.
function resolveLink(label: string, attrs: DirectiveAttrs): { href: string; newTab: boolean } {
	const platform = SOCIAL[label.toLowerCase()];
	const handle = attrs.to ?? '';
	if (!platform) return { href: handle, newTab: false };
	const href =
		platform.follow && 'follow' in attrs ? platform.follow(handle) : platform.href(handle);
	return { href, newTab: platform.newTab };
}

// `:t` attributes -> utility classes. font/color carry token names (libs/tokens);
// the rest are boolean flags.
function styleClasses(attrs: DirectiveAttrs): string[] {
	const classes: string[] = [];
	if (attrs.font) classes.push(`font-${attrs.font}`);
	if (attrs.color) classes.push(`text-${attrs.color}`);
	if ('italic' in attrs) classes.push('italic');
	if ('bold' in attrs) classes.push('font-bold');
	if ('underline' in attrs) classes.push('underline');
	if ('nowrap' in attrs) classes.push('whitespace-nowrap');
	return classes;
}

// Render a top-level prose node to HTML. `delete` (gfm strikethrough) maps to
// <s> so the existing .article-body :global(s) styling keeps working; the DLC
// `:t` / `:link` text directives expand to spans / anchors.
function proseHtml(node: RootContent): string {
	const hast = toHast(node, {
		handlers: {
			delete: (state, deleteNode) => ({
				type: 'element',
				tagName: 's',
				properties: {},
				children: state.all(deleteNode),
			}),
			textDirective: (state, directiveNode) => {
				const directive = directiveNode as TextDirective;
				const attrs = (directive.attributes ?? {}) as DirectiveAttrs;
				const children = state.all(directive);
				if (directive.name === 'link') {
					const { href, newTab } = resolveLink(mdastToString(directive), attrs);
					if (newTab) {
						children.push({
							type: 'element',
							tagName: 'span',
							properties: { className: ['sr-only'] },
							children: [{ type: 'text', value: ' (opens in new tab)' }],
						});
					}
					return {
						type: 'element',
						tagName: 'a',
						properties: {
							href,
							className: [
								'text-text-strong',
								'underline',
								'decoration-border',
								'underline-offset-4',
							],
							...(newTab ? { target: '_blank', rel: 'noopener noreferrer' } : {}),
						},
						children,
					};
				}
				return {
					type: 'element',
					tagName: 'span',
					properties: { className: styleClasses(attrs) },
					children,
				};
			},
		},
	});
	return hast ? toHtml(hast) : '';
}

// Lower DLC directives to standard markdown for the text/llms.txt target: `:link`
// becomes a real link; `:t` keeps emphasis where it maps cleanly (bold/italic) and
// is otherwise unwrapped, since the styling is HTML-only.
function lowerDirectives(nodes: RootContent[]): RootContent[] {
	return nodes.flatMap((node) => {
		if ('children' in node && Array.isArray(node.children)) {
			node.children = lowerDirectives(node.children as RootContent[]) as typeof node.children;
		}
		if (node.type === 'textDirective') {
			const directive = node as TextDirective;
			const attrs = (directive.attributes ?? {}) as DirectiveAttrs;
			const children = directive.children as unknown as RootContent[];
			if (directive.name === 'link') {
				const { href } = resolveLink(mdastToString(directive), attrs);
				return [{ type: 'link', url: href, children } as unknown as RootContent];
			}
			if ('bold' in attrs) return [{ type: 'strong', children } as unknown as RootContent];
			if ('italic' in attrs) return [{ type: 'emphasis', children } as unknown as RootContent];
			return children;
		}
		return [node];
	});
}

function proseMarkdown(node: RootContent): string {
	return stringifier.stringify({ type: 'root', children: [node] } as Root).trim();
}

// `## Intro {#getting-started}` -> { text: 'Intro', slug: 'getting-started' }.
// Falls back to a slug derived from the text when no explicit id is present.
function headingParts(node: Heading): { slug: string; text: string } {
	const raw = mdastToString(node).trim();
	const explicit = raw.match(/^(.*?)\s*\{#([\w-]+)\}$/);
	if (explicit) return { text: explicit[1].trim(), slug: explicit[2] };
	const slug = raw
		.toLowerCase()
		.replace(/[^\w]+/g, '-')
		.replace(/^-+|-+$/g, '');
	return { text: raw, slug };
}

function imageOf(node: RootContent): MdImage | null {
	if (
		node.type === 'paragraph' &&
		node.children.length === 1 &&
		node.children[0].type === 'image'
	) {
		return node.children[0];
	}
	return null;
}

export async function compile(raw: string, url: string): Promise<Compiled> {
	const tree = parser.parse(raw) as Root;
	let meta: ArticleMeta | undefined;
	const blocks: Block[] = [];
	const toc: TocEntry[] = [];
	const feed: string[] = [];
	const md: string[] = [];
	const text: string[] = [];

	for (const node of tree.children) {
		if (node.type === 'yaml') {
			meta = parseYaml(node.value) as ArticleMeta;
			continue;
		}

		if (node.type === 'heading') {
			const { slug, text: heading } = headingParts(node);
			blocks.push({ type: 'heading', depth: node.depth, slug, text: heading });
			toc.push({ slug, text: heading, depth: node.depth });
			feed.push(`<h${node.depth} id="${slug}">${escapeHtml(heading)}</h${node.depth}>`);
			md.push(`${'#'.repeat(node.depth)} ${heading}`);
			text.push(heading);
			continue;
		}

		if (node.type === 'code') {
			const lang = node.lang ?? 'text';
			if (lang === 'svg-canvas') {
				const title = node.meta?.trim() || meta?.title || 'diagram';
				blocks.push({ type: 'svgCanvas', svg: node.value, title });
				feed.push(`<p><em>[Diagram: ${escapeHtml(title)} — view at ${url}]</em></p>`);
				md.push(`> [diagram: ${title} — ${url}]`);
				continue;
			}
			blocks.push({
				type: 'code',
				lang,
				html: await highlight(node.value, lang),
				code: node.value,
			});
			feed.push(`<pre><code>${escapeHtml(node.value)}</code></pre>`);
			md.push(`\`\`\`${lang}\n${node.value}\n\`\`\``);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'linkcard') {
			const attrs = node.attributes ?? {};
			const tone: 'light' | 'dark' | undefined =
				attrs.tone === 'dark' ? 'dark' : attrs.tone === 'light' ? 'light' : undefined;
			const card = { src: attrs.src ?? '', url: attrs.url ?? '', title: attrs.title ?? '', tone };
			blocks.push({ type: 'linkcard', ...card });
			feed.push(`<p><a href="${card.url}">${escapeHtml(card.title)}</a></p>`);
			md.push(`[${card.title}](${card.url})`);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'placeholder') {
			const { kind, ...rest } = node.attributes ?? {};
			const meta: Record<string, string> = {};
			for (const [key, value] of Object.entries(rest)) if (value != null) meta[key] = value;
			const label = kind ?? '';
			const metaText = Object.entries(meta)
				.map(([k, v]) => ` ${k}="${v}"`)
				.join('');
			blocks.push({ type: 'placeholder', kind: label, meta });
			feed.push(
				`<pre>::${escapeHtml(label)}${Object.entries(meta)
					.map(([k, v]) => `\n${k} = "${escapeHtml(v)}"`)
					.join('')}</pre>`,
			);
			md.push(`> [placeholder ::${label}${metaText}]`);
			continue;
		}

		const image = imageOf(node);
		if (image) {
			const alt = image.alt ?? '';
			const absolute = `${IMAGE_CDN}${image.url}`;
			blocks.push({ type: 'image', src: image.url, alt });
			feed.push(`<p><img src="${absolute}" alt="${escapeHtml(alt)}" /></p>`);
			md.push(`![${alt}](${absolute})`);
			if (alt) text.push(alt);
			continue;
		}

		blocks.push({ type: 'prose', html: proseHtml(node) });
		feed.push(proseHtml(node));
		md.push(proseMarkdown(node));
		const plain = mdastToString(node).trim();
		if (plain) text.push(plain);
	}

	if (!meta) throw new Error(`missing frontmatter: ${url}`);

	// Provenance/recency rides as frontmatter on the full article markdown; the
	// index (/llms.txt) stays metadata-free per convention.
	const frontmatter = stringifyYaml({
		title: meta.title,
		created: meta.created,
		lastmod: meta.lastmod,
		lang: meta.lang,
		source: url,
	});

	return {
		meta,
		toc,
		blocks,
		feed: feed.join('\n'),
		markdown: `---\n${frontmatter}---\n\n# ${meta.title}\n\n${meta.description}\n\n${md.join('\n\n')}\n`,
		text: text.join('\n\n'),
	};
}

// Split a paragraph into inline segments at `:link` boundaries: text runs (incl.
// `:t` styling) become dead HTML, each `:link` a live segment the route renders
// with its icon. Keeps the {@html} surface minimal, mirroring article blocks.
function inlineSegments(node: Paragraph): InlineSegment[] {
	const segments: InlineSegment[] = [];
	let run: string[] = [];
	const flush = () => {
		if (run.length) {
			segments.push({ type: 'html', html: run.join('') });
			run = [];
		}
	};
	for (const child of node.children) {
		if (child.type === 'textDirective' && child.name === 'link') {
			flush();
			const attrs = (child.attributes ?? {}) as DirectiveAttrs;
			const label = mdastToString(child);
			const platform = label.toLowerCase();
			const { href, newTab } = resolveLink(label, attrs);
			segments.push({
				type: 'link',
				icon: platform in SOCIAL ? (platform as 'twitter' | 'github' | 'email') : undefined,
				href,
				label,
				newTab,
			});
		} else {
			run.push(proseHtml(child as RootContent));
		}
	}
	flush();
	return segments;
}

// A standalone page (e.g. the homepage at contents/homepage.md). Unlike an article
// it carries free-form frontmatter and produces blocks for the route to render
// plus the DLC-lowered prose body (getPage wraps it into the served document).
export function compilePage(raw: string): CompiledPage {
	const tree = parser.parse(raw) as Root;
	let meta: Record<string, string> = {};
	const blocks: PageBlock[] = [];
	const bodyNodes: RootContent[] = [];

	for (const node of tree.children) {
		if (node.type === 'yaml') {
			meta = (parseYaml(node.value) ?? {}) as Record<string, string>;
			continue;
		}
		bodyNodes.push(node);
		blocks.push(
			node.type === 'paragraph'
				? { type: 'p', segments: inlineSegments(node) }
				: { type: 'html', html: proseHtml(node) },
		);
	}

	const body = stringifier
		.stringify({ type: 'root', children: lowerDirectives(bodyNodes) } as Root)
		.trim();
	return { meta, blocks, body };
}
