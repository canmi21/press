import { describe, expect, it } from 'vitest';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import es from '../../../messages/es.json';
import fr from '../../../messages/fr.json';
import ja from '../../../messages/ja.json';
import ko from '../../../messages/ko.json';
import mw from '../../../messages/mw.json';
import tw from '../../../messages/tw.json';
import zh from '../../../messages/zh.json';

const locales = { de, en, es, fr, ja, ko, mw, tw, zh } as const;

describe('support action copy', () => {
	for (const [locale, messages] of Object.entries(locales)) {
		it(`${locale} keeps the resting copy inside every expanded label`, () => {
			expect(messages['support.like']).toContain('{count}');
			expect(messages['support.google']).toContain(messages['support.google-short']);
			expect(messages['support.sponsor']).toContain(messages['support.sponsor-short']);
		});
	}
});
