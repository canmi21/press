import { followSystemTheme } from '@canmi/theme';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { renderArticles, renderArticlesError, type ArticleListing } from './articles';
import { renderDerived, renderDerivedError, type DerivedReport } from './derived';
import { renderOverview, renderOverviewError, type OverviewSnapshot } from './overview';
import './style.css';

followSystemTheme();

const pages = {
	overview: {
		title: 'Overview',
	},
	articles: {
		title: 'Articles',
	},
	media: {
		title: 'Media',
	},
	derived: {
		title: 'Derived',
	},
	automations: {
		title: 'Automations',
	},
	activity: {
		title: 'Activity',
	},
} as const;

type Page = keyof typeof pages;

function requiredElement<T extends Element>(selector: string): T {
	const element = document.querySelector<T>(selector);
	if (element === null) throw new Error(`required element is missing: ${selector}`);
	return element;
}

function pageOf(link: HTMLButtonElement): Page {
	const page = link.dataset.page;
	if (page === undefined || !(page in pages)) throw new Error('a page link is missing its page');
	return page as Page;
}

const pageLinks = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-page]'));
const pageLabel = requiredElement<HTMLElement>('[data-page-label]');
const overview = requiredElement<HTMLElement>('[data-overview]');
const articles = requiredElement<HTMLElement>('[data-articles]');
const derived = requiredElement<HTMLElement>('[data-derived]');

function selectPage(page: Page): void {
	const selected = pages[page];
	for (const link of pageLinks) {
		if (pageOf(link) === page) link.setAttribute('aria-current', 'page');
		else link.removeAttribute('aria-current');
	}

	pageLabel.setAttribute('aria-label', selected.title);
	overview.hidden = page !== 'overview';
	articles.hidden = page !== 'articles';
	derived.hidden = page !== 'derived';
	document.title = selected.title;
}

for (const link of pageLinks) {
	link.addEventListener('click', () => selectPage(pageOf(link)));
}

if ('__TAURI_INTERNALS__' in window) {
	void invoke<OverviewSnapshot>('overview_snapshot').then(
		(snapshot) => renderOverview(overview, snapshot),
		(error: unknown) => renderOverviewError(overview, error),
	);
	void invoke<ArticleListing>('article_listing').then(
		(listing) => renderArticles(articles, listing),
		(error: unknown) => renderArticlesError(articles, error),
	);
	void invoke<DerivedReport>('derived_report').then(
		(report) => renderDerived(derived, report),
		(error: unknown) => renderDerivedError(derived, error),
	);
} else {
	const absent = new Error('Workspace data is available in the desktop app.');
	renderOverviewError(overview, absent);
	renderArticlesError(articles, absent);
	renderDerivedError(derived, absent);
}

if ('__TAURI_INTERNALS__' in window) {
	const currentWindow = getCurrentWindow();
	let requestedTitle: string | undefined;
	const syncTitle = () => {
		if (document.title === requestedTitle) return;
		requestedTitle = document.title;
		void currentWindow.setTitle(document.title);
	};

	syncTitle();
	new MutationObserver(syncTitle).observe(document.head, {
		childList: true,
		characterData: true,
		subtree: true,
	});
}
