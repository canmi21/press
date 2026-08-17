import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { rustUrlMap } from './rust.ts';

/**
 * Both sides with each constant on one line, however rustfmt chose to lay it out.
 *
 * The generator emits one constant per line; rustfmt wraps the ones that pass its width, which
 * put a byte comparison permanently out of step with the committed file and left this test
 * failing for a URL nobody had changed. What the mirror has to agree on is which names hold which
 * strings, so where the line breaks is exactly the difference to discard.
 */
function constants(rust: string): string {
	return rust.replace(/=\s+"/g, '= "');
}

it('keeps the committed Rust mirror in step with the map', () => {
	const committed = readFileSync(new URL('../../../apps/cms/src/urls.rs', import.meta.url), 'utf8');
	// A mismatch means the map changed without `mise run urls` -- regenerate rather than edit
	// the Rust file, which carries a do-not-edit header for this reason.
	expect(constants(committed)).toBe(constants(rustUrlMap()));
});
