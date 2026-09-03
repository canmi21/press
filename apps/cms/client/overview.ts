import { articleThumbnail, formatArticleDate } from './article-preview';
import { requiredElement } from './dom';

export type OverviewSnapshot = {
	articles: {
		total: number;
		sections: Array<{
			name: string;
			articles: number;
		}>;
		recent: Array<{
			title: string;
			subtitle: string | null;
			modified: string;
		}>;
	};
	media: {
		referenced: number;
		published: number;
		described: number;
	};
	health: {
		warnings: number;
		notices: number;
		gaps: Array<{
			level: 'warn' | 'info';
			subject: string;
			detail: string;
			action?: 'image' | 'alt' | 'favicon';
		}>;
	};
};

function countLabel(count: number, singular: string, plural = `${singular}s`): string {
	return `${count} ${count === 1 ? singular : plural}`;
}

function contentSummary(snapshot: OverviewSnapshot): string {
	const articles = snapshot.articles.total;
	const sections = snapshot.articles.sections.length;
	if (articles === 0) return 'No articles yet.';
	if (sections === 1) return `${countLabel(articles, 'article')} in one section.`;
	return `${countLabel(articles, 'article')} across ${countLabel(sections, 'section')}.`;
}

function mediaSummary(snapshot: OverviewSnapshot): string {
	const { referenced, published, described } = snapshot.media;
	if (referenced === 0) return 'No referenced media.';
	if (published === referenced && described === referenced) {
		return referenced === 1
			? 'The referenced media item is published and described.'
			: `All ${referenced} referenced media items are published and described.`;
	}
	return `${published} of ${referenced} media items published; ${described} described.`;
}

function issueSummary(warnings: number, notices: number): string {
	const parts = [
		warnings > 0 ? countLabel(warnings, 'warning') : undefined,
		notices > 0 ? countLabel(notices, 'notice') : undefined,
	].filter((part): part is string => part !== undefined);
	return parts.length === 2 ? `${parts[0]} and ${parts[1]}` : (parts[0] ?? 'no open checks');
}

function renderHealth(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const title = requiredElement<HTMLElement>(root, '[data-overview-title]');
	const lede = requiredElement<HTMLElement>(root, '[data-overview-lede]');
	const icon = requiredElement<HTMLElement>(root, '[data-overview-status-icon]');
	const attention = requiredElement<HTMLElement>(root, '[data-attention]');
	const healthList = requiredElement<HTMLUListElement>(root, '[data-health-list]');
	const issueCount = snapshot.health.warnings + snapshot.health.notices;
	healthList.replaceChildren();
	delete lede.dataset.state;

	if (snapshot.health.gaps.length === 0) {
		icon.dataset.state = 'ready';
		title.textContent = 'Everything is ready.';
		lede.textContent = 'There are no open workspace checks.';
		attention.hidden = true;
		return;
	}

	icon.dataset.state = snapshot.health.warnings > 0 ? 'warning' : 'notice';
	title.textContent = `${countLabel(issueCount, 'thing')} need${issueCount === 1 ? 's' : ''} attention.`;
	lede.textContent = `Workspace checks found ${issueSummary(
		snapshot.health.warnings,
		snapshot.health.notices,
	)}.`;
	attention.hidden = false;

	for (const gap of snapshot.health.gaps) {
		const item = document.createElement('li');
		item.dataset.level = gap.level;
		const marker = document.createElement('span');
		marker.className = 'health-marker';
		marker.setAttribute('aria-label', gap.level === 'warn' ? 'Warning' : 'Notice');
		const copy = document.createElement('div');
		const subject = document.createElement('strong');
		subject.textContent = gap.subject;
		const detail = document.createElement('span');
		detail.textContent = gap.detail;
		copy.appendChild(subject);
		copy.appendChild(detail);
		if (gap.action !== undefined) {
			const remedy = document.createElement('code');
			remedy.className = 'health-action';
			remedy.textContent = `cms ${gap.action}`;
			copy.appendChild(remedy);
		}
		item.appendChild(marker);
		item.appendChild(copy);
		healthList.appendChild(item);
	}
}

function recentRow(article: OverviewSnapshot['articles']['recent'][number]): HTMLLIElement {
	const item = document.createElement('li');
	item.className = 'article-preview';
	const copy = document.createElement('div');
	copy.className = 'article-preview-copy';
	const heading = document.createElement('div');
	heading.className = 'article-preview-heading';
	const title = document.createElement('strong');
	title.className = 'article-preview-title';
	title.textContent = article.title;
	const leader = document.createElement('span');
	leader.className = 'article-preview-leader';
	const modified = document.createElement('time');
	modified.className = 'article-preview-date';
	modified.dateTime = article.modified;
	modified.textContent = formatArticleDate(article.modified);
	heading.appendChild(title);
	heading.appendChild(leader);
	heading.appendChild(modified);
	copy.appendChild(heading);
	if (article.subtitle !== null) {
		const subtitle = document.createElement('p');
		subtitle.className = 'article-preview-subtitle';
		subtitle.textContent = article.subtitle;
		copy.appendChild(subtitle);
	}
	item.appendChild(articleThumbnail());
	item.appendChild(copy);
	return item;
}

// What the page last rendered, kept so the list can be re-fitted to a window that changed size
// without asking the workspace for a second snapshot -- the answer would be the same, and reading
// it costs a walk of every article.
let shown: { root: HTMLElement; snapshot: OverviewSnapshot } | undefined;
let watchingViewport = false;

/**
 * Fills the recent list with as many articles as the window has room for.
 *
 * The count is measured rather than chosen: a fixed number is either a short list on a tall
 * window or a scrollbar on a short one, and which of the two it is depends on the machine. So
 * every article the snapshot carries is laid out and the ones that overflow are taken back,
 * leaving the page exactly full. The exit to the library is placed before the trimming rather
 * than after it, because it occupies the room the last row would otherwise be measured into.
 *
 * One article always stays. A window too short for even that is a window nothing can be fitted
 * to, and an empty section under a heading reads as "no articles" rather than as "no room" -- so
 * the page scrolls, which is the honest failure of the two.
 *
 * A hidden page has no geometry to read, so this stands aside for one -- `main.ts` calls it again
 * when the Overview comes back.
 */
export function fitRecent(): void {
	if (shown === undefined) return;
	const { root, snapshot } = shown;
	const scroller = root.closest<HTMLElement>('.page-content');
	if (scroller === null || root.hidden || scroller.clientHeight === 0) return;

	const list = requiredElement<HTMLUListElement>(root, '[data-recent-list]');
	const all = requiredElement<HTMLButtonElement>(root, '[data-recent-all]');
	list.replaceChildren(...snapshot.articles.recent.map(recentRow));

	const overflows = (): boolean => scroller.scrollHeight > scroller.clientHeight;
	all.hidden = !overflows() && snapshot.articles.recent.length >= snapshot.articles.total;
	while (list.childElementCount > 1 && overflows()) list.lastElementChild?.remove();
	all.hidden = list.childElementCount >= snapshot.articles.total;
}

function renderRecent(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const section = requiredElement<HTMLElement>(root, '[data-recent]');
	section.hidden = snapshot.articles.recent.length === 0;

	// The snapshot carries the workspace total beside the articles it sends, so the exit names the
	// number rather than promising "more".
	requiredElement<HTMLElement>(root, '[data-recent-all-label]').textContent =
		`View all ${countLabel(snapshot.articles.total, 'article')}`;

	shown = { root, snapshot };
	fitRecent();

	if (!watchingViewport) {
		const scroller = root.closest<HTMLElement>('.page-content');
		if (scroller !== null) {
			watchingViewport = true;
			new ResizeObserver(() => fitRecent()).observe(scroller);
		}
	}
}

export function renderOverview(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const meta = requiredElement<HTMLElement>(root, '.overview-meta');
	meta.hidden = false;
	requiredElement<HTMLElement>(root, '[data-overview-content]').textContent =
		contentSummary(snapshot);
	requiredElement<HTMLElement>(root, '[data-overview-media]').textContent = mediaSummary(snapshot);
	renderHealth(root, snapshot);
	renderRecent(root, snapshot);
}

export function renderOverviewError(root: HTMLElement, error: unknown): void {
	requiredElement<HTMLElement>(root, '[data-overview-status-icon]').dataset.state = 'error';
	requiredElement<HTMLElement>(root, '[data-overview-title]').textContent =
		'Workspace unavailable.';
	const lede = requiredElement<HTMLElement>(root, '[data-overview-lede]');
	lede.textContent = error instanceof Error ? error.message : String(error);
	lede.dataset.state = 'error';
	requiredElement<HTMLElement>(root, '.overview-meta').hidden = true;
	requiredElement<HTMLElement>(root, '[data-attention]').hidden = true;
	requiredElement<HTMLElement>(root, '[data-recent]').hidden = true;
}
