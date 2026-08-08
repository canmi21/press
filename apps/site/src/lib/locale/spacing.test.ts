import { describe, expect, it } from 'vitest';
import { spaceScriptBoundaries } from './spacing.ts';

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
