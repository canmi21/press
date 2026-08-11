import { followSystemTheme } from '@canmi/theme';
import { getCurrentWindow } from '@tauri-apps/api/window';
import './style.css';

followSystemTheme();

const pages = {
	overview: {
		title: 'Overview',
		description: 'Content status and scheduled work will appear here.',
	},
	articles: {
		title: 'Articles',
		description: 'Article browsing and editing will appear here.',
	},
	media: {
		title: 'Media',
		description: 'Imported resources and their processing state will appear here.',
	},
	automations: {
		title: 'Automations',
		description: 'Scheduled content and resource tasks will appear here.',
	},
	activity: {
		title: 'Activity',
		description: 'Task progress and run history will appear here.',
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
const pageTitle = requiredElement<HTMLElement>('[data-page-title]');
const pageDescription = requiredElement<HTMLElement>('[data-page-description]');

function selectPage(page: Page): void {
	const selected = pages[page];
	for (const link of pageLinks) {
		if (pageOf(link) === page) link.setAttribute('aria-current', 'page');
		else link.removeAttribute('aria-current');
	}

	pageTitle.textContent = selected.title;
	pageDescription.textContent = selected.description;
	document.title = selected.title;
}

for (const link of pageLinks) {
	link.addEventListener('click', () => selectPage(pageOf(link)));
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
