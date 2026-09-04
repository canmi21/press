import { endonym } from '@canmi/locales';
import { invoke } from '@tauri-apps/api/core';
import { requiredElement } from './dom';
import { animateHeight } from './motion';
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

/** The article being read, and its title for the heading. */
let article: { path: string; title: string } | null = null;
let outline: SegmentOutline | null = null;
const details = new Map<string, SegmentDetail>();
const opened = new Set<string>();
let returnToLibrary: (() => void) | null = null;

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

/**
 * Prose, rendered.
 *
 * A segment is markdown, and reading eight translations of one paragraph through the punctuation
 * that marks up the ninth is reading the wrong thing. Inserted rather than escaped, which is safe
 * because the renderer emits no raw HTML and strips the one thing it would otherwise pass through
 * -- see prose.ts, where that is measured rather than assumed.
 */
function prose(className: string, markdown: string): HTMLElement {
	const node = element('div', className);
	node.innerHTML = renderMarkdown(markdown);
	return node;
}

/**
 * One segment, with its source and every translation of it beneath.
 *
 * The comparison is the point of the page, so a rendering names its language the way that
 * language writes it -- `ko-KR` tells a writer nothing they were asking, and 한국어 tells them
 * immediately. Names come from `@canmi/locales`, shared with the site's own picker.
 */
function renderSegment(row: SegmentRow): HTMLElement {
	const item = element('section', 'reading');
	if (row.stale) item.dataset.stale = '';

	const head = document.createElement('button');
	head.type = 'button';
	head.className = 'reading-head';
	head.setAttribute('aria-expanded', opened.has(row.id) ? 'true' : 'false');
	head.appendChild(
		element('span', 'reading-lead', row.source ?? row.preview ?? '(no text)'),
	);
	head.appendChild(
		element('span', 'reading-count', `${row.locales.length} ${row.stale ? 'kept' : 'locales'}`),
	);
	item.appendChild(head);

	const panel = element('div', 'reading-panel');
	if (!opened.has(row.id)) panel.style.height = '0rem';
	panel.appendChild(renderBody(row));
	item.appendChild(panel);

	head.addEventListener('click', () => {
		const opening = !opened.has(row.id);
		if (opening) opened.add(row.id);
		else opened.delete(row.id);
		head.setAttribute('aria-expanded', opening ? 'true' : 'false');
		if (opening && !details.has(row.id)) {
			void load(row.id).then(() => {
				panel.replaceChildren(renderBody(row));
				animateHeight(panel, true);
			}, fail);
		} else {
			animateHeight(panel, opening);
		}
	});

	return item;
}

function renderBody(row: SegmentRow): HTMLElement {
	const body = element('div', 'reading-body');
	const detail = details.get(row.id);
	if (detail === undefined) {
		body.appendChild(element('p', 'reading-note', 'Reading…'));
		return body;
	}

	if (detail.source !== null) {
		const pane = element('div', 'reading-pane');
		pane.dataset.origin = '';
		pane.appendChild(element('span', 'reading-language', 'Original'));
		pane.appendChild(prose('reading-prose', detail.source));
		body.appendChild(pane);
	} else {
		body.appendChild(
			element(
				'p',
				'reading-note',
				'The paragraph this translated is no longer in the article, so there is nothing to compare against.',
			),
		);
	}

	for (const rendering of detail.renderings) {
		const pane = element('div', 'reading-pane');
		pane.appendChild(element('span', 'reading-language', endonym(rendering.locale)));
		pane.appendChild(prose('reading-prose', rendering.text));
		pane.appendChild(
			element('span', 'reading-meta', `${rendering.model} · ${rendering.tokens} tokens`),
		);
		body.appendChild(pane);
	}
	return body;
}

async function load(id: string): Promise<void> {
	if (article === null) return;
	const detail = await invoke<SegmentDetail>('segment_detail', { article: article.path, id });
	details.set(id, detail);
}

function draw(): void {
	const shell = root();
	const title = requiredElement<HTMLElement>(shell, '[data-segments-title]');
	const back = requiredElement<HTMLButtonElement>(shell, '[data-segments-back]');
	const list = requiredElement<HTMLElement>(shell, '[data-segments-list]');

	title.textContent = article === null ? 'Segments' : article.title;
	back.hidden = article === null;
	list.replaceChildren();

	if (article === null) {
		list.appendChild(
			element(
				'p',
				'reading-empty',
				'Open an article from the library to read what it is made of.',
			),
		);
		return;
	}
	if (outline === null) {
		list.appendChild(element('p', 'reading-note', 'Reading the article…'));
		return;
	}

	const stale = outline.rows.filter((row) => row.stale);
	const live = outline.rows.filter((row) => !row.stale);
	// The ones with no paragraph left come first: they are the only text of themselves that
	// survives anywhere, so burying them under a hundred and forty live rows hides the reason
	// somebody came here.
	for (const row of [...stale, ...live]) list.appendChild(renderSegment(row));
}

/** Show one article's segments, and remember the way back. */
export function openArticleSegments(
	next: { path: string; title: string },
	back: () => void,
): void {
	article = next;
	returnToLibrary = back;
	outline = null;
	details.clear();
	opened.clear();
	requiredElement<HTMLElement>(root(), '[data-segments-error]').hidden = true;
	draw();

	void invoke<SegmentOutline>('article_segments', { article: next.path })
		.then((loaded) => {
			outline = loaded;
			draw();
		})
		.catch(fail);
}

export function registerSegments(): void {
	requiredElement<HTMLButtonElement>(root(), '[data-segments-back]').addEventListener(
		'click',
		() => returnToLibrary?.(),
	);
	draw();
}
