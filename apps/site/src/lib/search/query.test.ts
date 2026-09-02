import { describe, expect, it } from 'vitest';
import {
	MARK_CLOSE,
	MARK_OPEN,
	groupHits,
	markup,
	worthSearching,
	type SearchHit,
} from './query.ts';

describe('worthSearching', () => {
	it('holds Latin to two characters', () => {
		expect(worthSearching('a')).toBe(false);
		expect(worthSearching('ru')).toBe(true);
	});

	it('accepts a single Han character, which is a word', () => {
		expect(worthSearching('渲')).toBe(true);
		expect(worthSearching('码')).toBe(true);
		// Traditional and Japanese kanji are the same script and get the same answer.
		expect(worthSearching('渲')).toBe(true);
		expect(worthSearching('漢')).toBe(true);
	});

	it('does not extend that to kana or hangul, where one character is a particle', () => {
		expect(worthSearching('の')).toBe(false);
		expect(worthSearching('ア')).toBe(false);
		expect(worthSearching('이')).toBe(false);
		expect(worthSearching('レン')).toBe(true);
	});

	it('counts code points, so a character outside the basic plane is one character', () => {
		// U+20000, a CJK extension B ideograph: two UTF-16 units, one character, one Han script.
		expect(worthSearching('\u{20000}')).toBe(true);
		// An emoji is two units and one character, and is not Han.
		expect(worthSearching('\u{1F600}')).toBe(false);
	});

	it('rejects nothing at all', () => {
		expect(worthSearching('')).toBe(false);
	});
});

describe('markup', () => {
	it('turns the sentinels into marks', () => {
		expect(markup(`${MARK_OPEN}Cargo${MARK_CLOSE} tuning`, '')).toBe('<mark>Cargo</mark> tuning');
	});

	it('escapes the prose around them', () => {
		// The corpus is about web development, so its text really does contain tags.
		expect(markup(`render ${MARK_OPEN}<title>${MARK_CLOSE} in a component`, '')).toBe(
			'render <mark>&lt;title&gt;</mark> in a component',
		);
	});

	it('escapes a value carrying no marks at all', () => {
		expect(markup('<script>alert(1)</script>', '')).toBe('&lt;script&gt;alert(1)&lt;/script&gt;');
	});

	it('falls back when the service returned no highlight for the field', () => {
		expect(markup(undefined, '<b>plain</b>')).toBe('&lt;b&gt;plain&lt;/b&gt;');
	});

	it('leaves an unpaired sentinel unable to open a tag', () => {
		expect(markup(`${MARK_OPEN}half`, '')).toBe('<mark>half');
	});
});

describe('groupHits', () => {
	const hit = (path: string, heading: string): SearchHit => ({
		objectID: `${path}:mw:${heading}`,
		path,
		locale: 'mw',
		url: `https://example.test/${path}#${heading}`,
		title: path === 'a' ? 'Cargo' : 'SeamJS',
		subtitle: '',
		heading,
		text: '',
	});

	it('says one title once and keeps its sections under it', () => {
		const groups = groupHits([hit('a', 'one'), hit('a', 'two'), hit('b', 'three')]);
		expect(groups.map((group) => group.title)).toEqual(['Cargo', 'SeamJS']);
		expect(groups[0]?.sections.map((section) => section.heading)).toEqual(['one', 'two']);
	});

	it('places a group where its best section was, not where its last one is', () => {
		// b's only match outranks a's second, and a still leads because a's first outranks both.
		const groups = groupHits([hit('a', 'one'), hit('b', 'two'), hit('a', 'three')]);
		expect(groups.map((group) => group.path)).toEqual(['a', 'b']);
		expect(groups[0]?.sections.map((section) => section.heading)).toEqual(['one', 'three']);
	});

	it('caps sections per group and groups per answer', () => {
		const many = ['a', 'b', 'c', 'd', 'e', 'f'].flatMap((path) =>
			['1', '2', '3', '4'].map((heading) => hit(path, heading)),
		);
		const groups = groupHits(many);
		expect(groups).toHaveLength(5);
		for (const group of groups) expect(group.sections).toHaveLength(3);
	});

	it('has nothing to group when nothing matched', () => {
		expect(groupHits([])).toEqual([]);
	});
});

describe('groupHits and one destination per row', () => {
	const chunk = (path: string, heading: string, text: string): SearchHit => ({
		objectID: `${path}:mw:${heading}:${text}`,
		path,
		locale: 'mw',
		url: `https://example.test/${path}#${heading}`,
		title: 'Cargo',
		subtitle: '',
		heading,
		text,
	});

	it('keeps one row when a long section was stored as several records', () => {
		// Both chunks carry the same heading and the same anchor, so both go to one place.
		const groups = groupHits([
			chunk('a', 'bench', 'first half'),
			chunk('a', 'bench', 'second half'),
			chunk('a', 'panic', 'other'),
		]);
		expect(groups[0]?.sections.map((section) => section.heading)).toEqual(['bench', 'panic']);
		expect(groups[0]?.sections[0]?.text).toBe('first half');
	});
});
