import { URLS } from './index.ts';

/**
 * The Rust mirror of the URL map.
 *
 * cms is a Rust process and cannot import this library, so it reads a generated
 * `apps/cms/src/urls.rs` instead -- committed like the records under `data/build/`, because a
 * checkout must compile without a Node toolchain having run first. `mise run urls` rewrites it;
 * `rust.test.ts` fails when the committed file no longer matches this render, so drift between
 * the two languages cannot survive `mise run verify`. See spec/architecture.md.
 */
export function rustUrlMap(): string {
	const pairs: Array<[name: string, value: string]> = [];
	walk(URLS, [], pairs);
	const constants = pairs
		.map(([name, value]) => `pub const ${name}: &str = "${value}";`)
		.join('\n');
	return [
		'//! Generated from libs/urls/src/index.ts by `mise run urls`; do not edit.',
		'//! One URL map for both languages -- see spec/architecture.md.',
		'',
		constants,
		'',
	].join('\n');
}

function walk(value: unknown, path: string[], out: Array<[string, string]>): void {
	if (typeof value === 'string') {
		const name = path
			.map((segment) => segment.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase())
			.join('_');
		out.push([name, value]);
		return;
	}
	for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
		walk(child, [...path, key], out);
	}
}
