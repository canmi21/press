import { invoke } from '@tauri-apps/api/core';
import { requiredElement } from './dom';
import type { SegmentOutline, SegmentRow } from './segments';
import { animateHeight, animateWidth, slideIndicator } from './motion';
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
type Mark =
	| 'status'
	| 'section'
	| 'todo'
	| 'progress'
	| 'complete'
	| 'more'
	| 'check'
	| 'dash'
	| 'sort';

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
const opened = new Set<string>();
let menu: string | null = null;
/** Whether the pointer is over the sort control while a column holds the ordering. */
let offeringReset = false;
/** Handed in by the shell: show one article's segments on the page that reads them. */
let readSegments: ((article: { path: string; title: string }) => void) | null = null;
const collapsed = new Set<string>();
const selected = new Set<string>();
/** An article's contents, once it has been opened. Keyed by article path. */
const outlines = new Map<string, SegmentOutline>();
/** Segments ticked for deletion, keyed the same way. */
const markedSegments = new Set<string>();

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

/**
 * Whether this article has anything a sweep would take.
 *
 * Stale segments are the only thing an article carries that deleting fixes. Missing translations
 * and missing summaries are work to be *made*, and no sweep makes them.
 */
function sweepable(article: Article): boolean {
	return article.orphans > 0;
}

/**
 * What an article opens into.
 *
 * Only what can be acted on here, and one way to the rest. A stale segment is the one thing an
 * article carries that this page can do something about, so those are listed, each with a tick
 * and a Drop. The paragraphs the article still has are not listed: a hundred and forty rows of
 * first lines said nothing a reader could use, and reading them is the Segments page's job, so
 * the panel ends in the door to it.
 */
function detail(article: Article): HTMLElement {
	const panel = element('div', 'row-detail');

	if (sweepable(article)) {
		const outline = outlines.get(article.path);
		if (outline === undefined) {
			panel.appendChild(element('p', 'row-note', 'Reading the article…'));
			void invoke<SegmentOutline>('article_segments', { article: article.path })
				.then((next) => {
					outlines.set(article.path, next);
					draw();
				})
				.catch((error: unknown) => showError(error));
		} else {
			for (const row of outline.rows) {
				if (row.stale) panel.appendChild(staleRow(article, row));
			}
		}
	}

	const read = document.createElement('button');
	read.type = 'button';
	read.className = 'control row-read';
	read.textContent = `Read ${article.segments} ${article.segments === 1 ? 'segment' : 'segments'}`;
	read.addEventListener('click', (event) => {
		event.stopPropagation();
		readSegments?.({ path: article.path, title: article.title });
	});
	panel.appendChild(read);
	return panel;
}

/** A translation the article no longer has a paragraph for: tick it, or drop it on its own. */
function staleRow(article: Article, row: SegmentRow): HTMLElement {
	const key = `${article.path}#${row.id}`;
	const item = element('div', 'segment');

	const tick = document.createElement('button');
	tick.type = 'button';
	tick.className = 'checkbox';
	tick.setAttribute('role', 'checkbox');
	tick.setAttribute('aria-label', 'Select this segment');
	paintTick(tick, markedSegments.has(key) ? 'true' : 'false');
	tick.addEventListener('click', (event) => {
		event.stopPropagation();
		if (markedSegments.has(key)) markedSegments.delete(key);
		else markedSegments.add(key);
		draw();
	});
	item.appendChild(tick);

	item.appendChild(element('span', 'segment-text', row.preview ?? row.source ?? '(no text)'));
	item.appendChild(
		element('span', 'segment-locales', `${row.locales.length} ${row.locales.length === 1 ? 'locale' : 'locales'}`),
	);

	const drop = document.createElement('button');
	drop.type = 'button';
	drop.className = 'segment-drop';
	drop.textContent = 'Drop';
	drop.title = 'Deletes this translation, which the article no longer has a paragraph for.';
	drop.addEventListener('click', (event) => {
		event.stopPropagation();
		const ticked = [...markedSegments]
			.filter((marked) => marked.startsWith(`${article.path}#`))
			.map((marked) => marked.slice(article.path.length + 1));
		dropSegments(article, ticked.length > 0 ? ticked : [row.id]);
	});
	item.appendChild(drop);

	return item;
}

function showError(error: unknown): void {
	const notice = document.querySelector<HTMLElement>('[data-articles-error]');
	if (notice === null) return;
	notice.hidden = false;
	notice.textContent = error instanceof Error ? error.message : String(error);
}

/** Delete named segments from one article, then read both the article and the library back. */
function dropSegments(article: Article, ids: string[]): void {
	if (ids.length === 0) return;
	void invoke<number>('drop_segments', { article: article.path, ids })
		.then(() => {
			for (const id of ids) markedSegments.delete(`${article.path}#${id}`);
			outlines.delete(article.path);
			return invoke<ArticleListing>('article_listing');
		})
		.then((next) => {
			listing = next;
			draw();
		})
		.catch((error: unknown) => showError(error));
}

type Action = { name: string; why: string; run?: () => void };

function actionMenu(entries: Action[]): HTMLElement {
	const panel = element('div', 'menu');
	panel.dataset.menu = 'actions';
	for (const entry of entries) {
		const option = document.createElement('button');
		option.type = 'button';
		option.className = 'menu-option';
		option.textContent = entry.name;
		option.title = entry.why;
		if (entry.run === undefined) option.disabled = true;
		else {
			option.addEventListener('click', (event) => {
				event.stopPropagation();
				menu = null;
				entry.run?.();
			});
		}
		panel.appendChild(option);
	}
	return panel;
}

/** Dots rather than a named button: there will be more than one action, and Run repeated down a
 *  column was the same word said six times. */
function dots(key: string, entries: Action[]): HTMLElement {
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

type Tick = 'true' | 'false' | 'mixed';

/** A tick's whole appearance, in one place: three controls now put a box into these states. */
function paintTick(box: HTMLElement, state: Tick): void {
	box.setAttribute('aria-checked', state);
	if (state === 'true') box.replaceChildren(mark('check'));
	else if (state === 'mixed') box.replaceChildren(mark('dash'));
	else box.replaceChildren();
}

/**
 * Bring a group's select-all into line with the rows under it.
 *
 * Called after any tick rather than redrawing, so the header follows individual changes -- which
 * is what makes "select all, then untick the two I do not want" work: the header drops to mixed
 * instead of fighting the rows back to full.
 */
function syncSelectAll(group: Element | null): void {
	if (group === null) return;
	const head = group.querySelector<HTMLElement>('.select-all');
	if (head === null) return;
	const boxes = [...group.querySelectorAll<HTMLElement>('.row .checkbox')];
	const ticked = boxes.filter((box) => box.getAttribute('aria-checked') === 'true').length;
	paintTick(head, ticked === 0 ? 'false' : ticked === boxes.length ? 'true' : 'mixed');
}

function checkbox(article: Article): HTMLElement {
	const box = document.createElement('button');
	box.type = 'button';
	box.className = 'checkbox';
	box.setAttribute('role', 'checkbox');
	box.setAttribute('aria-label', `Select ${article.title}`);
	paintTick(box, selected.has(article.path) ? 'true' : 'false');
	box.addEventListener('click', (event) => {
		event.stopPropagation();
		// Updated in place rather than by redrawing. A redraw would drop the row under the pointer
		// and, in a group still collapsing, destroy the element its height animation is driving.
		if (selected.has(article.path)) selected.delete(article.path);
		else selected.add(article.path);
		const on = selected.has(article.path);
		paintTick(box, on ? 'true' : 'false');
		box.closest('.row')?.toggleAttribute('data-selected', on);
		syncSelectAll(box.closest('.group'));
		// A group's menu names what it would act on, so a tick changes its wording.
		if (menu?.startsWith('group:') === true) draw();
	});
	return box;
}

function renderRow(article: Article): HTMLElement {
	const row = element('div', 'row');
	if (outstanding(article)) row.dataset.attention = '';
	if (selected.has(article.path)) row.dataset.selected = '';

	row.appendChild(checkbox(article));

	const open = document.createElement('button');
	open.type = 'button';
	open.className = 'row-open';
	open.setAttribute('aria-expanded', opened.has(article.path) ? 'true' : 'false');

	const line = element('span', 'row-detail-text', detailText(article));
	if (underway(article)) line.dataset.tone = 'underway';
	const modified = element('span', 'row-modified');
	if (article.modified !== null) modified.textContent = formatDate(article.modified);

	open.append(element('span', 'row-title', article.title), line, modified);
	row.appendChild(open);

	row.appendChild(
		dots(
			`row:${article.path}`,
			sweepable(article)
				? [
						{
							name: 'Sweep stale segments',
							why: `Deletes ${article.orphans} ${article.orphans === 1 ? 'translation' : 'translations'} for paragraphs this article no longer has.`,
							run: () => sweep([article.path]),
						},
					]
				: [{ name: 'Open in the editor', why: 'The editor is not built yet.' }],
		),
	);

	// Always built, and folded shut when closed. A panel that only exists while open cannot be
	// animated: the element the motion drives would be created and destroyed by the toggle itself.
	const panel = element('div', 'row-panel');
	panel.appendChild(detail(article));
	if (!opened.has(article.path)) panel.style.height = '0px';
	row.appendChild(panel);

	open.addEventListener('click', () => {
		// Not a redraw, for the reason folding a group is not: the panel has to survive the press.
		const opening = !opened.has(article.path);
		if (opening) opened.add(article.path);
		else opened.delete(article.path);
		open.setAttribute('aria-expanded', opening ? 'true' : 'false');
		animateHeight(panel, opening);
	});

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
function columnHeader(articles: Article[]): HTMLElement {
	const head = element('div', 'row-columns');

	const all = document.createElement('button');
	all.type = 'button';
	all.className = 'checkbox select-all';
	all.setAttribute('role', 'checkbox');
	all.setAttribute('aria-label', 'Select every article in this group');
	const ticked = articles.filter((article) => selected.has(article.path)).length;
	paintTick(all, ticked === 0 ? 'false' : ticked === articles.length ? 'true' : 'mixed');
	all.addEventListener('click', (event) => {
		event.stopPropagation();
		// Anything short of everything means the press is asking for everything; only a full box
		// clears. That is what makes a mixed state actionable rather than a third thing to undo.
		const fill = all.getAttribute('aria-checked') !== 'true';
		const group = all.closest('.group');
		for (const article of articles) {
			if (fill) selected.add(article.path);
			else selected.delete(article.path);
		}
		for (const box of group?.querySelectorAll<HTMLElement>('.row .checkbox') ?? []) {
			paintTick(box, fill ? 'true' : 'false');
			box.closest('.row')?.toggleAttribute('data-selected', fill);
		}
		paintTick(all, fill ? 'true' : 'false');
		if (menu?.startsWith('group:') === true) draw();
	});
	head.appendChild(all);

	const inner = element('div', 'row-columns-inner');
	for (const [key, name, className] of COLUMNS) {
		const button = document.createElement('button');
		button.type = 'button';
		button.className = `col-sort ${className}`;
		button.appendChild(element('span', 'col-label', name));
		button.appendChild(mark('sort'));
		if (column === key) button.dataset.direction = ascending ? 'asc' : 'desc';
		button.addEventListener('click', () => {
			// Three states in a ring: off, up, down, off. The arrows are always drawn, so the ring
			// needs no reset control -- the way back out is one more press of the same heading.
			if (column !== key) {
				column = key;
				ascending = true;
			} else if (ascending) ascending = false;
			else column = null;
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
function groupEntries(name: string, articles: Article[]): Action[] {
	const ticked = articles.filter((article) => selected.has(article.path));
	const considered = ticked.length > 0 ? ticked : articles;
	const covered = considered.filter(sweepable);
	const scope = ticked.length > 0 ? `${considered.length} ticked` : `${considered.length} in ${name}`;
	const segments = covered.reduce((total, article) => total + article.orphans, 0);

	// Short enough not to wrap, and it still says which of the two things it would do. What it
	// would cover belongs in the title, where it can be as long as it needs to be.
	if (segments === 0) return [{ name: 'Sweep', why: `Nothing stale in the ${scope}.` }];
	return [
		{
			name: ticked.length > 0 ? 'Sweep selected' : `Sweep ${name}`,
			why: `Deletes ${segments} stale ${segments === 1 ? 'segment' : 'segments'} from ${covered.length} of the ${scope}.`,
			run: () => sweep(covered.map((article) => article.path)),
		},
	];
}

/**
 * Take the stale segments out of these articles, then read the library back.
 *
 * Not a task and not on a thread. The catalogue exists for work that takes minutes, asks a model,
 * or cannot safely run twice at once; this is a YAML rewrite per article under the record's own
 * lock, and giving it a progress bar would describe something nobody can watch. What it does need
 * is the listing again, because the numbers it just changed are on screen.
 */
function sweep(paths: string[]): void {
	if (paths.length === 0) return;
	void invoke<number>('sweep_segments', { articles: paths })
		.then(() => invoke<ArticleListing>('article_listing'))
		.then((next) => {
			listing = next;
			draw();
		})
		.catch((error: unknown) => {
			const notice = document.querySelector<HTMLElement>('[data-articles-error]');
			if (notice === null) return;
			notice.hidden = false;
			notice.textContent = error instanceof Error ? error.message : String(error);
		});
}

function renderGroup(key: string, name: string, shape: Mark, articles: Article[]): HTMLElement {
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
	panel.appendChild(columnHeader(articles));
	const body = element('div', 'group-rows');
	for (const article of articles) body.appendChild(renderRow(article));
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
	paintSortControl(root);

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

/**
 * The sort control, which is also where a column sort is undone.
 *
 * A column heading takes the ordering, so the button above stops naming a menu choice and says the
 * ordering came from elsewhere. That leaves no way back except pressing the same heading twice
 * more, which is a route somebody has to already know. Under the pointer it becomes the reset --
 * the one place the state is displayed is the place to offer its undo.
 *
 * The label changes width when it does, so the change goes through `animateWidth`: the control is
 * measured, pinned, driven and released rather than snapping between two sizes.
 */
function paintSortControl(root: HTMLElement): void {
	const control = requiredElement<HTMLButtonElement>(root, '[data-menu="sort"]');
	const wording = requiredElement<HTMLElement>(root, '[data-menu-label="sort"]');
	const slot = requiredElement<HTMLElement>(root, '[data-menu-icon="sort"]');
	const resetting = column !== null && offeringReset;
	const next = resetting
		? 'Cancel'
		: column === null
			? (SORTS.find(([value]) => value === sort)?.[1] ?? 'Recent')
			: 'By column';

	if (wording.textContent === next) return;
	animateWidth(control, () => {
		wording.textContent = next;
		// `toggleAttribute`, never `dataset.x = undefined`: assigning undefined to a dataset
		// property writes the string "undefined", which still matches `[data-reset]` -- so the
		// control stayed in its reset styling for good.
		control.toggleAttribute('data-reset', resetting);
		// `toggleAttribute`, not `.hidden = `, and typed as `Element` so nothing lets it be the
		// latter again: `hidden` is on HTMLElement's prototype and not on SVGElement's, so
		// assigning it to a mark sets a plain JavaScript property that reflects to nothing. The
		// icon stayed put while the label changed, and casting the mark to HTMLElement was what
		// let it compile.
		requiredElement<Element>(slot, '[data-mark="sort"]').toggleAttribute('hidden', resetting);
		requiredElement<Element>(slot, '[data-mark="reset"]').toggleAttribute('hidden', !resetting);
	});
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
				renderGroup(`section:${section}`, label(section), 'section', articles),
			);
		}
		return;
	}

	const running = ordered(kept.filter(underway));
	const behind = ordered(kept.filter((article) => outstanding(article) && !underway(article)));
	const current = ordered(kept.filter((article) => !outstanding(article) && !underway(article)));

	if (behind.length > 0) {
		list.appendChild(renderGroup('todo', 'Todo', 'todo', behind));
	}
	if (running.length > 0) {
		list.appendChild(renderGroup('progress', 'In Progress', 'progress', running));
	}
	if (current.length > 0) {
		list.appendChild(renderGroup('complete', 'Complete', 'complete', current));
	}
}

export function onReadSegments(open: (article: { path: string; title: string }) => void): void {
	readSegments = open;
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
			// Offering the reset means the press is the reset. Opening the menu underneath it would
			// be answering a different question from the one the button is currently asking.
			if (control.dataset.reset !== undefined) {
				column = null;
				offeringReset = false;
				menu = null;
				draw();
				return;
			}
			menu = menu === control.dataset.menu ? null : (control.dataset.menu ?? null);
			draw();
		});
	}

	const sortControl = root.querySelector<HTMLButtonElement>('[data-menu="sort"]');
	sortControl?.addEventListener('pointerenter', () => {
		if (column === null) return;
		offeringReset = true;
		paintSortControl(root);
	});
	sortControl?.addEventListener('pointerleave', () => {
		if (!offeringReset) return;
		offeringReset = false;
		paintSortControl(root);
	});
	document.addEventListener('click', () => {
		if (menu === null) return;
		menu = null;
		draw();
	});

	draw();
}

/**
 * Re-measure what only exists on screen.
 *
 * The tab indicator is positioned from its tab's box, and a hidden page has no box -- the same
 * reason the Overview re-fits its recent list when it is shown. Showing the library is the moment
 * that geometry exists.
 */
export function fitArticles(): void {
	const root = document.querySelector<HTMLElement>('[data-articles]');
	if (root === null) return;
	const active = root.querySelector<HTMLButtonElement>('[data-group][aria-selected="true"]');
	const indicator = root.querySelector<HTMLElement>('[data-tab-indicator]');
	if (active !== null && indicator !== null) slideIndicator(indicator, active);
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
