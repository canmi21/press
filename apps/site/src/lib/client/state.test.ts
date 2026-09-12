import { beforeEach, describe, expect, it } from 'vitest';
import { forget, recall, remember, type Store, VERSION } from './state';

const KEY = 'state';

/** As much of a store as this module touches, which is why it can be this small. */
function store(): Store & { items: Map<string, string> } {
	const items = new Map<string, string>();
	return {
		items,
		getItem: (key) => items.get(key) ?? null,
		setItem: (key, value) => void items.set(key, value),
	};
}

describe('the reader state record', () => {
	let local: ReturnType<typeof store>;
	beforeEach(() => (local = store()));

	it('writes one key, holding the version and flat dotted names', () => {
		remember(local, 'support.preferred', true);
		expect(JSON.parse(local.getItem(KEY) ?? '{}')).toEqual({
			version: VERSION,
			'support.preferred': true,
		});
		expect(local.items.size).toBe(1);
	});

	it('keeps the other facts when one of them changes', () => {
		remember(local, 'support.preferred', true);
		remember(local, 'reader.something', 'else');
		remember(local, 'support.preferred', false);
		expect(recall(local, 'support.preferred', true)).toBe(false);
		expect(recall(local, 'reader.something', '')).toBe('else');
	});

	it('falls back where nothing is stored, or where the type is not what was asked', () => {
		expect(recall(local, 'support.preferred', false)).toBe(false);
		remember(local, 'support.preferred', 'yes');
		expect(recall(local, 'support.preferred', false)).toBe(false);
	});

	it('falls back on a record that is not a record', () => {
		for (const junk of ['null', '[]', '"text"', '{oops', '7']) {
			local.setItem(KEY, junk);
			expect(recall(local, 'support.preferred', false)).toBe(false);
		}
	});

	it('leaves a record from a later version alone and still reads what it knows', () => {
		// The reader's other device runs a later build, which is what cloud sync will make
		// ordinary. Its keys are not this build's to discard.
		local.setItem(
			KEY,
			JSON.stringify({ version: VERSION + 9, 'support.preferred': true, 'from.tomorrow': 1 }),
		);
		expect(recall(local, 'support.preferred', false)).toBe(true);
		remember(local, 'support.preferred', false);
		const stored = JSON.parse(local.getItem(KEY) ?? '{}');
		expect(stored.version).toBe(VERSION + 9);
		expect(stored['from.tomorrow']).toBe(1);
	});

	it('forgets one key without disturbing the record around it', () => {
		remember(local, 'support.preferred', true);
		remember(local, 'reader.something', 'else');
		forget(local, 'support.preferred');
		const stored = JSON.parse(local.getItem(KEY) ?? '{}');
		expect(stored).toEqual({ version: VERSION, 'reader.something': 'else' });
	});
});
