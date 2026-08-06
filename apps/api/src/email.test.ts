import { describe, expect, it } from 'vitest';
import { canonicalEmail } from './email';

describe('canonicalEmail', () => {
	it.each([
		[' Alice+notes@Example.com ', 'alice@example.com'],
		['reader@sub.example.com', 'reader@sub.example.com'],
		['first.last!tag@example.co.uk', 'first.last!tag@example.co.uk'],
	])('canonicalizes %j', (input, expected) => {
		expect(canonicalEmail(input)).toBe(expected);
	});

	it.each([
		undefined,
		'',
		'name',
		'name@localhost',
		'name@@example.com',
		'+tag@example.com',
		'.name@example.com',
		'name..part@example.com',
		'name@-example.com',
		'name@example-.com',
		'name@例子.中国',
	])('rejects %j', (input) => {
		expect(canonicalEmail(input)).toBeUndefined();
	});
});
