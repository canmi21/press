import { describe, expect, it } from 'vitest';
import { fillSlot, spaceScriptBoundaries } from './spacing.ts';

/** What `Intl.ListFormat` hands back, as the parts it hands them back in. */
const listed = (parts: readonly string[]) => spaceScriptBoundaries(parts).join('');

describe('spacing a Latin run against CJK', () => {
	// The case this exists for: Chinese joins a list with a bare connective.
	it('opens both sides of a connective between two Latin names', () => {
		expect(listed(['crates.io', '和', 'npm'])).toBe('crates.io 和 npm');
	});

	it('leaves a Latin list in a Latin locale alone', () => {
		expect(listed(['crates.io', ' and ', 'npm'])).toBe('crates.io and npm');
	});

	it('spaces Japanese and Korean connectives the same way', () => {
		expect(listed(['crates.io', 'と', 'npm'])).toBe('crates.io と npm');
		expect(listed(['crates.io', '및', 'npm'])).toBe('crates.io 및 npm');
	});

	// Full-width punctuation carries its own space inside the glyph. Another one beside it
	// opens a hole, which is why this matches script letters rather than a block range.
	it('keeps CJK punctuation tight against a Latin run', () => {
		expect(listed(['npm', '，分属'])).toBe('npm，分属');
		expect(listed(['crates.io', '、', 'npm'])).toBe('crates.io、npm');
		expect(listed(['来自', 'npm', '。'])).toBe('来自 npm。');
	});

	it('adds nothing where a space was already written', () => {
		expect(listed(['来自 ', 'npm'])).toBe('来自 npm');
	});

	it('leaves a run that never changes script alone', () => {
		expect(listed(['来自', '仓库'])).toBe('来自仓库');
		expect(listed(['from', 'npm'])).toBe('fromnpm');
	});

	it('returns the first part untouched', () => {
		expect(spaceScriptBoundaries(['npm'])).toEqual(['npm']);
		expect(spaceScriptBoundaries([])).toEqual([]);
	});
});

describe('filling a slot in a rendered sentence', () => {
	const fill = (sentence: string, value: string) => fillSlot(sentence, '\u0000', value);

	it('spaces the join by what meets there, not by the language of the sentence', () => {
		// Chinese types no space and needs one here; the same sentence needs none when the value
		// is itself CJK.
		expect(fill('已为你显示\u0000', 'English (US)')).toBe('已为你显示 English (US)');
		expect(fill('已为你显示\u0000', '日本語 (JP)')).toBe('已为你显示日本語 (JP)');
	});

	it('adds nothing where the sentence already separates the two', () => {
		// A Japanese comma carries its own trailing space in the glyph, and Korean and every Latin
		// sentence have typed one already. Each would otherwise gain a second.
		expect(fill('ないため、\u0000を表示しています', 'English (US)')).toBe(
			'ないため、English (US)を表示しています',
		);
		expect(fill('언어는 \u0000입니다', 'English (US)')).toBe('언어는 English (US)입니다');
		expect(fill('reading \u0000', 'English (US)')).toBe('reading English (US)');
	});

	it('spaces the far side too, when the sentence continues in another script', () => {
		expect(fill('showing \u0000 now', '日本語 (JP)')).toBe('showing 日本語 (JP) now');
	});
});
