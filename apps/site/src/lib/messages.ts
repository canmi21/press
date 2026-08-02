import type { LocaleCode } from './locale';

type Messages = {
	writing: string;
};

export const MESSAGES = {
	mw: { writing: 'Writing' },
	de: { writing: 'Artikel' },
	en: { writing: 'Writing' },
	es: { writing: 'Artículos' },
	fr: { writing: 'Articles' },
	ja: { writing: '記事' },
	ko: { writing: '글' },
	zh: { writing: '文章' },
	tw: { writing: '文章' },
} as const satisfies Record<LocaleCode, Messages>;
