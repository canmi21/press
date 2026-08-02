import { URLS } from '@canmi/urls';
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
import type { Resolved } from '$lib/assets';
import type { ArticleMeta } from '$lib/article.svelte';
import { assertLanguageTag } from '../locale.ts';
import type { Block, Compiled, CompiledPage, InlineSegment, PageBlock, TocEntry } from './types.ts';
import type { TextDirective } from 'mdast-util-directive';
import type { Heading, Image as MdImage, Paragraph, Root, RootContent } from 'mdast';

// Feed and markdown targets need absolute image URLs, and they must resolve the same way the
// rendered page does. Both now read the host from libs/urls rather than each spelling it out.
const IMAGE_CDN = `${URLS.apps.production.cdn}/image/`;

const parser = unified()
	.use(remarkParse)
	.use(remarkFrontmatter, ['yaml'])
	.use(remarkGfm)
	.use(remarkDirective);

const stringifier = unified().use(remarkStringify, { bullet: '-', fences: true }).use(remarkGfm);

/**
 * Escape a value for either HTML text or a double-quoted attribute.
 *
 * Quotes are escaped because some of these land in `alt="..."`, and a description that
 * mentions a path or a window title routinely contains one -- 15 of the 24 written so far do.
 * Without this the attribute simply ends early, which is malformed output today and an
 * injection point the moment any of this text stops being ours.
 */
function escapeHtml(value: string): string {
	return value
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
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
		href: (h) => `${URLS.external.social.x}/${h}`,
		follow: (h) => `${URLS.external.social.twitterIntent}?screen_name=${h}`,
		newTab: true,
	},
	github: { href: (h) => `${URLS.external.github.web}/${h}`, newTab: true },
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
							// hast stores space-separated token lists as arrays. Writing this as one
							// string produced `rel="noopener,noreferrer"` in the output, which is a
							// single unrecognised token and so left the link unprotected.
							...(newTab ? { target: '_blank', rel: ['noopener', 'noreferrer'] } : {}),
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

/**
 * What an image is described as, preferring what the article said.
 *
 * The manifest's description belongs to the picture and is written once, so a directive that
 * says nothing still gets one -- including articles written before any description existed,
 * which pick it up on the next build. Writing `alt` overrides it for one page's context, and
 * an explicit `alt=""` means decorative and is honoured.
 *
 * There used to be a second rule here for markdown images, which cannot express "decorative"
 * at all: `![](x)` parses to an empty alt meaning only "unwritten". Local images all go
 * through the directive now, so that distinction has nothing left to describe.
 */
function altFor(written: string | null | undefined, resolved: Resolved | null): string {
	if (written != null) return written;
	return resolved?.description ?? '';
}

/** Widescreen, because the reason to crop at all is usually to make a row of images agree. */
const DEFAULT_CROP = '16 / 9';

/** Everything `object-position` is allowed to be here. Centred unless told otherwise. */
const ALIGNMENTS = ['center', 'top', 'bottom', 'left', 'right'] as const;

/**
 * `W:H` as a CSS `aspect-ratio`.
 *
 * Malformed input throws rather than falling back. A silent default would render a crop
 * nobody asked for, and a typo in a ratio is invisible in a way a missing image is not --
 * the page still looks deliberate.
 */
function cropRatio(value: string | null | undefined, url: string): string {
	if (value == null) return DEFAULT_CROP;
	const match = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(value.trim());
	if (!match || Number(match[1]) <= 0 || Number(match[2]) <= 0) {
		throw new Error(`::image ratio must be W:H with positive numbers, got "${value}": ${url}`);
	}
	return `${match[1]} / ${match[2]}`;
}

function cropAlign(value: string | null | undefined, url: string): string | undefined {
	if (value == null) return undefined;
	const wanted = value.trim().toLowerCase();
	if (!(ALIGNMENTS as readonly string[]).includes(wanted)) {
		throw new Error(
			`::image align must be one of ${ALIGNMENTS.join(', ')}, got "${value}": ${url}`,
		);
	}
	return wanted === 'center' ? undefined : wanted;
}

export type CompileContext = {
	resolveAsset: (reference: string) => Resolved | null;
	highlight: (code: string, lang: string) => Promise<string>;
	/** Present only while reading the source view; translations inherit validated frontmatter. */
	sourceFile?: string;
};

export async function compile(
	raw: string,
	url: string,
	{ resolveAsset, highlight, sourceFile }: CompileContext,
): Promise<Compiled> {
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
			if (sourceFile) assertLanguageTag(meta?.lang, sourceFile);
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
			// A card's cover is an asset like any other, so it gets the same variants and
			// placeholder. Resolving here rather than in the component keeps the manifest --
			// every base64 preview in it -- out of the client bundle.
			blocks.push({ type: 'linkcard', ...card, ...resolveAsset(card.src) });
			feed.push(`<p><a href="${card.url}">${escapeHtml(card.title)}</a></p>`);
			md.push(`[${card.title}](${card.url})`);
			continue;
		}

		// `::image` is the cropped presentation of an asset. Plain `![]()` stays uncropped, so
		// writing this directive is itself the request to crop -- which is why the defaults
		// here are a ratio and an alignment rather than "no change".
		if (node.type === 'leafDirective' && node.name === 'image') {
			const attrs = node.attributes ?? {};
			const src = attrs.src ?? '';
			const crop = cropRatio(attrs.ratio, url);
			const align = cropAlign(attrs.align, url);
			const absolute = `${IMAGE_CDN}/image/${src}`;
			const resolved = resolveAsset(src);
			const alt = altFor(attrs.alt, resolved);

			blocks.push({ type: 'image', src, alt, crop, align, ...resolved });
			// The crop does not survive into the feed or the markdown target, and should not:
			// neither runs a layout, and a crop is how a page shows an image rather than
			// anything the image says.
			feed.push(`<p><img src="${absolute}" alt="${escapeHtml(alt)}" /></p>`);
			md.push(`![${alt}](${absolute})`);
			if (alt) text.push(alt);
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
			const absolute = `${IMAGE_CDN}${image.url}`;
			// Feed and markdown get one plain URL, because neither can express a srcset and
			// both are read by things that will not run a layout.
			const resolved = resolveAsset(image.url);
			const alt = altFor(image.alt, resolved);
			blocks.push({ type: 'image', src: image.url, alt, ...resolved });
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
