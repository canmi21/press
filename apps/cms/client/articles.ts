import { requiredElement } from './dom';

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

type Filter = 'all' | 'attention';

let listing: ArticleListing | null = null;
let filter: Filter = 'all';
let opened: string | null = null;

/**
 * Whether an article is carrying work.
 *
 * Three unrelated conditions, and they are equally outstanding: a locale short of segments, a
 * missing summary, and segments an edit left stale. Nothing here ranks them, because the page
 * does not know which the author would rather do first.
 */
function outstanding(article: Article): boolean {
	return article.gaps.length > 0 || article.summaryGaps.length > 0 || article.orphans > 0;
}

function formatDate(value: string): string {
	return new Date(value).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
	});
}

function sectionLabel(name: string): string {
	return name
		.split('-')
		.map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
		.join(' ');
}

function element<K extends keyof HTMLElementTagNameMap>(
	tag: K,
	className: string,
	text?: string,
): HTMLElementTagNameMap[K] {
	const node = document.createElement(tag);
	node.className = className;
	if (text !== undefined) node.textContent = text;
	return node;
}

/** The work an article is carrying, as short phrases rather than one joined sentence. */
function findings(article: Article): string[] {
	const found: string[] = [];
	const missing = article.gaps.reduce((total, gap) => total + gap.segments, 0);
	if (missing > 0) {
		const locales = article.gaps.map((gap) => gap.locale).join(', ');
		found.push(`${missing} ${missing === 1 ? 'segment' : 'segments'} untranslated in ${locales}`);
	}
	if (article.summaryGaps.length > 0) {
		found.push(`No summary in ${article.summaryGaps.join(', ')}`);
	}
	if (article.orphans > 0) {
		found.push(
			`${article.orphans} stale ${article.orphans === 1 ? 'segment' : 'segments'} from an edit`,
		);
	}
	return found;
}

/**
 * The panel a row opens.
 *
 * It carries what the row deliberately left out -- the subtitle, the path, where each locale
 * stands -- plus the command that closes the work. The command is text, not a button: an
 * operation only becomes a control once the task substrate can watch it and refuse a second
 * copy, and `cms locale` has not moved below the shells yet. See spec/architecture/cms.md.
 */
function detail(article: Article, locales: string[]): HTMLElement {
	const panel = element('div', 'row-detail');

	if (article.subtitle !== null) panel.appendChild(element('p', 'row-subtitle', article.subtitle));
	panel.appendChild(element('p', 'row-path', article.path));

	const short = new Map(article.gaps.map((gap) => [gap.locale, gap.segments]));
	const coverage = element('div', 'locales');
	for (const locale of locales) {
		const missing = short.get(locale) ?? 0;
		const chip = element('span', 'locale', locale);
		// The source language is not a translation of anything, so it is marked as the origin
		// rather than counted as complete.
		if (locale === locales[0]) chip.dataset.state = 'source';
		else if (missing > 0) chip.dataset.state = 'short';
		if (missing > 0) chip.title = `${missing} segments missing`;
		coverage.appendChild(chip);
	}
	panel.appendChild(coverage);

	const found = findings(article);
	if (found.length > 0) {
		const list = element('ul', 'row-findings');
		for (const line of found) list.appendChild(element('li', '', line));
		panel.appendChild(list);

		const command = element('p', 'row-command');
		command.appendChild(element('code', '', article.orphans > 0 ? 'cms locale' : 'cms summary'));
		command.appendChild(document.createTextNode(' closes this from a terminal.'));
		panel.appendChild(command);
	}

	return panel;
}

function renderRow(article: Article, locales: string[]): HTMLElement {
	const row = element('div', 'row');
	if (outstanding(article)) row.dataset.attention = '';
	if (opened === article.path) row.dataset.open = '';

	const summary = document.createElement('button');
	summary.type = 'button';
	summary.className = 'row-summary';
	summary.setAttribute('aria-expanded', opened === article.path ? 'true' : 'false');

	const identity = element('span', 'row-identity');
	identity.appendChild(element('span', 'row-title', article.title));

	const section = element('span', 'row-section', sectionLabel(article.section));
	const segments = element('span', 'row-segments', String(article.segments));

	const state = element('span', 'row-state');
	if (article.orphans > 0) state.textContent = `${article.orphans} stale`;
	else if (article.gaps.length > 0 || article.summaryGaps.length > 0) state.textContent = 'Gaps';

	const modified = element('span', 'row-modified');
	if (article.modified !== null) modified.textContent = formatDate(article.modified);

	summary.append(identity, section, segments, state, modified);
	summary.addEventListener('click', () => {
		opened = opened === article.path ? null : article.path;
		draw();
	});

	row.appendChild(summary);
	if (opened === article.path) row.appendChild(detail(article, locales));
	return row;
}

function draw(): void {
	if (listing === null) return;
	const root = requiredElement<HTMLElement>(document, '[data-articles]');
	const behind = listing.articles.filter(outstanding);

	// Outstanding first, then most recently touched. Sorting by date alone buries the work under
	// whatever was edited last, which is the arrangement this page had and the reason it read as
	// an index rather than a queue.
	const shown = (filter === 'attention' ? behind : listing.articles).toSorted((a, b) => {
		if (outstanding(a) !== outstanding(b)) return outstanding(a) ? -1 : 1;
		return (b.modified ?? '').localeCompare(a.modified ?? '');
	});

	const lede = requiredElement<HTMLElement>(root, '[data-articles-state]');
	const stale = behind.reduce((total, article) => total + article.orphans, 0);
	lede.textContent =
		behind.length === 0
			? 'Every article is translated, summarised and current.'
			: `${stale} stale ${stale === 1 ? 'segment' : 'segments'} across ${behind.length} ${behind.length === 1 ? 'article' : 'articles'}.`;
	lede.dataset.state = behind.length === 0 ? 'ready' : 'attention';

	const meta = requiredElement<HTMLElement>(root, '[data-articles-total]');
	const total = listing.articles.reduce((sum, article) => sum + article.segments, 0);
	meta.replaceChildren(
		element('span', '', `${listing.articles.length} articles`),
		element('span', '', `${total} segments`),
		element('span', '', `${listing.locales.length} locales`),
	);

	for (const control of root.querySelectorAll<HTMLButtonElement>('[data-filter]')) {
		const active = control.dataset.filter === filter;
		control.setAttribute('aria-pressed', active ? 'true' : 'false');
	}

	const list = requiredElement<HTMLElement>(root, '[data-articles-list]');
	list.replaceChildren();
	if (shown.length === 0) {
		list.appendChild(element('p', 'ledger-empty', 'Nothing needs a pass.'));
		return;
	}
	for (const article of shown) list.appendChild(renderRow(article, listing.locales));
}

export function renderArticles(root: HTMLElement, next: ArticleListing): void {
	listing = next;
	for (const control of root.querySelectorAll<HTMLButtonElement>('[data-filter]')) {
		control.addEventListener('click', () => {
			filter = control.dataset.filter === 'attention' ? 'attention' : 'all';
			draw();
		});
	}
	draw();
}

export function renderArticlesError(root: HTMLElement, error: unknown): void {
	const lede = requiredElement<HTMLElement>(root, '[data-articles-state]');
	lede.textContent = error instanceof Error ? error.message : String(error);
	lede.dataset.state = 'error';
	requiredElement<HTMLElement>(root, '[data-articles-total]').replaceChildren();
	requiredElement<HTMLElement>(root, '[data-articles-list]').replaceChildren();
}
