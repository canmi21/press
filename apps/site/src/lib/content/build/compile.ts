import { URLS } from '@canmi/urls';
import { toHtml } from 'hast-util-to-html';
import { toHast, type Handler } from 'mdast-util-to-hast';
import { toString as mdastToString } from 'mdast-util-to-string';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import { unified } from 'unified';
import { parse as parseYaml, stringify as stringifyYaml } from 'yaml';
import type { Resolved } from './assets.ts';
import { assertLanguageTag } from '../../locale/index.ts';
import type {
	Block,
	CardAlign,
	Compiled,
	CompiledPage,
	CargoView,
	CrateRecord,
	InlineSegment,
	PageBlock,
	QuadrantDirection,
	QuadrantItem,
	QuadrantPosition,
	RepoRecord,
	TokeiView,
	TocEntry,
	TweetRecord,
	ArticleMeta,
	ArticleNote,
	ArticleReference,
} from '../types.ts';
import { languageLabel } from './highlight.ts';
import type { ContainerDirective, LeafDirective, TextDirective } from 'mdast-util-directive';

declare module 'mdast-util-directive' {
	interface TextDirectiveData {
		/**
		 * The number a `:fn` was given, written on the node by `numberNotes`.
		 *
		 * On the node rather than in a map beside it, so every target -- page, feed, markdown --
		 * reads the number the counter actually assigned instead of deriving its own.
		 */
		footnoteNumber?: number;
	}
}
import type { Heading, Image as MdImage, Nodes, Paragraph, Root, RootContent } from 'mdast';

// Feed and markdown targets need absolute image URLs, and they must resolve the same way the
// rendered page does. Both now read the host from libs/urls rather than each spelling it out.
const IMAGE_CDN = `${URLS.apps.production.cdn}/image/`;

const parser = unified()
	.use(remarkParse)
	.use(remarkFrontmatter, ['yaml'])
	.use(remarkGfm)
	.use(remarkDirective);

const stringifier = unified().use(remarkStringify, { bullet: '-', fences: true }).use(remarkGfm);

const QUADRANT_DIRECTIONS = ['top', 'right', 'bottom', 'left'] as const;
const QUADRANT_POSITIONS = ['top-left', 'top-right', 'bottom-left', 'bottom-right'] as const;

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

type SocialPlatform = 'twitter' | 'github' | 'email';

const SOCIAL: Record<
	SocialPlatform,
	{ href: (handle: string) => string; follow?: (handle: string) => string; newTab: boolean }
> = {
	twitter: {
		href: (h) => `${URLS.external.social.twitter}/${h}`,
		follow: (h) => `${URLS.external.social.twitterIntent}?screen_name=${h}`,
		newTab: true,
	},
	github: { href: (h) => `${URLS.external.github.web}/${h}`, newTab: true },
	email: { href: (h) => `mailto:${h}`, newTab: false },
};

// `:link[Twitter]{to=canmi21}` resolves to the profile; add the `follow` flag for
// the intent-follow prompt. Unknown platforms fall back to the raw `to` value.
function resolveLink(
	label: string,
	attrs: DirectiveAttrs,
): { href: string; newTab: boolean; platform?: SocialPlatform } {
	const handle = attrs.to ?? '';
	const named = label.toLowerCase();
	const platform =
		named in SOCIAL
			? (named as SocialPlatform)
			: /^[^@\s]+@[^@\s]+$/u.test(handle)
				? ('email' as const)
				: undefined;
	if (!platform) return { href: handle, newTab: false };
	const target = SOCIAL[platform];
	const href = target.follow && 'follow' in attrs ? target.follow(handle) : target.href(handle);
	return { href, newTab: target.newTab, platform };
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

/**
 * Number every `:fn` in one top-level node and collect what it says.
 *
 * A separate pass rather than work done while rendering, because a node is rendered more than
 * once -- the page and the feed each ask for it -- and a counter advanced inside the renderer
 * would count the same note twice. The number is written onto the directive so every target
 * afterwards reads the same one instead of deriving it again.
 *
 * The note's text lives in an attribute, which cannot hold a straight quote: the directive
 * syntax has no escape for one, so the parser drops the whole attribute and leaves a directive
 * that says nothing. That is indistinguishable from a typo at a glance, so it fails the build
 * here rather than rendering an empty marker. `validate.rs` refuses the same shape coming back
 * from a translation.
 */
function numberNotes(node: Nodes, notes: ArticleNote[], source: string): number[] {
	const numbers: number[] = [];
	const visit = (current: Nodes): void => {
		if (current.type === 'textDirective' && (current as TextDirective).name === 'fn') {
			const directive = current as TextDirective;
			const said = ((directive.attributes ?? {}) as DirectiveAttrs).is?.trim();
			const phrase = mdastToString(directive).trim();
			if (!said || !phrase) {
				throw new Error(
					`${source}: :fn is :fn[the words]{is="what they mean"}, and that note cannot contain a straight quote`,
				);
			}
			notes.push({ number: notes.length + 1, phrase, text: said });
			directive.data = { ...directive.data, footnoteNumber: notes.length };
			numbers.push(notes.length);
			return;
		}
		if ('children' in current) {
			for (const child of current.children) visit(child as Nodes);
		}
	};
	visit(node);
	return numbers;
}

function noteNumber(directive: TextDirective): number | undefined {
	return directive.data?.footnoteNumber;
}

function markProseLinks(node: Nodes): void {
	if (node.type === 'link') {
		node.data = {
			...node.data,
			hProperties: {
				...node.data?.hProperties,
				className: ['focus-link', 'spring-underline', 'article-link'],
			},
		};
	}
	if ('children' in node) {
		for (const child of node.children) markProseLinks(child);
	}
}

// Render a top-level prose node to HTML. `delete` (gfm strikethrough) maps to
// <s> so the existing .article-body :global(s) styling keeps working; the DLC
// `:t` / `:link` text directives expand to spans / anchors.
function proseHtml(node: RootContent, newTabNote: string): string {
	markProseLinks(node);
	const hast = toHast(node, {
		handlers: {
			delete: (state, deleteNode) => ({
				type: 'element',
				tagName: 's',
				properties: {},
				children: state.all(deleteNode),
			}),
			textDirective: ((state, directiveNode) => {
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
							children: [{ type: 'text', value: ` (${newTabNote})` }],
						});
					}
					return {
						type: 'element',
						tagName: 'a',
						properties: {
							href,
							className: ['focus-link', 'spring-underline', 'article-link', 'text-text-strong'],
							// hast stores space-separated token lists as arrays. Writing this as one
							// string produced `rel="noopener,noreferrer"` in the output, which is a
							// single unrecognised token and so left the link unprotected.
							...(newTab ? { target: '_blank', rel: ['noopener', 'noreferrer'] } : {}),
						},
						children,
					};
				}
				// An author's note wraps the words it explains and puts its marker after them, so
				// the sentence reads exactly as it would without the note and the collected note
				// at the end can name what it is about. The words get a span of their own -- no
				// resting style, purely so the walk back from a note has something to light up:
				// the marker's number is too small to catch an eye landing mid-page.
				// See spec/styling.md.
				if (directive.name === 'fn') {
					const number = noteNumber(directive);
					return [
						{
							type: 'element',
							tagName: 'span',
							properties: { className: ['note-words'] },
							children,
						},
						{
							type: 'element',
							tagName: 'sup',
							properties: { className: ['note-marker'] },
							children: [
								{
									type: 'element',
									tagName: 'a',
									properties: {
										href: `#note-${number}`,
										id: `marker-${number}`,
										// The id sits here, so the scroll margin has to as well: returning to a
										// marker lands it in the same band arriving at a section does.
										className: ['note-marker-link', 'focus-link', 'jump-target'],
									},
									children: [{ type: 'text', value: String(number) }],
								},
							],
						},
					];
				}
				// A translator's note explains the marked words in place. It becomes a real button
				// because a native title tooltip cannot carry these paragraph-length notes on touch
				// or keyboard; ArticleBody owns the one live popover used by every prose block.
				if (directive.name === 'tn') {
					const note = typeof attrs.is === 'string' ? attrs.is : '';
					return {
						type: 'element',
						tagName: 'button',
						properties: {
							type: 'button',
							className: ['tn-trigger', 'focus-link'],
							'data-tn-note': note,
							ariaControls: ['translator-note'],
							ariaExpanded: 'false',
						},
						children: [
							...children,
							{
								type: 'element' as const,
								tagName: 'svg',
								properties: {
									className: ['tn-icon'],
									viewBox: '0 0 24 24',
									fill: 'none',
									stroke: 'currentColor',
									strokeWidth: '2',
									strokeLineCap: 'round',
									strokeLineJoin: 'round',
									ariaHidden: 'true',
								},
								children: [
									{
										type: 'element' as const,
										tagName: 'circle',
										properties: { cx: '12', cy: '12', r: '10' },
										children: [],
									},
									{
										type: 'element' as const,
										tagName: 'path',
										properties: { d: 'M12 16v-4' },
										children: [],
									},
									{
										type: 'element' as const,
										tagName: 'path',
										properties: { d: 'M12 8h.01' },
										children: [],
									},
								],
							},
						],
					};
				}
				// A spoiler fogs its words until the reader asks for them; the asking is CSS
				// (hover or focus lifts the blur) so nothing here is live. The text stays real
				// underneath -- selectable, translated, read by assistive technology -- because
				// the fog is a display choice, not redaction. tabindex gives keyboards and taps
				// a way to ask on screens that never hover. See spec/styling.md.
				if (directive.name === 'spoiler') {
					return {
						type: 'element',
						tagName: 'span',
						properties: { className: ['spoiler', 'focus-link'], tabIndex: 0 },
						children,
					};
				}
				return {
					type: 'element',
					tagName: 'span',
					properties: { className: styleClasses(attrs) },
					children,
				};
			}) satisfies Handler,
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
			// The markdown target has a footnote of its own, and remark writes it. A real
			// `footnoteReference` rather than the text `[^1]`, which the serialiser would escape
			// into a literal bracket -- the reference is the node it already knows how to spell.
			if (directive.name === 'fn') {
				const number = String(noteNumber(directive));
				return [
					...children,
					{
						type: 'footnoteReference',
						identifier: number,
						label: number,
					} as unknown as RootContent,
				];
			}
			// Plain text has no way to hide a note behind a word, so it is spelled out in
			// brackets. Dropping it would lose the one thing the note exists to say, and the
			// readers of this target are models rather than people scanning a page.
			if (directive.name === 'tn') {
				const note = typeof attrs.is === 'string' ? attrs.is : '';
				return note
					? [...children, { type: 'text', value: ` [${note}]` } as unknown as RootContent]
					: children;
			}
			if ('bold' in attrs) return [{ type: 'strong', children } as unknown as RootContent];
			if ('italic' in attrs) return [{ type: 'emphasis', children } as unknown as RootContent];
			// `:spoiler` also lands here on purpose: plain text has no fog to lift, and this
			// target's readers are models, so the words are worth more than the hiding.
			return children;
		}
		return [node];
	});
}

function proseMarkdown(node: RootContent): string {
	// Lowered first, like the other markdown target. The serialiser has no handler for a
	// directive and throws on one it has not seen, so this path worked only for as long as every
	// directive in the corpus happened to be reachable another way -- `:tn` was the first that
	// was not, and it failed the whole page rather than the one node.
	return stringifier.stringify({ type: 'root', children: lowerDirectives([node]) } as Root).trim();
}

// `## Intro {#getting-started}` -> { text: 'Intro', slug: 'getting-started' }.
// Falls back to a slug derived from the text when no explicit id is present.
function headingParts(node: Heading): { slug: string; text: string } {
	const raw = mdastToString(node).trim();
	const [, explicitText, explicitSlug] = raw.match(/^(.*?)\s*\{#([\w-]+)\}$/) ?? [];
	if (explicitText !== undefined && explicitSlug !== undefined) {
		return { text: explicitText.trim(), slug: explicitSlug };
	}
	const slug = raw
		.toLowerCase()
		.replace(/[^\w]+/g, '-')
		.replace(/^-+|-+$/g, '');
	return { text: raw, slug };
}

function imageOf(node: RootContent): MdImage | null {
	if (node.type !== 'paragraph' || node.children.length !== 1) return null;
	const only = node.children[0];
	return only?.type === 'image' ? only : null;
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
function cropRatio(value: string | null | undefined, url: string, directive: string): string {
	if (value == null) return DEFAULT_CROP;
	const match = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(value.trim());
	if (!match || Number(match[1]) <= 0 || Number(match[2]) <= 0) {
		throw new Error(
			`::${directive} ratio must be W:H with positive numbers, got "${value}": ${url}`,
		);
	}
	return `${match[1]} / ${match[2]}`;
}

function cropAlign(
	value: string | null | undefined,
	url: string,
	directive: string,
): string | undefined {
	if (value == null) return undefined;
	const wanted = value.trim().toLowerCase();
	if (!(ALIGNMENTS as readonly string[]).includes(wanted)) {
		throw new Error(
			`::${directive} align must be one of ${ALIGNMENTS.join(', ')}, got "${value}": ${url}`,
		);
	}
	return wanted === 'center' ? undefined : wanted;
}

/**
 * An article's frontmatter, read with the compiler's own parser.
 *
 * `::article` needs the title of an article nothing has compiled yet, and a second reader of
 * the same format is how the two come to disagree: a folded title would reach the card as
 * `>-`. See spec/code.md.
 */
export function articleFrontmatter(raw: string, file: string): ArticleMeta {
	const front = (parser.parse(raw) as Root).children.find((node) => node.type === 'yaml');
	if (!front) throw new Error(`missing frontmatter: ${file}`);
	return parseYaml(front.value) as ArticleMeta;
}

export type CompileContext = {
	/**
	 * What a screen reader is told about a link that opens elsewhere, in this view's language.
	 *
	 * Handed in rather than looked up. This module is imported by vite.config.ts, which runs
	 * before the Paraglide plugin has generated any messages, so a build-time importer cannot
	 * call one. The caller compiles once per view and has both the locale and the message.
	 */
	newTabNote: string;
	resolveAsset: (reference: string) => Resolved | null;
	/**
	 * What every article in the corpus is called, in the view being compiled, keyed by path.
	 *
	 * Handed in because the compiler sees one article at a time while an `::article` card names
	 * another. A path this map does not hold is a typo rather than a working state, so the
	 * directive throws instead of degrading -- unlike an embed, nothing has to be fetched first.
	 */
	articles?: Record<string, ArticleReference>;
	/** External facts captured before the site build, so rendering never fetches them. */
	embeds?: {
		crates: Record<string, CrateRecord>;
		repos: Record<string, RepoRecord>;
		tweets: Record<string, TweetRecord>;
	};
	highlight: (code: string, lang: string) => Promise<string>;
	/** Present only while reading the source view; translations inherit validated frontmatter. */
	sourceFile?: string;
};

/**
 * The frontmatter keys `cms i18n` translates.
 *
 * A copy: the authority is `TRANSLATABLE_FRONTMATTER` in apps/cms/src/i18n/segment.rs, which is
 * what actually decides which keys become segments. It is repeated here rather than derived
 * because a site-only CI build has no Rust toolchain to ask -- see spec/architecture/data.md --
 * and held to the original by a test rather than by anybody remembering.
 *
 * What drift would cost: this list is what rejects a translator's note in a key that has nowhere
 * to render one. A key Rust translates and this list omits gets translated with the note left in
 * it, and the marker reaches the page as text.
 */
export const TRANSLATABLE_FRONTMATTER = ['title', 'subtitle', 'description'] as const;

function codeMeta(value: string | null | undefined): Record<string, string> {
	const fields: Record<string, string> = {};
	for (const match of value?.matchAll(/(\w+)(?:="([^"]*)")?/g) ?? []) {
		fields[match[1]!] = match[2] ?? 'true';
	}
	return fields;
}

function codePresentation(
	value: string | null | undefined,
	source: string,
): { title?: string; collapsible?: boolean; defaultExpanded?: boolean } {
	const props = codeMeta(value);
	const title = props.title?.trim() || undefined;
	const rawCollapsible = props.collapsible;
	const rawDefault = props.default;

	if (rawCollapsible !== undefined && rawCollapsible !== 'true' && rawCollapsible !== 'false') {
		throw new Error(`${source}: code fence collapsible must be true or false`);
	}
	if (rawDefault !== undefined && rawDefault !== 'expanded' && rawDefault !== 'collapsed') {
		throw new Error(`${source}: code fence default must be expanded or collapsed`);
	}
	if (!title && (rawCollapsible !== undefined || rawDefault !== undefined)) {
		throw new Error(`${source}: a collapsible code fence needs a title`);
	}

	if (!title) return {};
	const collapsible = rawCollapsible !== 'false';
	const defaultExpanded = rawDefault !== 'collapsed';
	if (!collapsible && !defaultExpanded) {
		throw new Error(`${source}: a code fence cannot be fixed open and default collapsed`);
	}
	return { title, collapsible, defaultExpanded };
}

function mermaidRatio(value: string | null | undefined, source: string): number | undefined {
	const raw = codeMeta(value).ratio;
	if (raw === undefined) return undefined;
	if (!/^(?:\d+(?:\.\d+)?|\.\d+)$/.test(raw)) {
		throw new Error(`${source}: Mermaid ratio must be a positive decimal`);
	}
	const ratio = Number(raw);
	if (!Number.isFinite(ratio) || ratio <= 0) {
		throw new Error(`${source}: Mermaid ratio must be a positive decimal`);
	}
	return ratio;
}

function requiredDirectiveAttribute(
	attrs: DirectiveAttrs,
	name: string,
	directive: string,
	source: string,
): string {
	const value = attrs[name]?.trim();
	if (!value) throw new Error(`${source}: ${directive} requires a non-empty ${name} attribute`);
	return value;
}

function quadrantBlock(
	node: ContainerDirective,
	source: string,
): Extract<Block, { type: 'quadrant' }> {
	const attrs = (node.attributes ?? {}) as DirectiveAttrs;
	const title = requiredDirectiveAttribute(attrs, 'title', 'quadrant', source);
	const description = attrs.description?.trim() || undefined;
	const axes = Object.fromEntries(
		QUADRANT_DIRECTIONS.map((direction) => [
			direction,
			requiredDirectiveAttribute(attrs, direction, 'quadrant', source),
		]),
	) as Record<QuadrantDirection, string>;
	const items: QuadrantItem[] = [];

	for (const child of node.children) {
		if (child.type !== 'leafDirective' || child.name !== 'quadrant-item') {
			throw new Error(`${source}: quadrant may contain only quadrant-item directives`);
		}
		const item = child as LeafDirective;
		const itemAttrs = (item.attributes ?? {}) as DirectiveAttrs;
		const at = requiredDirectiveAttribute(itemAttrs, 'at', 'quadrant-item', source);
		if (!QUADRANT_POSITIONS.includes(at as QuadrantPosition)) {
			throw new Error(
				`${source}: quadrant-item at must be one of ${QUADRANT_POSITIONS.join(', ')}`,
			);
		}
		const itemTitle = requiredDirectiveAttribute(itemAttrs, 'title', 'quadrant-item', source);
		const note = itemAttrs.note?.trim() || undefined;
		items.push({ at: at as QuadrantPosition, title: itemTitle, ...(note ? { note } : {}) });
	}
	return {
		type: 'quadrant',
		title,
		...(description ? { description } : {}),
		axes,
		items,
	};
}

function quadrantRegion(item: QuadrantItem, axes: Record<QuadrantDirection, string>): string {
	const [vertical, horizontal] = item.at.split('-') as ['top' | 'bottom', 'left' | 'right'];
	return `${axes[vertical]} / ${axes[horizontal]}`;
}

function cargoView(value: string | null | undefined): CargoView {
	return value === 'table' ? 'table' : 'treemap';
}

function tokeiView(value: string | null | undefined): TokeiView {
	return value === 'bar' || value === 'table' ? value : 'treemap';
}

function cardAlign(value: string | null | undefined): CardAlign {
	return value === 'left' || value === 'right' ? value : 'center';
}

function assertFrontmatterHasNoTranslatorNotes(
	meta: Partial<ArticleMeta> | Record<string, string>,
	file: string,
): void {
	for (const key of TRANSLATABLE_FRONTMATTER) {
		const value = meta[key];
		if (typeof value === 'string' && value.includes(':tn')) {
			throw new Error(`${file}: translator's notes are not allowed in frontmatter ${key}`);
		}
	}
}

export async function compile(
	raw: string,
	url: string,
	{ newTabNote, resolveAsset, articles, highlight, sourceFile, embeds }: CompileContext,
): Promise<Compiled> {
	const tree = parser.parse(raw) as Root;
	let meta: ArticleMeta | undefined;
	const blocks: Block[] = [];
	const toc: TocEntry[] = [];
	const feed: string[] = [];
	const md: string[] = [];
	const text: string[] = [];
	// Numbered by where they are written, across the whole article rather than per block.
	const notes: ArticleNote[] = [];

	for (const node of tree.children) {
		if (node.type === 'yaml') {
			meta = parseYaml(node.value) as ArticleMeta;
			if (sourceFile) assertLanguageTag(meta?.lang, sourceFile);
			assertFrontmatterHasNoTranslatorNotes(meta ?? {}, sourceFile ?? url);
			continue;
		}

		if (node.type === 'heading') {
			// Collected before the text is read off the node. `headingParts` flattens the heading
			// to a string, and a directive with no children leaves nothing behind when it does --
			// which is why a note in a heading never reaches the ToC or the slug.
			const marks = numberNotes(node, notes, sourceFile ?? url);
			const { slug, text: heading } = headingParts(node);
			const superscripts = marks.map((number) => `[^${number}]`).join('');
			blocks.push({
				type: 'heading',
				depth: node.depth,
				slug,
				text: heading,
				...(marks.length > 0 ? { notes: marks } : {}),
			});
			// Only the top level is offered as navigation. The rail is 192px wide and collapses to
			// a column of bars, which makes it a way to reach a section rather than an outline of
			// the article; a subsection is reached by arriving at its parent and reading on. Its
			// anchor still exists and still resolves -- what is filtered is the listing, not the
			// address. See spec/styling.md.
			if (node.depth === 2) toc.push({ slug, text: heading, depth: node.depth });
			feed.push(
				`<h${node.depth} id="${slug}">${escapeHtml(heading)}${marks
					.map((number) => `<sup>${number}</sup>`)
					.join('')}</h${node.depth}>`,
			);
			md.push(`${'#'.repeat(node.depth)} ${heading}${superscripts}`);
			text.push(heading);
			continue;
		}

		if (node.type === 'code') {
			const lang = node.lang ?? 'text';
			// Mermaid is still authored as an ordinary fenced block, but its source becomes a
			// client-rendered diagram rather than highlighted code. See spec/styling.md.
			if (lang.toLowerCase() === 'mermaid') {
				const ratio = mermaidRatio(node.meta, sourceFile ?? url);
				blocks.push({
					type: 'mermaid',
					source: node.value,
					...(ratio === undefined ? {} : { ratio }),
				});
				feed.push(`<pre><code class="language-mermaid">${escapeHtml(node.value)}</code></pre>`);
				md.push('```mermaid\n' + node.value + '\n```');
				continue;
			}
			// Pasted straight from the tool, so the markdown keeps something a person can read
			// and check against their terminal. Parsing it back costs less than keeping a second
			// machine-readable copy in step with it.
			if (lang === 'tokei') {
				const props = codeMeta(node.meta);
				const title = props.title || meta?.title || 'code statistics';
				blocks.push({ type: 'tokei', source: node.value, title, view: tokeiView(props.view) });
				feed.push(`<pre>${escapeHtml(node.value)}</pre>`);
				md.push('```\n' + node.value + '\n```');
				continue;
			}
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
				label: languageLabel(lang),
				...codePresentation(node.meta, sourceFile ?? url),
				html: await highlight(node.value, lang),
				code: node.value,
			});
			feed.push(`<pre><code>${escapeHtml(node.value)}</code></pre>`);
			md.push(`\`\`\`${lang}\n${node.value}\n\`\`\``);
			continue;
		}

		if (node.type === 'containerDirective' && node.name === 'quadrant') {
			const quadrant = quadrantBlock(node, sourceFile ?? url);
			blocks.push(quadrant);
			const entries = quadrant.items.map((item) => {
				const note = item.note ? ` — ${escapeHtml(item.note)}` : '';
				return `<li><strong>${escapeHtml(item.title)}</strong>${note} <small>(${escapeHtml(quadrantRegion(item, quadrant.axes))})</small></li>`;
			});
			feed.push(
				`<figure><figcaption><strong>${escapeHtml(quadrant.title)}</strong>${quadrant.description ? ` — ${escapeHtml(quadrant.description)}` : ''}</figcaption><ul>${entries.join('')}</ul></figure>`,
			);
			md.push(
				[
					`> [quadrant: ${quadrant.title}]`,
					...(quadrant.description ? [`> ${quadrant.description}`] : []),
					...quadrant.items.map(
						(item) =>
							`> - ${quadrantRegion(item, quadrant.axes)}: ${item.title}${item.note ? ` — ${item.note}` : ''}`,
					),
				].join('\n'),
			);
			text.push(
				[
					quadrant.title,
					...(quadrant.description ? [quadrant.description] : []),
					...quadrant.items.map(
						(item) =>
							`${quadrantRegion(item, quadrant.axes)}: ${item.title}${item.note ? ` — ${item.note}` : ''}`,
					),
				].join('\n'),
			);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'quadrant-item') {
			throw new Error(`${sourceFile ?? url}: quadrant-item must be inside a quadrant`);
		}

		if (node.type === 'leafDirective' && node.name === 'linkcard') {
			const attrs = node.attributes ?? {};
			const tone: 'light' | 'dark' | undefined =
				attrs.tone === 'dark' ? 'dark' : attrs.tone === 'light' ? 'light' : undefined;
			const card = { src: attrs.src ?? '', url: attrs.url ?? '', title: attrs.title ?? '', tone };
			// Cropped like `::image`, defaults included: a card is typed on purpose, so saying
			// nothing about the ratio reads as "the usual one" rather than as "leave it alone".
			// Screenshots arrive at whatever shape a window happened to be, and a column of
			// cards at ten heights is the thing a default ratio exists to prevent.
			const crop = cropRatio(attrs.ratio, url, 'linkcard');
			const align = cropAlign(attrs.align, url, 'linkcard');
			// A card's cover is an asset like any other, so it gets the same variants and
			// placeholder. Resolving here rather than in the component keeps the manifest --
			// every base64 preview in it -- out of the client bundle.
			blocks.push({ type: 'linkcard', ...card, crop, align, ...resolveAsset(card.src) });
			feed.push(`<p><a href="${card.url}">${escapeHtml(card.title)}</a></p>`);
			md.push(`[${card.title}](${card.url})`);
			continue;
		}

		// `::article` is a link to another article in this repo, drawn as the card the homepage
		// lists. It carries no copy of its own: name, subtitle and date are the target's, so a
		// retitled article retitles every card pointing at it and each view names it in its own
		// language. Compare `::linkcard`, which describes something outside the corpus and
		// therefore has to be told what to say.
		if (node.type === 'leafDirective' && node.name === 'article') {
			const source = sourceFile ?? url;
			const attrs = (node.attributes ?? {}) as DirectiveAttrs;
			const target = requiredDirectiveAttribute(attrs, 'path', 'article', source);
			const reference = articles?.[target];
			if (!reference) {
				throw new Error(`${source}: ::article path "${target}" does not name an article`);
			}
			const href = `${URLS.apps.production.site}/${target}`;
			blocks.push({ type: 'article', path: target, ...reference });
			feed.push(
				`<p><a href="${href}">${escapeHtml(reference.title)}</a> — ${escapeHtml(reference.subtitle)}</p>`,
			);
			md.push(`[${reference.title}](${href}) — ${reference.subtitle}`);
			text.push(`${reference.title}\n${reference.subtitle}`);
			continue;
		}

		// `::image` is the cropped presentation of an asset. Plain `![]()` stays uncropped, so
		// writing this directive is itself the request to crop -- which is why the defaults
		// here are a ratio and an alignment rather than "no change".
		if (node.type === 'leafDirective' && node.name === 'image') {
			const attrs = node.attributes ?? {};
			const src = attrs.src ?? '';
			const crop = cropRatio(attrs.ratio, url, 'image');
			const align = cropAlign(attrs.align, url, 'image');
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

		if (node.type === 'leafDirective' && node.name === 'cargo') {
			const name = node.attributes?.crate ?? '';
			const crate = embeds?.crates[name];
			if (crate) {
				blocks.push({ type: 'cargo', crate, view: cargoView(node.attributes?.view) });
				feed.push(
					`<p><em>[crate: ${escapeHtml(crate.name)} ${escapeHtml(crate.version)}]</em></p>`,
				);
				md.push(`> [crate: ${crate.name} ${crate.version}]`);
				continue;
			}
			// Named but not fetched. The article keeps saying which crate it meant, so `cms embed`
			// can fill it in later without anyone editing prose to ask again.
			blocks.push({ type: 'placeholder', kind: 'cargo', meta: { crate: name } });
			md.push(`> [crate: ${name}]`);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'github') {
			const name = node.attributes?.repo ?? '';
			const repo = embeds?.repos[name];
			const gitRef = node.attributes?.ref ?? undefined;
			if (repo) {
				blocks.push({
					type: 'github',
					repo,
					gitRef,
					title: node.attributes?.title ?? undefined,
					align: cardAlign(node.attributes?.align),
				});
				feed.push(`<p><em>[repository: ${escapeHtml(repo.full_name)}]</em></p>`);
				md.push(`> [repository: ${repo.full_name}]`);
				continue;
			}
			blocks.push({ type: 'placeholder', kind: 'github', meta: { repo: name } });
			md.push(`> [repository: ${name}]`);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'twitter') {
			const id = node.attributes?.tweet ?? '';
			const tweet = embeds?.tweets[id];
			if (tweet) {
				const href = `${URLS.external.social.twitter}/${tweet.author}/status/${tweet.id}`;
				blocks.push({ type: 'twitter', tweet });
				feed.push(
					`<blockquote><p>${escapeHtml(tweet.text).replaceAll('\n', '<br />')}</p>` +
						`<footer><a href="${href}">@${escapeHtml(tweet.author)} on Twitter</a></footer>` +
						'</blockquote>',
				);
				md.push(
					`> ${tweet.text.replaceAll('\n', '\n> ')}\n>\n> — [@${tweet.author} on Twitter](${href})`,
				);
				text.push(tweet.text);
				continue;
			}
			blocks.push({ type: 'placeholder', kind: 'twitter', meta: { tweet: id } });
			md.push(`> [tweet: ${id}]`);
			continue;
		}

		if (node.type === 'leafDirective' && node.name === 'placeholder') {
			const { kind, ...rest } = node.attributes ?? {};
			const placeholderMeta: Record<string, string> = {};
			for (const [key, value] of Object.entries(rest)) {
				if (value != null) placeholderMeta[key] = value;
			}
			const label = kind ?? '';
			const metaText = Object.entries(placeholderMeta)
				.map(([k, v]) => ` ${k}="${v}"`)
				.join('');
			blocks.push({ type: 'placeholder', kind: label, meta: placeholderMeta });
			feed.push(
				`<pre>::${escapeHtml(label)}${Object.entries(placeholderMeta)
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

		numberNotes(node, notes, sourceFile ?? url);
		blocks.push({ type: 'prose', html: proseHtml(node, newTabNote) });
		feed.push(proseHtml(node, newTabNote));
		md.push(proseMarkdown(node));
		const plain = mdastToString(node).trim();
		if (plain) text.push(plain);
	}

	if (notes.length > 0) {
		blocks.push({ type: 'footnotes', notes });
		feed.push(
			`<ol>${notes
				.map(
					({ number, phrase, text: said }) =>
						`<li id="note-${number}"><strong>${escapeHtml(phrase)}</strong> ${escapeHtml(said)}</li>`,
				)
				.join('')}</ol>`,
		);
		// The definition carries only what the note says. The phrase is already beside the marker
		// in the body, and a footnote that repeated the word it hangs off would read it twice.
		md.push(notes.map(({ number, text: said }) => `[^${number}]: ${said}`).join('\n'));
		text.push(notes.map(({ phrase, text: said }) => `${phrase} ${said}`).join('\n'));
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
function inlineSegments(node: Paragraph, newTabNote: string): InlineSegment[] {
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
			const { href, newTab, platform } = resolveLink(label, attrs);
			segments.push({
				type: 'link',
				icon: platform,
				href,
				label,
				newTab,
			});
		} else {
			run.push(proseHtml(child as RootContent, newTabNote));
		}
	}
	flush();
	return segments;
}

// A standalone page (e.g. the homepage at contents/homepage.md). Unlike an article
// it carries free-form frontmatter and produces blocks for the route to render
// plus the DLC-lowered prose body (getPage wraps it into the served document).
export function compilePage(
	raw: string,
	newTabNote: string,
	sourceFile = 'page frontmatter',
): CompiledPage {
	const tree = parser.parse(raw) as Root;
	let meta: Record<string, string> = {};
	const blocks: PageBlock[] = [];
	const bodyNodes: RootContent[] = [];

	for (const node of tree.children) {
		if (node.type === 'yaml') {
			meta = (parseYaml(node.value) ?? {}) as Record<string, string>;
			assertFrontmatterHasNoTranslatorNotes(meta, sourceFile);
			continue;
		}
		bodyNodes.push(node);
		blocks.push(
			node.type === 'paragraph'
				? { type: 'p', segments: inlineSegments(node, newTabNote) }
				: { type: 'html', html: proseHtml(node, newTabNote) },
		);
	}

	const body = stringifier
		.stringify({ type: 'root', children: lowerDirectives(bodyNodes) } as Root)
		.trim();
	return { meta, blocks, body };
}
