import type { Article, ArticleView, Page, PageView } from './types.ts';
import type { LocaleCode } from '../locale/index.ts';

type ArticleContent = Pick<ArticleView, 'meta' | 'toc' | 'blocks' | 'feed' | 'text'>;
type ArticleViewFields = Omit<ArticleView, keyof ArticleContent> & { content: number };

export type PackedArticle = Omit<Article, 'views'> & {
	contents: ArticleContent[];
	views: Record<LocaleCode, ArticleViewFields>;
};

type PackedPage = Omit<Page, 'views'> & {
	contents: PageView[];
	views: Record<LocaleCode, { content: number }>;
};

function articleContent(source: ArticleContent): ArticleContent {
	return {
		meta: source.meta,
		toc: source.toc,
		blocks: source.blocks,
		feed: source.feed,
		text: source.text,
	};
}

function sameArticleContent(left: ArticleContent, right: ArticleContent): boolean {
	return (
		left.meta === right.meta &&
		left.toc === right.toc &&
		left.blocks === right.blocks &&
		left.feed === right.feed &&
		left.text === right.text
	);
}

export function packArticles(articles: Article[]): PackedArticle[] {
	return articles.map((source) => {
		const { views, ...article } = source;
		const contents: ArticleContent[] = [];
		const packedViews = Object.fromEntries(
			Object.entries(views).map(([code, view]) => {
				const { meta, toc, blocks, feed, text, ...fields } = view;
				const shared = { meta, toc, blocks, feed, text };
				let content = sameArticleContent(source, shared)
					? -1
					: contents.findIndex((candidate) => sameArticleContent(candidate, shared));
				if (content === -1 && !sameArticleContent(source, shared)) {
					content = contents.push(shared) - 1;
				}
				return [code, { ...fields, content }];
			}),
		) as Record<LocaleCode, ArticleViewFields>;
		return { ...article, contents, views: packedViews };
	});
}

export function unpackArticles(packed: PackedArticle[]): Article[] {
	return packed.map(({ contents, views, ...article }) => ({
		...article,
		views: Object.fromEntries(
			Object.entries(views).map(([code, { content, ...fields }]) => [
				code,
				{
					...(content === -1 ? articleContent(article) : contents[content]),
					...fields,
				},
			]),
		) as Record<LocaleCode, ArticleView>,
	}));
}

export function packPages(pages: Page[]): PackedPage[] {
	return pages.map(({ views, ...page }) => {
		const contents: PageView[] = [];
		const packedViews = Object.fromEntries(
			Object.entries(views).map(([code, view]) => {
				let content = contents.findIndex(
					(candidate) => candidate.meta === view.meta && candidate.blocks === view.blocks,
				);
				if (content === -1) content = contents.push(view) - 1;
				return [code, { content }];
			}),
		) as Record<LocaleCode, { content: number }>;
		return { ...page, contents, views: packedViews };
	});
}

export function unpackPages(packed: PackedPage[]): Page[] {
	return packed.map(({ contents, views, ...page }) => ({
		...page,
		views: Object.fromEntries(
			Object.entries(views).map(([code, { content }]) => [code, contents[content]]),
		) as Record<LocaleCode, PageView>,
	}));
}
