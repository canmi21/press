import type { PageBlock, Page } from '../content/types';
import type { LocaleCode } from '../locale/index';
import * as m from '../paraglide/messages';

export type HomepageContent = {
	title: string;
	description: string;
	bio: PageBlock[];
	writing: string;
};

export function homepageContent(page: Page, code: LocaleCode): HomepageContent {
	const view = page.views[code];
	return {
		title: view.meta.title ?? 'Canmi',
		description: view.meta.description ?? '',
		// The bio is identity copy, not an article view. It deliberately remains English.
		bio: page.views.mw.blocks,
		writing: m['nav.writing']({}, { locale: code }),
	};
}
