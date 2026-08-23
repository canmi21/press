import { describe, expect, it } from 'vitest';
import { languageLabel } from './highlight';

describe('code language labels', () => {
	it('uses the canonical display name for ids and aliases', () => {
		expect(languageLabel('html')).toBe('HTML');
		expect(languageLabel('typescript')).toBe('TypeScript');
		expect(languageLabel('ts')).toBe('TypeScript');
		expect(languageLabel('js')).toBe('JavaScript');
		expect(languageLabel('cpp')).toBe('C++');
		expect(languageLabel('csharp')).toBe('C#');
		expect(languageLabel('objective-c')).toBe('Objective-C');
		expect(languageLabel('angular-ts')).toBe('Angular TypeScript');
		expect(languageLabel('shell')).toBe('Shell');
	});

	it('hides plain text and preserves an unknown authored name', () => {
		expect(languageLabel('text')).toBeUndefined();
		expect(languageLabel('plaintext')).toBeUndefined();
		expect(languageLabel('MyDSL')).toBe('MyDSL');
	});
});
