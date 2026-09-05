/**
 * The Segments page: one article, read a paragraph at a time against everything it became.
 *
 * Two panes. The roster on the left is every segment of the chosen article in the order the
 * article has them, one line each, and the study on the right is the one under the cursor: its
 * paragraph, then each translation of it. The roster is where a reader moves and the study is
 * where they stop, which is why the two scroll separately -- moving to the ninetieth segment must
 * not lose the roster's place, and reading a long translation must not scroll the roster away.
 *
 * The page chooses its own article. The library's rows send one here as a shortcut, and the menu
 * at the top does the same from inside the page, so nothing about this surface depends on where
 * somebody came from. With nothing chosen yet it opens the most recently written article rather
 * than asking, because an empty page that explains itself is still an empty page.
 */

import { endonym } from '@canmi/locales';
import { invoke } from '@tauri-apps/api/core';
import { reloadArticles, type ArticleListing } from './articles';
import { requiredElement } from './dom';
import { slideIndicator } from './motion';
import { renderMarkdown } from './prose';

export type SegmentRow = {
	id: string;
	stale: boolean;
	source: string | null;
	preview: string | null;
	locales: string[];
};

export type SegmentOutline = { article: string; rows: SegmentRow[] };

type Rendering = {
	locale: string;
	text: string;
	provider: string;
	model: string;
	at: string;
	tokens: number;
	review: boolean;
};

type SegmentDetail = {
	id: string;
	stale: boolean;
	source: string | null;
	renderings: Rendering[];
};

type Chosen = { path: string; title: string };
type Menu = 'article' | 'language';
/**
 * Which rows the roster lists. The views are the page's tabs, the way Status and Section are the
 * library's: all of them, the stale ones, or the ones short of a translation -- in the chosen
 * language, or in any language while every one of them is shown.
 */
type View = 'all' | 'stale' | 'missing';

let library: ArticleListing | null = null;
let article: Chosen | null = null;
let outline: SegmentOutline | null = null;
/** The segment in the study, by id. */
let current: string | null = null;
/** One locale to read against the original, or every one of them. */
let language: string | null = null;
let menu: Menu | null = null;
let view: View = 'all';
const details = new Map<string, SegmentDetail>();

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

function root(): HTMLElement {
	return requiredElement<HTMLElement>(document, '[data-segments]');
}

function fail(error: unknown): void {
	const notice = requiredElement<HTMLElement>(root(), '[data-segments-error]');
	notice.hidden = false;
	notice.textContent = error instanceof Error ? error.message : String(error);
}

function clearError(): void {
	requiredElement<HTMLElement>(root(), '[data-segments-error]').hidden = true;
}

/** Every row in reading order: the ones the article lost first, since they are why anyone sweeps. */
function allRows(): SegmentRow[] {
	if (outline === null) return [];
	return [...outline.rows.filter((row) => row.stale), ...outline.rows.filter((row) => !row.stale)];
}

/**
 * A live paragraph short of a translation: of the chosen language, or of any language while every
 * one is shown. A stale row has nothing to lack.
 */
function missing(row: SegmentRow): boolean {
	if (row.stale) return false;
	if (language !== null) return !row.locales.includes(language);
	return (library?.locales ?? []).some((locale) => !row.locales.includes(locale));
}

/** The rows the roster lists under the current view, in reading order. */
function rows(): SegmentRow[] {
	const listed = allRows();
	if (view === 'stale') return listed.filter((row) => row.stale);
	if (view === 'missing') return listed.filter(missing);
	return listed;
}

/**
 * One line of a segment with its markdown marks taken off.
 *
 * The roster is a place to recognise a paragraph, not to read it, and a first line seen through
 * backticks and asterisks is harder to recognise than the words. Only the marks that survive a
 * single line are handled; the study draws the segment properly.
 */
function plain(markdown: string): string {
	return markdown
		.replace(/^#{1,6}\s+/, '')
		.replace(/\{#[^}]*\}\s*$/, '')
		.replace(/[*_]{1,3}([^*_]+)[*_]{1,3}/g, '$1')
		.replace(/`([^`]+)`/g, '$1')
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1');
}

function count(total: number, noun: string): string {
	return `${total} ${total === 1 ? noun : `${noun}s`}`;
}

/**
 * Prose, rendered.
 *
 * A segment is markdown, and reading eight translations of one paragraph through the punctuation
 * that marks up the ninth is reading the wrong thing. Inserted rather than escaped, which is safe
 * because the renderer emits no raw HTML and strips the one thing it would otherwise pass through
 * -- see prose.ts, where that is measured rather than assumed.
 */
function prose(markdown: string): HTMLElement {
	const node = element('div', 'reading-prose');
	node.innerHTML = renderMarkdown(markdown);
	return node;
}

// ---- The two menus ------------------------------------------------------------------------------

function option(name: string, active: boolean, pick: () => void): HTMLButtonElement {
	const button = document.createElement('button');
	button.type = 'button';
	button.className = 'menu-option';
	button.textContent = name;
	if (active) button.dataset.active = '';
	button.addEventListener('click', (event) => {
		event.stopPropagation();
		menu = null;
		pick();
	});
	return button;
}

function drawHeader(): void {
	const shell = root();
	for (const tab of shell.querySelectorAll<HTMLButtonElement>('[data-view]')) {
		tab.setAttribute('aria-selected', tab.dataset.view === view ? 'true' : 'false');
	}
	fitSegments();

	requiredElement<HTMLElement>(shell, '[data-segments-label="article"]').textContent =
		article?.title ?? 'Article';
	requiredElement<HTMLElement>(shell, '[data-segments-label="language"]').textContent =
		language === null ? 'Every language' : endonym(language);

	for (const control of shell.querySelectorAll<HTMLButtonElement>('[data-segments-menu]')) {
		const kind = control.dataset.segmentsMenu as Menu;
		const anchor = control.parentElement;
		if (anchor === null) continue;
		anchor.querySelector('.menu')?.remove();
		control.setAttribute('aria-expanded', menu === kind ? 'true' : 'false');
		if (menu !== kind) continue;

		const panel = element('div', 'menu');
		if (kind === 'article') {
			const entries = [...(library?.articles ?? [])].sort((a, b) =>
				a.title.localeCompare(b.title, 'en-US'),
			);
			for (const entry of entries) {
				panel.appendChild(
					option(entry.title, entry.path === article?.path, () => {
						openArticleSegments({ path: entry.path, title: entry.title });
					}),
				);
			}
		} else {
			panel.appendChild(
				option('Every language', language === null, () => {
					language = null;
					settle();
					draw();
				}),
			);
			for (const locale of library?.locales ?? []) {
				panel.appendChild(
					option(endonym(locale), language === locale, () => {
						language = locale;
						settle();
						draw();
					}),
				);
			}
		}
		anchor.appendChild(panel);
	}
}

// ---- The roster ---------------------------------------------------------------------------------

function drawRoster(): void {
	const roster = requiredElement<HTMLElement>(root(), '[data-segments-roster]');
	roster.replaceChildren();
	if (outline === null) {
		roster.appendChild(element('p', 'reading-note', 'Reading the article…'));
		return;
	}

	const listed = rows();
	const caption =
		view === 'stale'
			? count(listed.length, 'stale segment')
			: view === 'missing'
				? `${count(listed.length, 'segment')} untranslated${language === null ? '' : ` in ${endonym(language)}`}`
				: count(listed.length, 'segment');
	roster.appendChild(element('div', 'roster-head', caption));

	if (listed.length === 0) {
		roster.appendChild(element('p', 'reading-note', 'Nothing under this view.'));
		return;
	}

	const live = allRows().filter((row) => !row.stale);
	for (const row of listed) {
		const line = document.createElement('button');
		line.type = 'button';
		line.className = 'roster-row';
		if (row.stale) line.dataset.stale = '';
		if (row.id === current) line.setAttribute('aria-current', 'true');
		line.dataset.segment = row.id;
		// Numbered by place in the article, which a filtered view keeps: the twelfth paragraph is
		// still the twelfth when it is the only one listed. A stale one has no place to number.
		const place = row.stale ? '' : String(live.indexOf(row) + 1);
		line.appendChild(element('span', 'roster-index', place));
		line.appendChild(element('span', 'roster-text', plain(row.source ?? row.preview ?? '(no text)')));
		if (row.stale) line.appendChild(element('span', 'roster-tag', 'stale'));
		line.addEventListener('click', () => select(row.id));
		roster.appendChild(line);
	}
}

/** Keep the study on a row the roster lists, moving it to the first one when the view hid it. */
function settle(): void {
	const listed = rows();
	if (listed.some((row) => row.id === current)) return;
	current = listed[0]?.id ?? null;
	if (current !== null && !details.has(current)) {
		const id = current;
		void load(id).then(() => {
			if (current === id) drawStudy();
		}, fail);
	}
}

/** Move the study to one segment. The roster is touched in place so it keeps its scroll. */
function select(id: string): void {
	current = id;
	const roster = requiredElement<HTMLElement>(root(), '[data-segments-roster]');
	for (const line of roster.querySelectorAll<HTMLElement>('.roster-row')) {
		if (line.dataset.segment === id) line.setAttribute('aria-current', 'true');
		else line.removeAttribute('aria-current');
	}
	drawStudy();
	if (!details.has(id)) {
		void load(id).then(() => {
			if (current === id) drawStudy();
		}, fail);
	}
}

/** Up and down walk the roster; the study follows. */
function step(direction: 1 | -1): void {
	const listed = rows();
	if (listed.length === 0) return;
	const at = listed.findIndex((row) => row.id === current);
	const next = listed[Math.min(listed.length - 1, Math.max(0, at + direction))];
	if (next === undefined || next.id === current) return;
	select(next.id);
	root()
		.querySelector<HTMLElement>(`.roster-row[data-segment="${next.id}"]`)
		?.scrollIntoView({ block: 'nearest' });
}

// ---- The study ----------------------------------------------------------------------------------

function pane(name: string, body: string, meta?: string): HTMLElement {
	const block = element('section', 'study-pane');
	const line = element('div', 'study-line');
	line.appendChild(element('span', 'study-language', name));
	if (meta !== undefined) line.appendChild(element('span', 'study-meta', meta));
	block.appendChild(line);
	block.appendChild(prose(body));
	return block;
}

function drawStudy(): void {
	const study = requiredElement<HTMLElement>(root(), '[data-segments-study]');
	study.replaceChildren();
	const every = allRows();
	const row = every.find((entry) => entry.id === current);
	if (row === undefined) return;

	const head = element('header', 'study-head');
	const place = every.filter((entry) => !entry.stale).findIndex((entry) => entry.id === row.id);
	head.appendChild(
		element('span', 'study-title', row.stale ? 'No longer in the article' : `Segment ${place + 1}`),
	);
	if (row.stale) {
		const drop = document.createElement('button');
		drop.type = 'button';
		drop.className = 'control study-drop';
		drop.textContent = `Drop ${count(row.locales.length, 'translation')}`;
		drop.title = 'Deletes these translations. The paragraph they belong to has been edited away.';
		drop.addEventListener('click', () => dropCurrent(row.id));
		head.appendChild(drop);
	} else {
		head.appendChild(element('span', 'study-id', row.id));
	}
	study.appendChild(head);

	const detail = details.get(row.id);
	if (detail === undefined) {
		study.appendChild(element('p', 'reading-note', 'Reading…'));
		return;
	}

	if (detail.source !== null) {
		const origin = pane('Original', detail.source);
		origin.dataset.origin = '';
		study.appendChild(origin);
	} else {
		study.appendChild(
			element(
				'p',
				'reading-note',
				'The paragraph these translated is no longer in the article, so there is nothing to compare them against.',
			),
		);
	}
	// Every language the corpus carries, in its order, and an absent one said rather than left as
	// space: a pane that is not there and a pane still loading look the same.
	const order = library?.locales ?? detail.renderings.map((rendering) => rendering.locale);
	for (const locale of order) {
		if (language !== null && locale !== language) continue;
		const rendering = detail.renderings.find((entry) => entry.locale === locale);
		if (rendering !== undefined) {
			study.appendChild(
				pane(
					endonym(locale),
					rendering.text,
					`${rendering.model}, ${count(rendering.tokens, 'token')}`,
				),
			);
			continue;
		}
		if (row.stale) continue;
		const block = element('section', 'study-pane');
		const line = element('div', 'study-line');
		line.appendChild(element('span', 'study-language', endonym(locale)));
		block.appendChild(line);
		block.appendChild(element('p', 'study-absent', 'Not translated yet.'));
		study.appendChild(block);
	}
}

async function load(id: string): Promise<void> {
	if (article === null) return;
	const detail = await invoke<SegmentDetail>('segment_detail', { article: article.path, id });
	details.set(id, detail);
}

/**
 * Delete one stale segment's translations, then read the article back.
 *
 * The study moves to the row that took the dropped one's place, so a reader sweeping several can
 * press the same control again rather than finding their way back to the roster each time.
 */
function dropCurrent(id: string): void {
	if (article === null) return;
	const chosen = article;
	const listed = rows();
	const at = listed.findIndex((row) => row.id === id);
	const successor = listed[at + 1] ?? listed[at - 1];
	void invoke<number>('drop_segments', { article: chosen.path, ids: [id] })
		.then(() => {
			details.delete(id);
			current = successor?.id ?? null;
			reloadArticles();
			return read(chosen);
		})
		.catch(fail);
}

async function read(chosen: Chosen): Promise<void> {
	const loaded = await invoke<SegmentOutline>('article_segments', { article: chosen.path });
	if (article?.path !== chosen.path) return;
	outline = loaded;
	if (current === null || !rows().some((row) => row.id === current)) {
		current = rows()[0]?.id ?? null;
	}
	draw();
	if (current !== null && !details.has(current)) {
		const id = current;
		await load(id);
		if (current === id) drawStudy();
	}
}

function draw(): void {
	drawHeader();
	drawRoster();
	drawStudy();
}

/** Show one article's segments. The library's rows call this as a shortcut past the menu. */
export function openArticleSegments(next: Chosen): void {
	if (article?.path === next.path && outline !== null) return;
	article = next;
	outline = null;
	current = null;
	view = 'all';
	details.clear();
	clearError();
	draw();
	void read(next).catch(fail);
}

/**
 * Re-place the bar under the tabs, which is measured from a box a hidden page does not have. The
 * shell calls this when the page is shown, for the same reason the library's is called then.
 */
export function fitSegments(): void {
	const shell = document.querySelector<HTMLElement>('[data-segments]');
	if (shell === null || shell.hidden) return;
	const active = shell.querySelector<HTMLButtonElement>('[data-view][aria-selected="true"]');
	const indicator = shell.querySelector<HTMLElement>('[data-tab-indicator]');
	if (active !== null && indicator !== null) slideIndicator(indicator, active);
}

export function registerSegments(): void {
	const shell = root();
	for (const tab of shell.querySelectorAll<HTMLButtonElement>('[data-view]')) {
		tab.addEventListener('click', () => {
			view = tab.dataset.view as View;
			settle();
			draw();
		});
	}
	for (const control of shell.querySelectorAll<HTMLButtonElement>('[data-segments-menu]')) {
		control.addEventListener('click', (event) => {
			event.stopPropagation();
			const kind = control.dataset.segmentsMenu as Menu;
			menu = menu === kind ? null : kind;
			drawHeader();
		});
	}
	document.addEventListener('click', () => {
		if (menu === null) return;
		menu = null;
		drawHeader();
	});
	requiredElement<HTMLElement>(shell, '[data-segments-roster]').addEventListener(
		'keydown',
		(event) => {
			if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
			event.preventDefault();
			step(event.key === 'ArrowDown' ? 1 : -1);
			shell.querySelector<HTMLElement>('.roster-row[aria-current]')?.focus();
		},
	);

	draw();
	void invoke<ArticleListing>('article_listing')
		.then((loaded) => {
			library = loaded;
			if (article !== null) {
				drawHeader();
				return;
			}
			// The most recently written article, which is the one most likely to be under review.
			const latest = [...loaded.articles].sort((a, b) =>
				(b.modified ?? '').localeCompare(a.modified ?? ''),
			)[0];
			if (latest !== undefined) openArticleSegments({ path: latest.path, title: latest.title });
		})
		.catch(fail);
}
