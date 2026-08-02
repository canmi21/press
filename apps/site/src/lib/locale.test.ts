import { describe, expect, it } from 'vitest';
import {
	LOCALE_CODES,
	acceptedLocale,
	assertLanguageTag,
	languageTag,
	privateHtml,
	resolveLocale,
	withoutLanguageParameter,
} from './locale';

describe('locale resolution', () => {
	it('lets each source win over every source below it', () => {
		expect(resolveLocale({ query: 'fr', cookie: 'de', acceptLanguage: 'ja-JP' })).toBe('fr');
		expect(resolveLocale({ query: null, cookie: 'de', acceptLanguage: 'ja-JP' })).toBe('de');
		expect(resolveLocale({ query: null, cookie: null, acceptLanguage: 'ja-JP' })).toBe('ja');
		expect(resolveLocale({ query: null, cookie: null, acceptLanguage: null })).toBe('mw');
	});

	it('falls through unknown and malformed values instead of rejecting the request', () => {
		expect(resolveLocale({ query: 'klingon', cookie: 'es', acceptLanguage: 'fr-FR' })).toBe('es');
		expect(resolveLocale({ query: '%', cookie: 'unknown', acceptLanguage: 'ko-KR' })).toBe('ko');
	});

	it('honours Accept-Language weights and the two Chinese scripts', () => {
		expect(acceptedLocale('de-DE;q=0.5, zh-Hant-TW;q=0.9, en-US;q=0.8')).toBe('tw');
		expect(acceptedLocale('zh-Hans-CN')).toBe('zh');
		expect(acceptedLocale('zh-TW-u-nu-hanidec')).toBe('tw');
		expect(acceptedLocale('fr-FR;q=broken, en-US;q=0.8')).toBe('en');
		expect(acceptedLocale('fr-FR;q=1.2, de-DE;q=0.7')).toBe('de');
	});
});

it('rejects a malformed source language with the article file named', () => {
	expect(() => assertLanguageTag('zh_CN', 'contents/example.md')).toThrow(
		'contents/example.md: invalid BCP-47 lang frontmatter "zh_CN"',
	);
	expect(() => assertLanguageTag('chinese', 'contents/example.md')).toThrow(
		/contents\/example\.md/,
	);
	expect(() => assertLanguageTag('zh-Hant-TW', 'contents/example.md')).not.toThrow();
});

it('never maps an internal translation code directly into an html lang value', () => {
	for (const code of LOCALE_CODES) {
		const tag = languageTag(code, 'pl-PL');
		expect(tag).not.toBe(code);
		expect(tag).toMatch(/^[a-z]{2,3}(?:-[A-Z]{2})$/);
	}
});

it('marks HTML private without changing an asset response', async () => {
	const html = privateHtml(
		new Response('<!doctype html>', { headers: { 'content-type': 'text/html; charset=utf-8' } }),
	);
	expect(html.headers.get('cache-control')).toBe('private, no-store');

	const asset = new Response('body', {
		headers: { 'content-type': 'text/css', 'cache-control': 'public, max-age=31536000' },
	});
	expect(privateHtml(asset)).toBe(asset);
	expect(asset.headers.get('cache-control')).toBe('public, max-age=31536000');
});

it('removes only lang and keeps unrelated query parameters in their original order', () => {
	const clean = withoutLanguageParameter(
		new URL('/post?utm=a&lang=ja&draft=1&utm=b#section', import.meta.url),
	);
	expect(clean).toBe('/post?utm=a&draft=1&utm=b#section');
});
