import { ARTICLE_THUMBNAIL_LINES } from '@canmi/primitives';

const articleDate = new Intl.DateTimeFormat('en-US', {
	month: 'short',
	day: 'numeric',
	year: 'numeric',
	timeZone: 'UTC',
});

export function formatArticleDate(value: string): string {
	const date = new Date(value);
	return Number.isNaN(date.getTime()) ? value : articleDate.format(date);
}

export function articleThumbnail(): HTMLElement {
	const thumbnail = document.createElement('div');
	thumbnail.className = 'article-preview-thumbnail';
	thumbnail.dataset.articleIcon = '';
	thumbnail.setAttribute('aria-hidden', 'true');
	for (const line of ARTICLE_THUMBNAIL_LINES) {
		const bar = document.createElement('span');
		bar.dataset.iconBar = '';
		bar.style.width = line.width;
		bar.style.marginTop = line.marginTop;
		thumbnail.appendChild(bar);
	}
	return thumbnail;
}
