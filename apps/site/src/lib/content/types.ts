import type { LocaleCode } from '../locale/index.ts';

export type ArticleMeta = {
	title: string;
	subtitle: string;
	description: string;
	/** The source view's public BCP-47 language tag. */
	lang: string;
	created: string;
	lastmod: string;
};

// One markdown source compiles to several targets; only the custom blocks below
// need bespoke per-target output. `block` drives page rendering, `feed` is
// self-contained HTML for RSS, `markdown` is LLM-friendly (/llms.txt), `text` is
// plain text where advanced expressions collapse to empty.
export type Block =
	| { type: 'prose'; html: string }
	| { type: 'heading'; depth: number; slug: string; text: string }
	| { type: 'code'; lang: string; html: string; code: string }
	| { type: 'svgCanvas'; svg: string; title: string }
	| { type: 'tokei'; source: string; title: string; view: TokeiView }
	| { type: 'cargo'; crate: CrateRecord; view: CargoView }
	| { type: 'twitter'; tweet: TweetRecord }
	| {
			type: 'github';
			repo: RepoRecord;
			gitRef?: string;
			title?: string;
			align: CardAlign;
	  }
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

export type CardAlign = 'left' | 'center' | 'right';
export type CargoView = 'treemap' | 'table';
export type TokeiView = 'treemap' | 'bar' | 'table';

export type Compiled = {
	meta: ArticleMeta;
	toc: TocEntry[];
	blocks: Block[];
	feed: string;
	markdown: string;
	text: string;
};

export type ArticleView = Pick<Compiled, 'meta' | 'toc' | 'blocks' | 'feed' | 'text'> & {
	code: LocaleCode;
	languageTag: string;
	canonical: string;
	/** False when this locale is showing the complete source article as a safe fallback. */
	translationAvailable: boolean;
	/**
	 * What the article is about, withholding what it concludes. Written by `cms summary` into a
	 * sidecar rather than into the article, so it is absent until that has been run.
	 */
	summary?: ArticleSummary;
};

export type ArticleSummary = {
	text: string;
	provider: string;
};

export type CrateDep = {
	name: string;
	version: string;
	kind: string;
	optional: boolean;
	target: string | null;
	features: string[];
	size: number | null;
	depth: number;
};

export type CrateRecord = {
	name: string;
	version: string;
	rust_version: string | null;
	features: Record<string, string[]>;
	deps: CrateDep[];
	total_dep_size: number;
};

export type RepoRecord = {
	full_name: string;
	description: string | null;
	language: string | null;
	stars: number;
	forks: number;
	open_issues: number;
	license: string | null;
	pushed_at: string | null;
};

export type TweetRecord = {
	id: string;
	author: string;
	text: string;
	created: string;
	likes: number;
	reposts: number;
	replies: number;
};

export type Alternate = {
	code: Exclude<LocaleCode, 'mw'> | 'x-default';
	languageTag: string;
	href: string;
};

export type Article = Compiled & {
	path: string;
	url: string;
	views: Record<LocaleCode, ArticleView>;
	canonicalUrls: string[];
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

export type PageView = Pick<CompiledPage, 'meta' | 'blocks'>;

// The source remains the public /<slug>.md document. Browser-facing HTML chooses
// one of the compiled views using the same request locale as an article.
export type Page = {
	path: string;
	markdown: string;
	views: Record<LocaleCode, PageView>;
};
