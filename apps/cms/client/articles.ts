export type ArticleListing = {
	locales: string[];
	articles: Array<{
		path: string;
		section: string;
		title: string;
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

/**
 * The shortest label that still tells two locales apart.
 *
 * Truncating to the language subtag collides the moment a second locale shares it: `zh-CN` and
 * `zh-TW` both render as `ZH`, and the strip then shows one of them twice while claiming to show
 * both. So the language is used only where it is unique in the set, and the region stands in
 * where it is not -- which is exactly the part a reader needs to tell those two apart.
 *
 * Derived from the locales the snapshot actually carries rather than from a table, so adding a
 * ninth locale cannot leave a stale entry behind.
 */
function localeLabels(locales: readonly string[]): Map<string, string> {
	const languages = new Map<string, number>();
	for (const locale of locales) {
		const language = locale.split('-')[0] ?? locale;
		languages.set(language, (languages.get(language) ?? 0) + 1);
	}
	const labels = new Map<string, string>();
	for (const locale of locales) {
		const [language, region] = locale.split('-');
		const unique = (languages.get(language ?? locale) ?? 0) === 1;
		labels.set(locale, unique || region === undefined ? (language ?? locale) : region);
	}
	return labels;
}

function sectionLabel(name: string): string {
	return name
		.split('-')
		.map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
		.join(' ');
}

/**
 * A row's state, which decides its colour and how loudly it reads.
 *
 * Complete is the resting state and takes no hue at all -- an article that is finished is the
 * ordinary case, and colouring it green would leave a page of green that means nothing. Only the
 * two states a person can act on are marked.
 */
function toneOf(article: Article): 'complete' | 'partial' | 'untranslated' {
	if (article.translated >= article.wanted && article.summaryGaps.length === 0) return 'complete';
	if (article.translated === 0) return 'untranslated';
	return 'partial';
}

function gapSummary(article: Article): string {
	const parts: string[] = [];
	const segments = article.gaps.reduce((total, gap) => total + gap.segments, 0);
	if (segments > 0) {
		const locales = article.gaps.map((gap) => gap.locale).join(', ');
		parts.push(`${segments} missing in ${locales}`);
	}
	if (article.summaryGaps.length > 0) {
		parts.push(`summary missing in ${article.summaryGaps.join(', ')}`);
	}
	if (article.orphans > 0) {
		parts.push(`${article.orphans} orphaned by an edit`);
	}
	return parts.length === 0 ? 'Complete in every locale' : parts.join(' · ');
}

function renderArticle(
	article: Article,
	locales: readonly string[],
	labels: ReadonlyMap<string, string>,
): HTMLLIElement {
	const item = document.createElement('li');
	item.dataset.tone = toneOf(article);

	const heading = document.createElement('div');
	heading.className = 'article-heading';
	const title = document.createElement('strong');
	title.textContent = article.title;
	const path = document.createElement('code');
	path.className = 'article-path';
	path.textContent = article.path;
	heading.appendChild(title);
	heading.appendChild(path);

	const coverage = document.createElement('div');
	coverage.className = 'article-coverage';
	const track = document.createElement('span');
	track.className = 'meter-track';
	const fill = document.createElement('span');
	fill.className = 'meter-fill';
	const ratio = article.wanted === 0 ? 0 : article.translated / article.wanted;
	fill.style.setProperty('--meter-fill', `${(ratio * 100).toFixed(2)}%`);
	track.appendChild(fill);
	const count = document.createElement('span');
	count.className = 'meter-value';
	count.textContent = `${article.translated}/${article.wanted}`;
	coverage.appendChild(track);
	coverage.appendChild(count);

	const detail = document.createElement('span');
	detail.className = 'article-detail';
	detail.textContent = gapSummary(article);

	// One cell per locale, so a reader scans down a column to find the language that is behind
	// rather than reading the same sentence on every row. The grid is the article's own, not a
	// shared one: locales never wrap, so nothing has to line up between rows.
	const grid = document.createElement('div');
	grid.className = 'locale-grid';
	for (const locale of locales) {
		const cell = document.createElement('span');
		cell.className = 'locale-cell';
		const gap = article.gaps.find((entry) => entry.locale === locale);
		cell.dataset.state = gap === undefined ? 'covered' : 'short';
		cell.textContent = labels.get(locale) ?? locale;
		cell.title =
			gap === undefined
				? `${locale}: complete`
				: `${locale}: ${gap.segments} of ${article.segments} segments missing`;
		grid.appendChild(cell);
	}

	item.appendChild(heading);
	item.appendChild(coverage);
	item.appendChild(detail);
	item.appendChild(grid);
	return item;
}

export function renderArticles(root: HTMLElement, listing: ArticleListing): void {
	const total = listing.articles.length;
	const behind = listing.articles.filter((article) => toneOf(article) !== 'complete').length;
	requiredElement<HTMLElement>(root, '[data-articles-total]').textContent =
		`${total} ${total === 1 ? 'article' : 'articles'}`;
	requiredElement<HTMLElement>(root, '[data-articles-state]').textContent =
		behind === 0 ? 'All complete' : `${behind} behind`;

	const sections = new Map<string, Article[]>();
	for (const article of listing.articles) {
		const existing = sections.get(article.section);
		if (existing === undefined) sections.set(article.section, [article]);
		else existing.push(article);
	}

	const labels = localeLabels(listing.locales);
	const list = requiredElement<HTMLElement>(root, '[data-articles-list]');
	list.replaceChildren();
	for (const [section, articles] of sections) {
		const group = document.createElement('section');
		group.className = 'article-group';
		const heading = document.createElement('h2');
		heading.textContent = sectionLabel(section);
		const count = document.createElement('span');
		count.textContent = String(articles.length);
		const header = document.createElement('header');
		header.appendChild(heading);
		header.appendChild(count);
		const items = document.createElement('ul');
		items.className = 'article-list';
		for (const article of articles) {
			items.appendChild(renderArticle(article, listing.locales, labels));
		}
		group.appendChild(header);
		group.appendChild(items);
		list.appendChild(group);
	}
}

export function renderArticlesError(root: HTMLElement, error: unknown): void {
	requiredElement<HTMLElement>(root, '[data-articles-total]').textContent = '—';
	const state = requiredElement<HTMLElement>(root, '[data-articles-state]');
	state.textContent = error instanceof Error ? error.message : String(error);
	state.dataset.state = 'error';
}
