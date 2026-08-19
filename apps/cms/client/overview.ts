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

function renderRecent(root: HTMLElement, snapshot: OverviewSnapshot): void {
	const section = requiredElement<HTMLElement>(root, '[data-recent]');
	const list = requiredElement<HTMLUListElement>(root, '[data-recent-list]');
	list.replaceChildren();
	section.hidden = snapshot.articles.recent.length === 0;

	for (const article of snapshot.articles.recent) {
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
		list.appendChild(item);
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
