import { describe, expect, it } from 'vitest';
import { maskEmail } from './mask';

describe('maskEmail', () => {
	it('keeps a public provider readable', () => {
		expect(maskEmail('canmi@gmail.com')).toBe('c••••@gmail.com');
	});

	it('hides a domain the reader controls, because the name is the identity', () => {
		expect(maskEmail('hi@canmi.net')).toBe('h•@•••••.net');
	});

	it('masks every label of a private domain but the last', () => {
		expect(maskEmail('hi@mail.canmi.co.uk')).toBe('h•@••••.•••••.••.uk');
	});

	it('hides a one-character local part entirely, since its first character is all of it', () => {
		expect(maskEmail('a@qq.com')).toBe('•@qq.com');
	});

	it('canonicalises case before matching the provider list', () => {
		expect(maskEmail('Canmi@GMail.com')).toBe('c••••@gmail.com');
	});

	it('reveals nothing about a value that is not an address', () => {
		expect(maskEmail('@gmail.com')).toBe('••••');
		expect(maskEmail('canmi')).toBe('••••');
	});
});
