import { describe, expect, it, vi } from 'vitest';
import { LANGUAGE_ENDONYMS, languageChoices, selectContentLanguage } from './language-switcher';
import type { LocaleCode } from './locale';

function stableEndonyms(current: LocaleCode) {
	return languageChoices(current)
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
		const choices = languageChoices('zh');
		const original = choices.find((choice) => choice.code === 'mw');
		const translated = choices.find((choice) => choice.code === 'zh');

		expect(original).toMatchObject({ name: '原文', original: true, current: false });
		expect(translated).toMatchObject({ name: '中文 (简体)', original: false, current: true });
		expect(original?.name).not.toBe(translated?.name);
	});

	it('says original in whichever language is being read', () => {
		expect(languageChoices('ja')[0]).toMatchObject({ code: 'mw', name: '原文' });
		expect(languageChoices('ko')[0]).toMatchObject({ code: 'mw', name: '원문' });
		expect(languageChoices('de')[0]).toMatchObject({ code: 'mw', name: 'Original' });
	});

	it('lists the original first, and never among the eight', () => {
		for (const current of ['mw', 'de', 'en', 'es', 'fr', 'ja', 'ko', 'zh', 'tw'] as const) {
			const choices = languageChoices(current);
			expect(choices[0]?.code).toBe('mw');
			expect(choices.filter((choice) => choice.original)).toHaveLength(1);
			expect(Object.values(LANGUAGE_ENDONYMS)).not.toContain(choices[0]?.name);
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
