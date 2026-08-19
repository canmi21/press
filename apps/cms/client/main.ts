import { followSystemTheme } from '@canmi/theme';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { renderArticles, renderArticlesError, type ArticleListing } from './articles';
import {
	renderDerived,
	renderDerivedError,
	renderTaskRuns,
	type DerivedReport,
	type TaskRun,
} from './derived';
import { renderOverview, renderOverviewError, type OverviewSnapshot } from './overview';
import './style.css';
import { requiredElement } from './dom';

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

function pageOf(link: HTMLButtonElement): Page {
	const page = link.dataset.page;
	if (page === undefined || !(page in pages)) throw new Error('a page link is missing its page');
	return page as Page;
}

const pageLinks = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-page]'));
const pageLabel = requiredElement<HTMLElement>(document, '[data-page-label]');
const overview = requiredElement<HTMLElement>(document, '[data-overview]');
const articles = requiredElement<HTMLElement>(document, '[data-articles]');
const derived = requiredElement<HTMLElement>(document, '[data-derived]');
let liveTaskRuns: TaskRun[] = [];
let taskPoll: number | undefined;

// A one-second cadence keeps minute-long work visibly current without scanning the machine-wide
// lock directory more often than a person can use the updates.
const taskPollInterval = 1_000;

function showDerivedReport(report: DerivedReport): void {
	renderDerived(derived, report, liveTaskRuns, startTask);
}

function scheduleTaskPoll(): void {
	if (taskPoll !== undefined) window.clearTimeout(taskPoll);
	taskPoll = window.setTimeout(() => {
		taskPoll = undefined;
		void refreshTaskRuns();
	}, taskPollInterval);
}

async function refreshTaskRuns(): Promise<void> {
	const hadLiveRuns = liveTaskRuns.length > 0;
	try {
		liveTaskRuns = await invoke<TaskRun[]>('live_task_runs');
		renderTaskRuns(derived, liveTaskRuns);
		if (liveTaskRuns.length > 0) scheduleTaskPoll();
		else if (hadLiveRuns) {
			void invoke<DerivedReport>('derived_report').then(showDerivedReport, (error: unknown) =>
				renderDerivedError(derived, error),
			);
		}
	} catch (error: unknown) {
		renderDerivedError(derived, error);
	}
}

function startTask(task: 'favicon'): void {
	if (liveTaskRuns.some((run) => run.task === task)) return;
	void invoke<number>('start_favicon_collection').then(
		() => scheduleTaskPoll(),
		(error: unknown) => renderDerivedError(derived, error),
	);
}

function selectPage(page: Page): void {
	const selected = pages[page];
	for (const link of pageLinks) {
		if (pageOf(link) === page) link.setAttribute('aria-current', 'page');
		else link.removeAttribute('aria-current');
	}

	pageLabel.setAttribute('aria-label', selected.title);
	pageLabel.dataset.page = page;
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
	void invoke<DerivedReport>('derived_report').then(showDerivedReport, (error: unknown) =>
		renderDerivedError(derived, error),
	);
	void refreshTaskRuns();
	window.addEventListener('focus', () => {
		if (taskPoll === undefined) void refreshTaskRuns();
	});
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
