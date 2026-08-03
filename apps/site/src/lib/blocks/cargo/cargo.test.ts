import { describe, expect, it } from 'vitest';
import { dependencyItems } from './cargo';
import type { CrateDep } from '../../content/types';

function dep(name: string, version: string, depth: number): CrateDep {
	return {
		name,
		version,
		kind: 'normal',
		optional: false,
		target: null,
		features: [],
		size: 100,
		depth,
	};
}

describe('Cargo dependency identities', () => {
	it('remain unique when a real graph contains several versions of one crate', () => {
		// seam-cli currently contains this shape. Keying its tiles by name crashed hydration at
		// `windows-sys`, leaving the server-rendered widgets missing from the live article.
		const graph = [
			dep('windows-sys', '0.60.2', 1),
			dep('windows-sys', '0.61.2', 2),
			dep('syn', '2.0.119', 2),
			dep('syn', '3.0.3', 3),
		];
		const keys = dependencyItems(graph).map(({ key }) => key);
		expect(new Set(keys).size).toBe(graph.length);
	});

	it('does not trust even a repeated full identity to be unique', () => {
		// Historical manifests may predate resolver deduplication; rendering them must not crash.
		const repeated = dep('syn', '2.0.119', 2);
		const keys = dependencyItems([repeated, repeated]).map(({ key }) => key);
		expect(new Set(keys).size).toBe(2);
	});
});
