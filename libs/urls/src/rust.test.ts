import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { rustUrlMap } from './rust.ts';

it('keeps the committed Rust mirror in step with the map', () => {
	const committed = readFileSync(new URL('../../../apps/cms/src/urls.rs', import.meta.url), 'utf8');
	// A mismatch means the map changed without `mise run urls` -- regenerate rather than edit
	// the Rust file, which carries a do-not-edit header for this reason.
	expect(committed).toBe(rustUrlMap());
});
