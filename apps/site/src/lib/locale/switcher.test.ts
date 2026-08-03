import { describe, expect, it, vi } from 'vitest';
import {
	LANGUAGE_ENDONYMS,
	languageChoices,
	orderFor,
	selectContentLanguage,
	sourceLabel,
} from './switcher';
import type { LocaleCode } from './index';

function stableEndonyms(current: LocaleCode) {
	return languageChoices(current, 'zh')
		.filter((choice) => !choice.original)
		.map(({ code, name }) => ({ code, name }));
}

describe('article language switcher', () => {
	it('closes on the active code without navigating', () => {
		const navigate = vi.fn();

		expect(
			selectContentLanguage('ja', 'ja', new URL('/post?draft=1', import.meta.url), navigate),
		).toBe(false);
		expect(navigate).not.toHaveBeenCalled();
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

	it('names the source language briefly, and follows the article rather than assuming Chinese', () => {
		// A CJK view can spell it out; a Latin one would crowd the row, so it gets the subtag.
		expect(sourceLabel('zh', 'zh')).toBe('中文');
		expect(sourceLabel('zh', 'ja')).toBe('中国語');
		expect(sourceLabel('zh', 'en')).toBe('CN');

		// The original is not always Chinese. An English-authored article says so.
		expect(sourceLabel('en', 'zh')).toBe('英语');
		expect(sourceLabel('en', 'fr')).toBe('US');

		// `mw` reads 原文, so it belongs with the compact scripts whatever the article is in.
		expect(sourceLabel('en', 'mw')).toBe('English');
		expect(sourceLabel('zh', 'mw')).toBe('中文');

		// `mw` reads 原文, so it belongs with the compact scripts whatever the article is in.
		expect(sourceLabel('en', 'mw')).toBe('English');
		expect(sourceLabel('zh', 'mw')).toBe('中文');

		// Traditional Chinese is a different code, which is the distinction CN and TW carry.
		expect(sourceLabel('zh-Hant', 'en')).toBe('TW');
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
		const navigate = vi.fn();
		expect(selectContentLanguage('mw', 'zh', new URL('/post', import.meta.url), navigate)).toBe(
			true,
		);
		expect(navigate).toHaveBeenLastCalledWith('/post?lang=zh');
		expect(selectContentLanguage('zh', 'mw', new URL('/post', import.meta.url), navigate)).toBe(
			true,
		);
		expect(navigate).toHaveBeenLastCalledWith('/post?lang=mw');
	});

	it('keeps the translated endonym list identical in every view', () => {
		const expected = stableEndonyms('mw');
		for (const current of ['de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const) {
			expect(stableEndonyms(current)).toEqual(expected);
		}
	});
});
