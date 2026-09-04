import { requiredElement } from './dom';
import type { TaskRun } from './derived';

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

type Grouping = 'status' | 'section';
type Filter = 'all' | 'attention' | 'current';
type Sort = 'recent' | 'longest' | 'title';
type Shape = 'attention' | 'underway' | 'current' | 'section';

/**
 * Which catalogue task closes which kind of finding.
 *
 * A run does not record the items it is working on, only its task id -- so an article cannot be
 * matched to a run exactly. What is knowable is coarser and still true: while `locale` is running,
 * the articles short of translations are the ones being worked on. Stated as a map rather than
 * inferred at the call site so that adding a task is one line here.
 */
const CLOSES: Record<string, (article: Article) => boolean> = {
	locale: (article) => article.gaps.length > 0 || article.orphans > 0,
	i18n: (article) => article.gaps.length > 0 || article.orphans > 0,
	summary: (article) => article.summaryGaps.length > 0,
};

let listing: ArticleListing | null = null;
let runs: TaskRun[] = [];
let grouping: Grouping = 'status';
let filter: Filter = 'all';
let sort: Sort = 'recent';
let opened: string | null = null;
let menu: string | null = null;

const FILTERS: Array<[Filter, string]> = [
	['all', 'Everything'],
	['attention', 'Needs a pass'],
	['current', 'Current only'],
];

const SORTS: Array<[Sort, string]> = [
	['recent', 'Recent'],
	['longest', 'Longest'],
	['title', 'Title'],
];

function outstanding(article: Article): boolean {
	return article.gaps.length > 0 || article.summaryGaps.length > 0 || article.orphans > 0;
}

/** Whether a live run is closing this article's kind of work right now. */
function underway(article: Article): boolean {
	return runs.some((run) => CLOSES[run.task]?.(article) === true);
}

function formatDate(value: string): string {
	return new Date(value).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
	});
}

function label(name: string): string {
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

/** A group's mark, cloned from the templates in index.html. */
function icon(shape: Shape): DocumentFragment {
	const template = document.querySelector<HTMLTemplateElement>(`[data-icon="${shape}"]`);
	if (template === null) throw new Error(`no icon template for ${shape}`);
	return template.content.cloneNode(true) as DocumentFragment;
}

function findings(article: Article): string[] {
	const found: string[] = [];
	const missing = article.gaps.reduce((total, gap) => total + gap.segments, 0);
	if (missing > 0) {
		const locales = article.gaps.map((gap) => gap.locale).join(', ');
		found.push(`${missing} ${missing === 1 ? 'segment' : 'segments'} untranslated in ${locales}`);
	}
	if (article.summaryGaps.length > 0) found.push(`No summary in ${article.summaryGaps.join(', ')}`);
	if (article.orphans > 0) {
		found.push(
			`${article.orphans} stale ${article.orphans === 1 ? 'segment' : 'segments'} from an edit`,
		);
	}
	return found;
}

function detail(article: Article, locales: string[]): HTMLElement {
	const panel = element('div', 'row-detail');
	if (article.subtitle !== null) panel.appendChild(element('p', 'row-subtitle', article.subtitle));
	panel.appendChild(element('p', 'row-path', article.path));

	const short = new Map(article.gaps.map((gap) => [gap.locale, gap.segments]));
	const coverage = element('div', 'locales');
	for (const locale of locales) {
		const missing = short.get(locale) ?? 0;
		const chip = element('span', 'locale', locale);
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

		// Text, not a button. An operation becomes a control once the task substrate can watch it
		// and refuse a second copy; `locale` and `summary` have not been lifted out of the CLI
		// adapter yet. See spec/architecture/cms.md.
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

	const summary = document.createElement('button');
	summary.type = 'button';
	summary.className = 'row-summary';
	summary.setAttribute('aria-expanded', opened === article.path ? 'true' : 'false');

	const title = element('span', 'row-title', article.title);
	const section = element('span', 'row-section', label(article.section));
	const segments = element('span', 'row-segments', String(article.segments));

	const state = element('span', 'row-state');
	if (underway(article)) {
		state.dataset.tone = 'underway';
		state.textContent = 'Running';
	} else if (article.orphans > 0) {
		state.dataset.tone = 'attention';
		state.textContent = `${article.orphans} stale`;
	} else if (article.gaps.length > 0 || article.summaryGaps.length > 0) {
		state.dataset.tone = 'attention';
		state.textContent = 'Gaps';
	}

	const modified = element('span', 'row-modified');
	if (article.modified !== null) modified.textContent = formatDate(article.modified);

	summary.append(title, section, segments, state, modified);
	summary.addEventListener('click', () => {
		opened = opened === article.path ? null : article.path;
		draw();
	});

	row.appendChild(summary);
	if (opened === article.path) row.appendChild(detail(article, locales));
	return row;
}

/** The header row naming each column, repeated per group so a long page never loses it. */
function columns(): HTMLElement {
	const head = element('div', 'row-columns');
	for (const [name, className] of [
		['Article', 'col-title'],
		['Section', 'col-section'],
		['Segments', 'col-segments'],
		['State', 'col-state'],
		['Modified', 'col-modified'],
	] as const) {
		head.appendChild(element('span', className, name));
	}
	return head;
}

function renderGroup(
	name: string,
	shape: Shape,
	articles: Article[],
	locales: string[],
): HTMLElement {
	const group = element('section', 'group');
	group.dataset.shape = shape;

	const head = element('header', 'group-head');
	head.appendChild(icon(shape));
	head.appendChild(element('span', 'group-name', name));
	head.appendChild(element('span', 'group-count', String(articles.length)));
	group.appendChild(head);

	group.appendChild(columns());
	const body = element('div', 'group-rows');
	for (const article of articles) body.appendChild(renderRow(article, locales));
	group.appendChild(body);
	return group;
}

function ordered(articles: Article[]): Article[] {
	return articles.toSorted((a, b) => {
		if (sort === 'longest') return b.segments - a.segments;
		if (sort === 'title') return a.title.localeCompare(b.title);
		return (b.modified ?? '').localeCompare(a.modified ?? '');
	});
}

function menuPanel(
	kind: 'filter' | 'sort',
	options: Array<[string, string]>,
	active: string,
	choose: (value: string) => void,
): HTMLElement {
	const panel = element('div', 'menu');
	for (const [value, name] of options) {
		const option = document.createElement('button');
		option.type = 'button';
		option.className = 'menu-option';
		option.textContent = name;
		if (value === active) option.dataset.active = '';
		option.addEventListener('click', () => {
			choose(value);
			menu = null;
			draw();
		});
		panel.appendChild(option);
	}
	panel.dataset.menu = kind;
	return panel;
}

function draw(): void {
	if (listing === null) return;
	const root = requiredElement<HTMLElement>(document, '[data-articles]');

	for (const tab of root.querySelectorAll<HTMLButtonElement>('[data-group]')) {
		tab.setAttribute('aria-selected', tab.dataset.group === grouping ? 'true' : 'false');
	}
	requiredElement<HTMLElement>(root, '[data-menu-label="filter"]').textContent =
		FILTERS.find(([value]) => value === filter)?.[1] ?? 'Filter';
	requiredElement<HTMLElement>(root, '[data-menu-label="sort"]').textContent =
		SORTS.find(([value]) => value === sort)?.[1] ?? 'Recent';

	for (const anchor of root.querySelectorAll<HTMLElement>('.menu-anchor')) {
		const control = requiredElement<HTMLButtonElement>(anchor, '[data-menu]');
		const kind = control.dataset.menu;
		anchor.querySelector('.menu')?.remove();
		control.setAttribute('aria-expanded', menu === kind ? 'true' : 'false');
		if (menu !== kind) continue;
		anchor.appendChild(
			kind === 'filter'
				? menuPanel('filter', FILTERS, filter, (value) => {
						filter = value as Filter;
					})
				: menuPanel('sort', SORTS, sort, (value) => {
						sort = value as Sort;
					}),
		);
	}

	const kept = listing.articles.filter((article) => {
		if (filter === 'attention') return outstanding(article);
		if (filter === 'current') return !outstanding(article);
		return true;
	});

	const list = requiredElement<HTMLElement>(root, '[data-articles-list]');
	list.replaceChildren();

	if (kept.length === 0) {
		list.appendChild(element('p', 'group-empty', 'Nothing here.'));
		return;
	}

	if (grouping === 'section') {
		const sections = new Map<string, Article[]>();
		for (const article of ordered(kept)) {
			const existing = sections.get(article.section);
			if (existing === undefined) sections.set(article.section, [article]);
			else existing.push(article);
		}
		for (const [section, articles] of sections) {
			list.appendChild(renderGroup(label(section), 'section', articles, listing.locales));
		}
		return;
	}

	// Status grouping is the default because it is the one that answers "what do I do next". An
	// empty group is still drawn when a run could fill it, so the page does not appear to change
	// shape when work starts.
	const running = ordered(kept.filter(underway));
	const behind = ordered(kept.filter((article) => outstanding(article) && !underway(article)));
	const current = ordered(kept.filter((article) => !outstanding(article) && !underway(article)));

	if (behind.length > 0) {
		list.appendChild(renderGroup('Needs a pass', 'attention', behind, listing.locales));
	}
	if (running.length > 0) {
		list.appendChild(renderGroup('In progress', 'underway', running, listing.locales));
	}
	if (current.length > 0) {
		list.appendChild(renderGroup('Current', 'current', current, listing.locales));
	}
}

export function renderArticles(root: HTMLElement, next: ArticleListing): void {
	listing = next;

	for (const tab of root.querySelectorAll<HTMLButtonElement>('[data-group]')) {
		tab.addEventListener('click', () => {
			grouping = tab.dataset.group === 'section' ? 'section' : 'status';
			draw();
		});
	}
	for (const control of root.querySelectorAll<HTMLButtonElement>('[data-menu]')) {
		control.addEventListener('click', (event) => {
			event.stopPropagation();
			menu = menu === control.dataset.menu ? null : (control.dataset.menu ?? null);
			draw();
		});
	}
	// A menu closes on the next click anywhere else, which is the platform behaviour and cheaper
	// than a backdrop element that would also have to be kept out of the tab order.
	document.addEventListener('click', () => {
		if (menu === null) return;
		menu = null;
		draw();
	});

	draw();
}

/** Live runs, so the status grouping can show what is being worked on right now. */
export function renderArticleRuns(next: TaskRun[]): void {
	runs = next;
	draw();
}

export function renderArticlesError(root: HTMLElement, error: unknown): void {
	const notice = requiredElement<HTMLElement>(root, '[data-articles-error]');
	notice.hidden = false;
	notice.textContent = error instanceof Error ? error.message : String(error);
	requiredElement<HTMLElement>(root, '[data-articles-list]').replaceChildren();
}
