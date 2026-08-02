import { describe, expect, it } from 'vitest';
import { feedLocale } from './feed';

function request(url: string): Request {
	return new Request(url, {
		headers: {
			'Accept-Language': 'de-DE',
			Cookie: 'language=fr',
		},
	});
}

describe('feed locale', () => {
	it('ignores cookies and Accept-Language and answers only to the query parameter', () => {
		expect(feedLocale(request('https://example.com/atom.xml'))).toBe('mw');
		expect(feedLocale(request('https://example.com/atom.xml?lang=ja'))).toBe('ja');
		expect(feedLocale(request('https://example.com/atom.xml?lang=unknown'))).toBe('mw');
	});
});
