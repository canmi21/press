import { describe, expect, it, vi } from 'vitest';
import {
	contentLanguageHref,
	LANGUAGE_ENDONYMS,
	languageChoices,
	MARK_SIZE,
	markHeightRem,
	orderFor,
	selectContentLanguage,
	sourceCode,
	sourceLabel,
	sourceLanguageName,
	triggerLabel,
} from './switcher';
import * as m from '../paraglide/messages';
import { SITE_LANGUAGE, type LocaleCode } from './index';

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
	it('names the site language on a page, which is what its own tag already says', () => {
		// A page used to read `Original` with nothing in brackets, while the worker was declaring
		// `<html lang="en-US">` over the same document. The switcher now names what the page says
		// it is.
		expect(languageChoices('zh', SITE_LANGUAGE).at(-1)).toMatchObject({
			code: 'mw',
			name: '原文 (英语)',
		});
		expect(languageChoices('de', SITE_LANGUAGE).at(-1)).toMatchObject({
			code: 'mw',
			name: 'Original (US)',
		});
		// Still one row per language, and still last.
		expect(languageChoices('en', SITE_LANGUAGE)).toHaveLength(9);
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
		for (const render of [
			m['notice.translated'],
			m['notice.polished'],
			m['notice.script'],
			m['notice.unavailable'],
		]) {
			// `source` is unused by three of the four; passing it to all of them keeps this loop
			// one loop rather than a special case for the notice that names two languages.
			const inputs = { language: 'X', source: 'Y' };
			const original = render(inputs, { locale: 'mw' });
			for (const locale of ['de', 'en', 'es', 'fr', 'ja', 'ko', 'tw'] as const) {
				const rendered = render(inputs, { locale });
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

describe('the closed switcher', () => {
	it('names the language being read, with the region rather than the internal code', () => {
		// zh is published as zh-CN and tw as zh-TW, so the bracket separates one language's two
		// publications. `?lang=` codes would put `ZH` here, and those are ours alone.
		expect(triggerLabel('zh', 'en')).toBe('简体中文 (CN)');
		expect(triggerLabel('tw', 'en')).toBe('繁體中文 (TW)');
		expect(triggerLabel('en', 'zh')).toBe('English (US)');
		expect(triggerLabel('ja', 'zh')).toBe('日本語 (JP)');
		expect(triggerLabel('ko', 'zh')).toBe('한국어 (KR)');
		expect(triggerLabel('de', 'zh')).toBe('Deutsch (DE)');
		expect(triggerLabel('fr', 'zh')).toBe('Français (FR)');
		expect(triggerLabel('es', 'zh')).toBe('Español (ES)');
	});

	it('drops the region for a caller with no room, and keeps every endonym distinct without it', () => {
		// The article's metadata row asks for this; nothing else does. It is safe to ask for
		// because the region qualifies nothing among the published views -- the eight names below
		// are already eight different strings. See spec/styling.md.
		const labels = (['zh', 'tw', 'en', 'ja', 'ko', 'de', 'fr', 'es'] as const).map((code) =>
			triggerLabel(code, 'zh', { region: false }),
		);
		expect(labels).toEqual([
			'简体中文',
			'繁體中文',
			'English',
			'日本語',
			'한국어',
			'Deutsch',
			'Français',
			'Español',
		]);
		expect(new Set(labels).size).toBe(labels.length);
	});

	it('keeps the region on the original view fallback, where it is the whole identifier', () => {
		// `Original` names no language by itself, so the qualifier is not decoration there and the
		// option does not reach it.
		expect(triggerLabel('mw', 'pl', { region: false })).toContain('(');
		// A source language the site publishes still folds to its endonym and loses the region.
		expect(triggerLabel('mw', 'zh', { region: false })).toBe('简体中文');
	});

	it('folds the script into the Chinese name instead of leaving two brackets', () => {
		// The menu row keeps the endonym as `@canmi/locales` writes it; only the trigger folds it,
		// because only the trigger already ends in a bracket.
		expect(LANGUAGE_ENDONYMS.zh).toBe('中文 (简体)');
		expect(LANGUAGE_ENDONYMS.tw).toBe('中文 (繁體)');
		expect(triggerLabel('zh', SITE_LANGUAGE)).not.toContain('(简体)');
		expect(triggerLabel('tw', SITE_LANGUAGE)).not.toContain('(繁體)');
	});

	it('names the article language on the original view, where the menu names the state', () => {
		// The two say different things on purpose: the trigger answers what is being read, the row
		// inside the menu answers which view it is.
		expect(triggerLabel('mw', 'zh')).toBe('简体中文 (CN)');
		expect(triggerLabel('mw', 'zh-Hant')).toBe('繁體中文 (TW)');
		expect(triggerLabel('mw', 'ja')).toBe('日本語 (JP)');

		const rows = languageChoices('mw', 'zh');
		expect(rows.at(-1)?.name).toBe(`${m['language.original']({}, { locale: 'mw' })} (CN)`);
	});

	it('keeps Original where there is no language of ours to name', () => {
		// A language this site publishes no view of has no endonym to show and no region that
		// would mean anything. A page is not that case: it has the site's own language.
		expect(triggerLabel('mw', 'it')).toBe(`${m['language.original']({}, { locale: 'mw' })} (IT)`);
		expect(triggerLabel('mw', SITE_LANGUAGE)).toBe('English (US)');
	});
});

describe('the marks the menu is scanned by', () => {
	it('gives every mark the height its own ink asks for', () => {
		// The classes are literals because Tailwind reads source text, so nothing in the build
		// would notice one drifting from the measurement it stands for. This does.
		for (const [mark, size] of Object.entries(MARK_SIZE)) {
			const written = /^h-\[([\d.]+)rem\] w-auto$/.exec(size)?.[1];
			expect(written, `${mark} is not a height this test can read`).toBeDefined();
			expect(Number(written)).toBeCloseTo(markHeightRem(mark as keyof typeof MARK_SIZE), 3);
		}
	});

	it('draws one glyph at one size, whatever is decorating it', () => {
		// `translate-2-line` and `translate-2-ai-line` are the same drawing plus a sparkle. Sized
		// on their own ink the plain one comes out larger, and the letterform they share renders
		// at two sizes on neighbouring rows.
		expect(MARK_SIZE['translate-simplified']).toBe(MARK_SIZE['translate-ai']);
	});

	it('leaves no mark at the shared height the set used to carry', () => {
		// `h-4` was one class for four glyphs that do not fill their viewBox alike, which is the
		// thing being corrected. A mark back at exactly 1rem would mean the table was bypassed.
		for (const size of Object.values(MARK_SIZE)) expect(size).not.toBe('h-4 w-auto');
	});
});
