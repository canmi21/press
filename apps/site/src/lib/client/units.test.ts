import { expect, it } from 'vitest';
import { remFromDefaultPixels } from './units';

it('converts authored pixel measurements with the project default ratio', () => {
	expect(remFromDefaultPixels(1)).toBe('0.0625rem');
	expect(remFromDefaultPixels(16)).toBe('1rem');
	expect(remFromDefaultPixels(96)).toBe('6rem');
});
