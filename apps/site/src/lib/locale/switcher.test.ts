import { describe, expect, it, vi } from 'vitest';
import {
	contentLanguageHref,
	LANGUAGE_ENDONYMS,
	languageChoices,
	orderFor,
	selectContentLanguage,
	sourceCode,
	sourceLabel,
	sourceLanguageName,
} from './switcher';
import * as m from '../paraglide/messages';
import type { LocaleCode } from './index';

function stableEndonyms(current: LocaleCode) {
	return languageChoices(current, 'zh')
		.filter((choice) => !choice.original)
		.map(({ code, name }) => ({ code, name }));
}

describe('article language switcher', () => {
	it('closes on the active code without navigating', () => {
		const select = vi.fn();

		expect(selectContentLanguage('ja', 'ja', select)).toBe(false);
		expect(select).not.toHaveBeenCalled();
	});

	it('names the original for its state, never for its language', () => {
		// Both are Chinese, and labelling the original by its language put the same string in the
		// list twice with nothing to choose between them. The one distinction worth showing is
		// that the translation has been regularised and the original has not.
		const choices = languageChoices('zh', 'zh');
		const original = choices.find((choice) => choice.code === 'mw');
		const translated = choices.find((choice) => choice.code === 'zh');

		expect(original).toMatchObject({ name: '原文 (中文)', original: true, current: false });
		expect(translated).toMatchObject({ name: '中文 (简体)', original: false, current: true });
		expect(original?.name).not.toBe(translated?.name);
	});

	it('says original in whichever language is being read', () => {
		expect(languageChoices('ja', 'zh').at(-1)).toMatchObject({ code: 'mw', name: '原文 (中国語)' });
		expect(languageChoices('ko', 'zh').at(-1)).toMatchObject({ code: 'mw', name: '원문 (중국어)' });
		expect(languageChoices('de', 'zh').at(-1)).toMatchObject({ code: 'mw', name: 'Original (CN)' });
	});

	// The switcher also sits on pages, which have no article and so no language for the
	// qualifier to name. The row stays -- the preference it writes is site-wide, and choosing
	// the original is a different answer from choosing English once an article is opened -- but
	// the brackets go rather than being filled from somewhere.
	it('drops the qualifier when there is no article to name', () => {
		expect(languageChoices('zh', undefined).at(-1)).toMatchObject({ code: 'mw', name: '原文' });
		expect(languageChoices('de', undefined).at(-1)).toMatchObject({
			code: 'mw',
			name: 'Original',
		});
		// Still one row per language, and still last.
		expect(languageChoices('en', undefined)).toHaveLength(9);
	});

	it('names the source language briefly, and follows the article rather than assuming Chinese', () => {
		// A CJK view can spell it out; a Latin one would crowd the row, so it gets the subtag.
		expect(sourceLabel('zh', 'zh')).toBe('中文');
		expect(sourceLabel('zh', 'ja')).toBe('中国語');
		expect(sourceLabel('zh', 'en')).toBe('CN');

		// The original is not always Chinese. An English-authored article says so.
		expect(sourceLabel('en', 'zh')).toBe('英语');
		expect(sourceLabel('en', 'fr')).toBe('US');

		// `mw` labels itself `Original`, so its parenthetical is a region code like the other
		// Latin-script views rather than a spelled-out name in the article's own language.
		expect(sourceLabel('en', 'mw')).toBe('US');
		expect(sourceLabel('zh', 'mw')).toBe('CN');

		// Traditional Chinese is a different code, which is the distinction CN and TW carry.
		expect(sourceLabel('zh-Hant', 'en')).toBe('TW');
	});

	it('spells out the source language in the current interface language, script and all', () => {
		// "中文" covers both scripts and so names neither. The tag carries the script into
		// DisplayNames, which already spells the split out in every interface language.
		expect(sourceLanguageName('zh-CN', 'en')).toBe('Simplified Chinese');
		expect(sourceLanguageName('zh', 'zh')).toBe('简体中文');
		expect(sourceLanguageName('zh', 'ja')).toBe('簡体中国語');
		expect(sourceLanguageName('zh-Hant', 'en')).toBe('Traditional Chinese');
		expect(sourceLanguageName('zh-TW', 'zh')).toBe('繁体中文');

		// Only Chinese splits by script here; everything else keeps its plain name.
		expect(sourceLanguageName('en-US', 'zh')).toBe('英语');
	});

	it('resolves which view an article was written for, or none', () => {
		expect(sourceCode('zh')).toBe('zh');
		expect(sourceCode('zh-CN')).toBe('zh');
		expect(sourceCode('zh-Hant')).toBe('tw');
		expect(sourceCode('zh-HK')).toBe('tw');
		expect(sourceCode('en-US')).toBe('en');

		// The eight are what may be read, not a promise about what may be written.
		expect(sourceCode('it-IT')).toBeUndefined();
	});

	it('states the source language as the main one, not the only one', () => {
		// Frontmatter `lang` picks an article's primary language; a mixed-language original still
		// gets one tag. The copy has to be true of an article that is mostly, not wholly, in it.
		const qualifier: Record<string, string> = {
			zh: '主要',
			tw: '主要',
			en: 'mainly',
			ja: '主に',
			ko: '주로',
			es: 'principalmente',
			fr: 'principalement',
			de: 'überwiegend',
		};
		for (const [locale, word] of Object.entries(qualifier)) {
			expect(m['notice.polished']({ language: 'X' }, { locale: locale as LocaleCode })).toContain(
				word,
			);
		}
	});

	it('gives every locale its own sentence rather than the baseLocale falling in', () => {
		// A missing key resolves to `mw` and renders, so a gap never announces itself -- the
		// reader simply gets the original's wording where their own language should have been.
		// Requiring each locale to differ from what it would fall back to is what catches that.
		// `zh` is exempt: `mw` is written in it, which is the whole point of `mw`.
		for (const render of [m['notice.translated'], m['notice.polished'], m['notice.script']]) {
			const original = render({ language: 'X' }, { locale: 'mw' });
			for (const locale of ['de', 'en', 'es', 'fr', 'ja', 'ko', 'tw'] as const) {
				const rendered = render({ language: 'X' }, { locale });
				expect(rendered).toContain('X');
				expect(rendered).not.toBe(original);
			}
		}
	});

	it('carries the script sentence in the two views that can reach it', () => {
		// Only the Chinese pair can be the sibling script of an article they already read.
		expect(m['notice.script']({ language: 'X' }, { locale: 'zh' })).toContain('简体版本');
		expect(m['notice.script']({ language: 'X' }, { locale: 'tw' })).toContain('繁體版本');
	});

	it('separates a script conversion from a translation', () => {
		// The state that decides which sentence is shown. A Simplified article read in Traditional
		// is neither the original nor a translation, and calling it one overstates the distance.
		expect(sourceCode('zh')).not.toBe('tw');
		expect(sourceCode('zh-Hant')).not.toBe('zh');

		// Both directions resolve, so neither Chinese view falls through to the translated copy.
		expect(sourceCode('zh')).toBe('zh');
		expect(sourceCode('zh-TW')).toBe('tw');
	});

	it('lists the original last, and never among the eight', () => {
		for (const current of ['mw', 'de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const) {
			const choices = languageChoices(current, 'zh');
			expect(choices.at(-1)?.code).toBe('mw');
			expect(choices.filter((choice) => choice.original)).toHaveLength(1);
			expect(Object.values(LANGUAGE_ENDONYMS)).not.toContain(choices.at(-1)?.name);
		}
	});

	it('orders by the reader rather than by the view', () => {
		// A reader in Japanese should not walk past four European languages to reach Chinese, and
		// a reader in French should not do the reverse.
		expect(languageChoices('en', 'zh', 'ja').map((choice) => choice.code)).toEqual([
			'en',
			'zh',
			'tw',
			'ja',
			'ko',
			'de',
			'fr',
			'es',
			'mw',
		]);
		expect(languageChoices('en', 'zh', 'fr').map((choice) => choice.code)).toEqual([
			'en',
			'es',
			'fr',
			'ja',
			'zh',
			'tw',
			'ko',
			'de',
			'mw',
		]);
	});

	it('holds the same sequence across every view a reader moves through', () => {
		// The order follows the reader, so switching translations must not reshuffle the menu.
		for (const preferred of ['ja', 'fr'] as const) {
			const expected = languageChoices('mw', 'zh', preferred).map((choice) => choice.code);
			for (const current of ['de', 'en', 'ja', 'zh', 'tw'] as const) {
				expect(languageChoices(current, 'zh', preferred).map((choice) => choice.code)).toEqual(
					expected,
				);
			}
		}
	});

	it('keeps both orders complete, so neither can lose a language', () => {
		const codes = Object.keys(LANGUAGE_ENDONYMS).toSorted();
		for (const preferred of ['zh', 'tw', 'ja', 'ko', 'en', 'de', 'fr', 'es'] as const) {
			expect(orderFor(preferred).toSorted()).toEqual(codes);
		}
	});

	it('reaches both Chinese views from either of them', () => {
		const select = vi.fn();
		expect(selectContentLanguage('mw', 'zh', select)).toBe(true);
		expect(select).toHaveBeenLastCalledWith('zh');
		expect(selectContentLanguage('zh', 'mw', select)).toBe(true);
		expect(select).toHaveBeenLastCalledWith('mw');
	});

	it('keeps unrelated URL state when linking to the original', () => {
		expect(
			contentLanguageHref('mw', new URL('/post?draft=1&lang=ja#details', import.meta.url)),
		).toBe('/post?draft=1&lang=mw#details');
	});

	it('keeps the translated endonym list identical in every view', () => {
		const expected = stableEndonyms('mw');
		for (const current of ['de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const) {
			expect(stableEndonyms(current)).toEqual(expected);
		}
	});
});
