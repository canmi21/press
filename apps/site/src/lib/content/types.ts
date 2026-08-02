import type { ArticleMeta } from '$lib/article.svelte';
import type { LocaleCode } from '$lib/locale';

// One markdown source compiles to several targets; only the custom blocks below
// need bespoke per-target output. `block` drives page rendering, `feed` is
// self-contained HTML for RSS, `markdown` is LLM-friendly (/llms.txt), `text` is
// plain text where advanced expressions collapse to empty.
export type Block =
	| { type: 'prose'; html: string }
	| { type: 'heading'; depth: number; slug: string; text: string }
	| { type: 'code'; lang: string; html: string; code: string }
	| { type: 'svgCanvas'; svg: string; title: string }
	| {
			type: 'linkcard';
			src: string;
			url: string;
			title: string;
			tone?: 'light' | 'dark';
			width?: number;
			height?: number;
			preview?: string;
			srcset?: string;
			/** What the cover shows. Offered as the link's description, never as its name. */
			description?: string;
	  }
	| { type: 'placeholder'; kind: string; meta: Record<string, string> }
	| {
			type: 'image';
			src: string;
			alt: string;
			width?: number;
			height?: number;
			/** The image's own aspect ratio, as derived. Not what it is displayed at. */
			ratio?: string;
			/**
			 * A ratio to crop the displayed image to, as `16 / 9` ready for CSS.
			 *
			 * Cropping is presentation, so it is done by the browser with `object-fit` rather
			 * than by producing another object. A stored variant per ratio and alignment would
			 * multiply the bucket and, worse, make a content id mean "this image as shown here"
			 * instead of "this image".
			 */
			crop?: string;
			/** `object-position` for that crop. Absent means centred. */
			align?: string;
			preview?: string;
			srcset?: string;
	  };

export type TocEntry = { slug: string; text: string; depth: number };

export type Compiled = {
	meta: ArticleMeta;
	toc: TocEntry[];
	blocks: Block[];
	feed: string;
	markdown: string;
	text: string;
};

export type ArticleView = Pick<Compiled, 'meta' | 'blocks' | 'text'> & {
	code: LocaleCode;
	languageTag: string;
	canonical: string;
};

export type Alternate = { code: LocaleCode | 'x-default'; languageTag: string; href: string };

export type Article = Compiled & {
	path: string;
	url: string;
	views: Record<LocaleCode, ArticleView>;
	alternates: Alternate[];
};

// A page paragraph is split at `:link` boundaries so styled text stays dead HTML
// while each link renders live (with its `<Icon>`), keeping the {@html} zone small.
export type InlineSegment =
	| { type: 'html'; html: string }
	| {
			type: 'link';
			icon?: 'twitter' | 'github' | 'email';
			href: string;
			label: string;
			newTab: boolean;
	  };

export type PageBlock = { type: 'p'; segments: InlineSegment[] } | { type: 'html'; html: string };

// A standalone, non-article page (e.g. the homepage). `meta` carries frontmatter;
// `blocks` are rendered by the route; `body` is the DLC-lowered prose.
export type CompiledPage = {
	meta: Record<string, string>;
	blocks: PageBlock[];
	body: string;
};

// As served at /<slug>.md: `markdown` is the generated standalone document (see
// the mapping in content/index.ts), the rest passes through from CompiledPage.
export type Page = Omit<CompiledPage, 'body'> & { markdown: string };
