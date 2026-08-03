import type { LocaleCode } from './locale';

type Messages = {
	writing: string;
	translatorNote: string;
	closeTranslatorNote: string;
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
	mw: {
		writing: 'Writing',
		translatorNote: '来自译者',
		closeTranslatorNote: '关闭译者注',
		original: '原文',
	},
	de: {
		writing: 'Artikel',
		translatorNote: 'Anmerkung des Übersetzers',
		closeTranslatorNote: 'Anmerkung schließen',
		original: 'Original',
	},
	en: {
		writing: 'Writing',
		translatorNote: 'From the translator',
		closeTranslatorNote: "Close translator's note",
		original: 'Original',
	},
	es: {
		writing: 'Artículos',
		translatorNote: 'Nota del traductor',
		closeTranslatorNote: 'Cerrar la nota del traductor',
		original: 'Original',
	},
	fr: {
		writing: 'Articles',
		translatorNote: 'Note du traducteur',
		closeTranslatorNote: 'Fermer la note du traducteur',
		original: 'Original',
	},
	ja: {
		writing: '記事',
		translatorNote: '訳者より',
		closeTranslatorNote: '訳注を閉じる',
		original: '原文',
	},
	ko: {
		writing: '글',
		translatorNote: '번역자 주',
		closeTranslatorNote: '번역자 주 닫기',
		original: '원문',
	},
	zh: {
		writing: '文章',
		translatorNote: '来自译者',
		closeTranslatorNote: '关闭译者注',
		original: '原文',
	},
	tw: {
		writing: '文章',
		translatorNote: '來自譯者',
		closeTranslatorNote: '關閉譯者註',
		original: '原文',
	},
} as const satisfies Record<LocaleCode, Messages>;
