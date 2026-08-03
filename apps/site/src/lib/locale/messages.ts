import type { LocaleCode } from './index';

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

type TranslationCode = Exclude<LocaleCode, 'mw'>;

/** Copy wrapped around a linked source-language name. */
type TranslatedCopy = {
	beforeLanguage: string;
	afterLanguage: string;
};

/**
 * Copy for the case where the article is already in the language being read.
 *
 * Four slots rather than two, because the sentence names the language *and* points somewhere,
 * and those are not the same word here. The language name is stated plainly and the link goes
 * on the word for the untouched article, which is what the reader is being sent to.
 */
export type PolishedCopy = {
	beforeLanguage: string;
	beforeLink: string;
	/**
	 * The word for the untouched article as it reads inside a sentence.
	 *
	 * Deliberately not `MESSAGES.original`, which is the same word as a menu row label. English
	 * capitalises a label but not a mid-sentence noun, and German capitalises both; one string
	 * cannot be right in both positions, and collapsing them would fix one language by breaking
	 * another. See spec/locale.md.
	 */
	linkLabel: string;
	afterLink: string;
};

/**
 * What a non-original view says about itself, by why it differs from the article.
 *
 * Two rows, because "translated" and "same language, lightly regularised" are different claims
 * and a reader who speaks the article's own language is owed the second one rather than being
 * told they are reading a translation of their own language.
 */
type TranslationNotice = {
	translated: TranslatedCopy;
	polished: PolishedCopy;
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

export const TRANSLATION_NOTICE = {
	de: {
		translated: {
			beforeLanguage: 'Du liest eine Übersetzung. Das Original ist auf ',
			afterLanguage: '.',
		},
		polished: {
			beforeLanguage: 'Dieser Artikel ist überwiegend auf ',
			beforeLink:
				' geschrieben. Die Fassung, die du liest, kann leichte sprachliche Glättungen enthalten; empfohlen wird das ',
			linkLabel: 'Original',
			afterLink: '.',
		},
	},
	en: {
		translated: {
			beforeLanguage: "You're reading a translation. The original is in ",
			afterLanguage: '.',
		},
		polished: {
			beforeLanguage: 'This article is written mainly in ',
			beforeLink: '. The version you are reading may carry small changes in wording; the ',
			linkLabel: 'original',
			afterLink: ' reads as it was published.',
		},
	},
	es: {
		translated: {
			beforeLanguage: 'Estás leyendo una traducción. El original está en ',
			afterLanguage: '.',
		},
		polished: {
			beforeLanguage: 'Este artículo está escrito principalmente en ',
			beforeLink:
				'. La versión que estás leyendo puede incluir ligeros retoques de redacción; te recomendamos el ',
			linkLabel: 'original',
			afterLink: '.',
		},
	},
	fr: {
		translated: {
			beforeLanguage: 'Vous lisez une traduction. L’original est en ',
			afterLanguage: '.',
		},
		polished: {
			beforeLanguage: 'Cet article est écrit principalement en ',
			beforeLink:
				'. La version que vous lisez peut comporter de légères retouches de formulation ; nous vous recommandons l’',
			linkLabel: 'original',
			afterLink: '.',
		},
	},
	ja: {
		translated: {
			beforeLanguage: '翻訳版を読んでいます。原文は',
			afterLanguage: 'です。',
		},
		polished: {
			beforeLanguage: 'この記事は主に',
			beforeLink: 'で書かれています。読んでいる版には細かな表現の調整が入っている場合があります。',
			linkLabel: '原文',
			afterLink: 'をおすすめします。',
		},
	},
	ko: {
		translated: {
			beforeLanguage: '번역본을 읽고 있습니다. 원문은 ',
			afterLanguage: '입니다.',
		},
		polished: {
			beforeLanguage: '이 글은 주로 ',
			beforeLink: '로 작성되었습니다. 지금 읽고 있는 판은 표현이 조금 다듬어졌을 수 있습니다. ',
			linkLabel: '원문',
			afterLink: '을 권합니다.',
		},
	},
	zh: {
		translated: {
			beforeLanguage: '你正在阅读翻译版本，原文是',
			afterLanguage: '。',
		},
		polished: {
			beforeLanguage: '这篇文章主要以',
			beforeLink: '写成。你正在阅读的译本可能带有细微的措辞润色，推荐阅读',
			linkLabel: '原文',
			afterLink: '。',
		},
	},
	tw: {
		translated: {
			beforeLanguage: '你正在閱讀翻譯版本，原文是',
			afterLanguage: '。',
		},
		polished: {
			beforeLanguage: '這篇文章主要以',
			beforeLink: '寫成。你正在閱讀的譯本可能帶有細微的措辭潤飾，推薦閱讀',
			linkLabel: '原文',
			afterLink: '。',
		},
	},
} as const satisfies Record<TranslationCode, TranslationNotice>;

/** The one language here published under two scripts, and so the only pair this can happen to. */
type ScriptCode = 'zh' | 'tw';

/**
 * The article is in the reader's language, in the other script.
 *
 * Its own state rather than a case of `polished`, because the gap is not editing but conversion:
 * nothing was rewritten, the characters were mapped. That is a smaller claim than translation
 * and a different one from polishing, and it comes with a reason to move that neither of the
 * others has -- the reader can already read the original exactly as written, so the only thing
 * between them and the author is a script they also read.
 *
 * Keyed by the two Chinese views alone, and not folded into the matrix above as an optional
 * row. This state is reachable only from the view that *is* the sibling script, so six of those
 * eight rows could never be shown; a table that can only be two-thirds filled is the wrong shape
 * for the fact. See spec/locale.md.
 */
export const SCRIPT_NOTICE = {
	zh: {
		beforeLanguage: '这篇文章主要以',
		beforeLink: '写成。你正在阅读的简体版本经过字词转换，',
		linkLabel: '原文',
		afterLink: '更接近作者本来的表达。',
	},
	tw: {
		beforeLanguage: '這篇文章主要以',
		beforeLink: '寫成。你正在閱讀的繁體版本經過字詞轉換，',
		linkLabel: '原文',
		afterLink: '更接近作者本來的表達。',
	},
} as const satisfies Record<ScriptCode, PolishedCopy>;
