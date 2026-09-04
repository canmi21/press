import { requiredElement } from './dom';
import { animateHeight, slideIndicator } from './motion';
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
type Column = 'title' | 'detail' | 'modified';
type Mark = 'status' | 'section' | 'todo' | 'progress' | 'complete' | 'more' | 'check';

/**
 * Which catalogue task closes which kind of finding.
 *
 * A run records its task id and not the items it is working on, so an article cannot be matched to
 * a run exactly. What is knowable is coarser and still true: while `locale` runs, the articles
 * short of translations are the ones being worked on.
 */
const CLOSES: Record<string, (article: Article) => boolean> = {
	locale: (article) => article.gaps.length > 0 || article.orphans > 0,
	i18n: (article) => article.gaps.length > 0 || article.orphans > 0,
	summary: (article) => article.summaryGaps.length > 0,
};

const FILTERS: Array<[Filter, string]> = [
	['all', 'Everything'],
	['attention', 'Todo'],
	['current', 'Complete only'],
];

const SORTS: Array<[Sort, string]> = [
	['recent', 'Recent'],
	['longest', 'Longest'],
	['title', 'Title'],
];

const COLUMNS: Array<[Column, string, string]> = [
	['title', 'Article', 'col-title'],
	['detail', 'Detail', 'col-detail'],
	['modified', 'Modified', 'col-modified'],
];

let listing: ArticleListing | null = null;
let runs: TaskRun[] = [];
let grouping: Grouping = 'status';
let filter: Filter = 'all';
let sort: Sort = 'recent';
/** The column a press put the ordering on, or nothing while the menu above still owns it. */
let column: Column | null = null;
let ascending = true;
let opened: string | null = null;
let menu: string | null = null;
const collapsed = new Set<string>();
const selected = new Set<string>();

function outstanding(article: Article): boolean {
	return article.gaps.length > 0 || article.summaryGaps.length > 0 || article.orphans > 0;
}

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

function mark(name: Mark): DocumentFragment {
	const template = document.querySelector<HTMLTemplateElement>(`[data-icon="${name}"]`);
	if (template === null) throw new Error(`no icon template for ${name}`);
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

/** The line a row shows under Detail, which is also what that column sorts on. */
function detailText(article: Article): string {
	if (underway(article)) return 'Running now';
	const found = findings(article);
	if (found.length > 0) return found.join(', ');
	return `${article.segments} segments, all current`;
}

function commandFor(article: Article): string {
	return article.orphans > 0 || article.gaps.length > 0 ? 'cms locale' : 'cms summary';
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
	return panel;
}

/**
 * The menu behind a row of dots.
 *
 * Every entry is disabled and says why. The operations have not moved below both shells, so the
 * task substrate cannot watch one or refuse a second copy -- and an entry that starts nothing is
 * the half of a run mechanism that lies. The shape is here so that enabling one later changes no
 * layout. See spec/architecture/cms.md.
 */
function actionMenu(entries: Array<[string, string]>): HTMLElement {
	const panel = element('div', 'menu');
	panel.dataset.menu = 'actions';
	for (const [name, why] of entries) {
		const option = document.createElement('button');
		option.type = 'button';
		option.className = 'menu-option';
		option.textContent = name;
		option.disabled = true;
		option.title = why;
		panel.appendChild(option);
	}
	return panel;
}

/** Dots rather than a named button: there will be more than one action, and Run repeated down a
 *  column was the same word said six times. */
function dots(key: string, entries: Array<[string, string]>): HTMLElement {
	const anchor = element('div', 'menu-anchor');
	const button = document.createElement('button');
	button.type = 'button';
	button.className = 'dots';
	button.setAttribute('aria-label', 'Actions');
	button.setAttribute('aria-expanded', menu === key ? 'true' : 'false');
	button.appendChild(mark('more'));
	button.addEventListener('click', (event) => {
		event.stopPropagation();
		menu = menu === key ? null : key;
		draw();
	});
	anchor.appendChild(button);
	if (menu === key) anchor.appendChild(actionMenu(entries));
	return anchor;
}

function checkbox(article: Article): HTMLElement {
	const box = document.createElement('button');
	box.type = 'button';
	box.className = 'checkbox';
	box.setAttribute('role', 'checkbox');
	box.setAttribute('aria-checked', selected.has(article.path) ? 'true' : 'false');
	box.setAttribute('aria-label', `Select ${article.title}`);
	if (selected.has(article.path)) box.appendChild(mark('check'));
	box.addEventListener('click', (event) => {
		event.stopPropagation();
		// Updated in place rather than by redrawing. A redraw would drop the row under the pointer
		// and, in a group still collapsing, destroy the element its height animation is driving.
		if (selected.has(article.path)) selected.delete(article.path);
		else selected.add(article.path);
		const on = selected.has(article.path);
		box.setAttribute('aria-checked', on ? 'true' : 'false');
		box.replaceChildren(...(on ? [mark('check')] : []));
		box.closest('.row')?.toggleAttribute('data-selected', on);
		// A group's menu names what it would act on, so a tick changes its wording.
		if (menu?.startsWith('group:') === true) draw();
	});
	return box;
}

function renderRow(article: Article, locales: string[]): HTMLElement {
	const row = element('div', 'row');
	if (outstanding(article)) row.dataset.attention = '';
	if (selected.has(article.path)) row.dataset.selected = '';

	row.appendChild(checkbox(article));

	const open = document.createElement('button');
	open.type = 'button';
	open.className = 'row-open';
	open.setAttribute('aria-expanded', opened === article.path ? 'true' : 'false');

	const line = element('span', 'row-detail-text', detailText(article));
	if (underway(article)) line.dataset.tone = 'underway';
	else if (outstanding(article)) line.dataset.tone = 'attention';
	const modified = element('span', 'row-modified');
	if (article.modified !== null) modified.textContent = formatDate(article.modified);

	open.append(element('span', 'row-title', article.title), line, modified);
	open.addEventListener('click', () => {
		opened = opened === article.path ? null : article.path;
		draw();
	});
	row.appendChild(open);

	const command = commandFor(article);
	row.appendChild(
		dots(`row:${article.path}`, [
			[`Run ${command}`, `Not runnable from here yet -- \`${command}\` closes this in a terminal.`],
			['Open in the editor', 'The editor is not built yet.'],
		]),
	);

	if (opened === article.path) row.appendChild(detail(article, locales));
	return row;
}

/**
 * The column names, which are also the sort control.
 *
 * One column orders the list at a time and the last one pressed is the one in force: pressing a
 * second moves the ordering onto it rather than adding a tie-break nobody asked for. Pressing the
 * column already in force reverses it, and with none pressed the menu above decides, which is what
 * default means here.
 */
function columnHeader(): HTMLElement {
	const head = element('div', 'row-columns');
	head.appendChild(element('span', 'col-select'));

	const inner = element('div', 'row-columns-inner');
	for (const [key, name, className] of COLUMNS) {
		const button = document.createElement('button');
		button.type = 'button';
		button.className = `col-sort ${className}`;
		button.textContent = name;
		if (column === key) button.dataset.direction = ascending ? 'asc' : 'desc';
		button.addEventListener('click', () => {
			if (column === key) ascending = !ascending;
			else {
				column = key;
				// A date reads newest first on a first press and text reads A to Z. Both are what
				// somebody expects, and they are opposite directions.
				ascending = key !== 'modified';
			}
			draw();
		});
		inner.appendChild(button);
	}
	head.appendChild(inner);
	head.appendChild(element('span', 'col-actions'));
	return head;
}

function ordered(articles: Article[]): Article[] {
	if (column === null) {
		return articles.toSorted((a, b) => {
			if (sort === 'longest') return b.segments - a.segments;
			if (sort === 'title') return a.title.localeCompare(b.title);
			return (b.modified ?? '').localeCompare(a.modified ?? '');
		});
	}
	const direction = ascending ? 1 : -1;
	const held = column;
	return articles.toSorted((a, b) => {
		if (held === 'modified') {
			return direction * (a.modified ?? '').localeCompare(b.modified ?? '');
		}
		if (held === 'detail') return direction * detailText(a).localeCompare(detailText(b));
		return direction * a.title.localeCompare(b.title);
	});
}

/**
 * What a group's own menu would act on.
 *
 * Nothing ticked means the whole group; ticks mean those. Said in the entry's own words rather
 * than left to be discovered, because one button with two readings is exactly what an interface
 * has to state out loud.
 */
function groupEntries(name: string, articles: Article[]): Array<[string, string]> {
	const ticked = articles.filter((article) => selected.has(article.path));
	const scope = ticked.length > 0 ? `${ticked.length} selected` : `all ${articles.length} in ${name}`;
	return [
		[`Run for ${scope}`, 'Not runnable from here yet -- see the command on each article.'],
		['Open the first', 'The editor is not built yet.'],
	];
}

function renderGroup(
	key: string,
	name: string,
	shape: Mark,
	articles: Article[],
	locales: string[],
): HTMLElement {
	const group = element('section', 'group');

	const head = element('header', 'group-head');
	const badge = element('span', 'group-mark');
	badge.appendChild(mark(shape));
	head.appendChild(badge);

	const toggle = document.createElement('button');
	toggle.type = 'button';
	toggle.className = 'group-toggle';
	toggle.setAttribute('aria-expanded', collapsed.has(key) ? 'false' : 'true');
	toggle.append(
		element('span', 'group-name', name),
		element('span', 'group-count', String(articles.length)),
	);
	head.append(toggle, dots(`group:${key}`, groupEntries(name, articles)));
	group.appendChild(head);

	const panel = element('div', 'group-panel');
	panel.appendChild(columnHeader());
	const body = element('div', 'group-rows');
	for (const article of articles) body.appendChild(renderRow(article, locales));
	panel.appendChild(body);
	if (collapsed.has(key)) panel.style.height = '0px';
	group.appendChild(panel);

	toggle.addEventListener('click', () => {
		// Not a redraw: the panel has to survive so its height can be driven, and a rebuild
		// mid-flight would drop the very element the animation is holding.
		const opening = collapsed.has(key);
		if (opening) collapsed.delete(key);
		else collapsed.add(key);
		toggle.setAttribute('aria-expanded', opening ? 'true' : 'false');
		animateHeight(panel, opening);
	});

	return group;
}

function drawViewControls(root: HTMLElement): void {
	let active: HTMLButtonElement | null = null;
	for (const tab of root.querySelectorAll<HTMLButtonElement>('[data-group]')) {
		const chosen = tab.dataset.group === grouping;
		tab.setAttribute('aria-selected', chosen ? 'true' : 'false');
		if (chosen) active = tab;
	}
	const indicator = root.querySelector<HTMLElement>('[data-tab-indicator]');
	if (indicator !== null && active !== null) slideIndicator(indicator, active);

	requiredElement<HTMLElement>(root, '[data-menu-label="filter"]').textContent =
		FILTERS.find(([value]) => value === filter)?.[1] ?? 'Filter';
	requiredElement<HTMLElement>(root, '[data-menu-label="sort"]').textContent =
		column === null ? (SORTS.find(([value]) => value === sort)?.[1] ?? 'Recent') : 'By column';

	for (const anchor of root.querySelectorAll<HTMLElement>('.view-controls .menu-anchor')) {
		const control = requiredElement<HTMLButtonElement>(anchor, '[data-menu]');
		const kind = control.dataset.menu;
		anchor.querySelector('.menu')?.remove();
		control.setAttribute('aria-expanded', menu === kind ? 'true' : 'false');
		if (menu !== kind) continue;

		const panel = element('div', 'menu');
		const options = kind === 'filter' ? FILTERS : SORTS;
		for (const [value, name] of options) {
			const option = document.createElement('button');
			option.type = 'button';
			option.className = 'menu-option';
			option.textContent = name;
			const current = kind === 'filter' ? filter : column === null ? sort : null;
			if (current === value) option.dataset.active = '';
			option.addEventListener('click', () => {
				if (kind === 'filter') filter = value as Filter;
				else {
					sort = value as Sort;
					// Choosing here takes the ordering back off whichever column was holding it.
					column = null;
				}
				menu = null;
				draw();
			});
			panel.appendChild(option);
		}
		anchor.appendChild(panel);
	}
}

function draw(): void {
	if (listing === null) return;
	const root = requiredElement<HTMLElement>(document, '[data-articles]');
	drawViewControls(root);

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
			list.appendChild(
				renderGroup(`section:${section}`, label(section), 'section', articles, listing.locales),
			);
		}
		return;
	}

	const running = ordered(kept.filter(underway));
	const behind = ordered(kept.filter((article) => outstanding(article) && !underway(article)));
	const current = ordered(kept.filter((article) => !outstanding(article) && !underway(article)));

	if (behind.length > 0) {
		list.appendChild(renderGroup('todo', 'Todo', 'todo', behind, listing.locales));
	}
	if (running.length > 0) {
		list.appendChild(renderGroup('progress', 'In Progress', 'progress', running, listing.locales));
	}
	if (current.length > 0) {
		list.appendChild(renderGroup('complete', 'Complete', 'complete', current, listing.locales));
	}
}

export function renderArticles(root: HTMLElement, next: ArticleListing): void {
	listing = next;

	for (const slot of root.querySelectorAll<HTMLElement>('[data-tab-icon]')) {
		const name = slot.dataset.tabIcon;
		if (name === 'status' || name === 'section') slot.appendChild(mark(name));
	}

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
	document.addEventListener('click', () => {
		if (menu === null) return;
		menu = null;
		draw();
	});

	draw();
}

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
