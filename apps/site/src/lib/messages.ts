import type { LocaleCode } from './locale';

type Messages = {
	writing: string;
	/**
	 * The article as written, named rather than described.
	 *
	 * Not the source language's own name. A Chinese original and its Simplified Chinese
	 * translation are both Chinese, and the translation is the one that has been regularised --
	 * a deliberate misspelling corrected, an unconventional mark normalised. Labelling the
	 * original by its language would put two identical names in the list and hide the only
	 * distinction that matters, which is that one of them is untouched.
	 */
	original: string;
};

export const MESSAGES = {
	mw: { writing: 'Writing', original: '原文' },
	de: { writing: 'Artikel', original: 'Original' },
	en: { writing: 'Writing', original: 'Original' },
	es: { writing: 'Artículos', original: 'Original' },
	fr: { writing: 'Articles', original: 'Original' },
	ja: { writing: '記事', original: '原文' },
	ko: { writing: '글', original: '원문' },
	zh: { writing: '文章', original: '原文' },
	tw: { writing: '文章', original: '原文' },
} as const satisfies Record<LocaleCode, Messages>;
