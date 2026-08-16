import { articleThumbnail, formatArticleDate } from './article-preview';

export type ArticleListing = {
	locales: string[];
	articles: Array<{
		path: string;
		section: string;
		title: string;
		subtitle: string | null;
		modified: string | null;
		lang: string;
		segments: number;
		translated: number;
		wanted: number;
		gaps: Array<{ locale: string; segments: number }>;
		orphans: number;
		summary: boolean;
		summaryGaps: string[];
	}>;
};

type Article = ArticleListing['articles'][number];

function requiredElement<T extends Element>(root: Element, selector: string): T {
	const element = root.querySelector<T>(selector);
	if (element === null) throw new Error(`required element is missing: ${selector}`);
	return element;
}

function sectionLabel(name: string): string {
	return name
		.split('-')
		.map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
		.join(' ');
}

function gapSummary(article: Article): string | null {
	const parts: string[] = [];
	const segments = article.gaps.reduce((total, gap) => total + gap.segments, 0);
	if (segments > 0) {
		const locales = article.gaps.map((gap) => gap.locale).join(', ');
		parts.push(
			`${segments} translation ${segments === 1 ? 'segment' : 'segments'} missing in ${locales}`,
		);
	}
	if (article.summaryGaps.length > 0) {
		parts.push(`Summary missing in ${article.summaryGaps.join(', ')}`);
	}
	if (article.orphans > 0) {
		parts.push(
			`${article.orphans} stale ${article.orphans === 1 ? 'segment' : 'segments'} from an edit`,
		);
	}
	return parts.length === 0 ? null : parts.join(' · ');
}

function renderArticle(article: Article): HTMLLIElement {
	const item = document.createElement('li');
	item.className = 'article-preview article-library-row';

	const copy = document.createElement('div');
	copy.className = 'article-preview-copy';
	const heading = document.createElement('div');
	heading.className = 'article-preview-heading';
	const title = document.createElement('strong');
	title.className = 'article-preview-title';
	title.textContent = article.title;
	heading.appendChild(title);

	if (article.modified !== null) {
		const leader = document.createElement('span');
		leader.className = 'article-preview-leader';
		const modified = document.createElement('time');
		modified.className = 'article-preview-date';
		modified.dateTime = article.modified;
		modified.textContent = formatArticleDate(article.modified);
		heading.appendChild(leader);
		heading.appendChild(modified);
	}
	copy.appendChild(heading);

	const subtitle = document.createElement('p');
	subtitle.className = 'article-preview-subtitle';
	subtitle.textContent = article.subtitle ?? article.path;
	copy.appendChild(subtitle);

	const gap = gapSummary(article);
	if (gap !== null) {
		const detail = document.createElement('p');
		detail.className = 'article-library-gap';
		detail.textContent = gap;
		copy.appendChild(detail);
	}

	item.appendChild(articleThumbnail());
	item.appendChild(copy);
	return item;
}

export function renderArticles(root: HTMLElement, listing: ArticleListing): void {
	const total = listing.articles.length;
	const sections = new Map<string, Article[]>();
	for (const article of listing.articles) {
		const existing = sections.get(article.section);
		if (existing === undefined) sections.set(article.section, [article]);
		else existing.push(article);
	}

	requiredElement<HTMLElement>(root, '[data-articles-total]').textContent =
		total === 0
			? 'No articles yet.'
			: `${total} ${total === 1 ? 'article' : 'articles'} across ${sections.size} ${sections.size === 1 ? 'section' : 'sections'}.`;

	const behind = listing.articles.filter((article) => gapSummary(article) !== null).length;
	const state = requiredElement<HTMLElement>(root, '[data-articles-state]');
	state.hidden = behind === 0;
	state.textContent = behind === 0 ? '' : `${behind} need attention`;
	delete state.dataset.state;

	const list = requiredElement<HTMLElement>(root, '[data-articles-list]');
	list.replaceChildren();
	for (const [section, articles] of sections) {
		const group = document.createElement('section');
		group.className = 'article-group';
		const heading = document.createElement('h2');
		heading.textContent = sectionLabel(section);
		const items = document.createElement('ul');
		items.className = 'article-list';
		for (const article of articles) items.appendChild(renderArticle(article));
		group.appendChild(heading);
		group.appendChild(items);
		list.appendChild(group);
	}
}

export function renderArticlesError(root: HTMLElement, error: unknown): void {
	requiredElement<HTMLElement>(root, '[data-articles-total]').textContent = 'Articles unavailable.';
	const state = requiredElement<HTMLElement>(root, '[data-articles-state]');
	state.hidden = false;
	state.textContent = error instanceof Error ? error.message : String(error);
	state.dataset.state = 'error';
}
