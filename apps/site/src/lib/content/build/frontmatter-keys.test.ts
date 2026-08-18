import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { TRANSLATABLE_FRONTMATTER } from './compile';

/**
 * The Rust declaration this list is a copy of.
 *
 * Read out of the source rather than out of a generated artefact, so there is no regeneration
 * step to forget. The declared length is captured too: a list edited without its count is a Rust
 * compile error, and matching both here means this test cannot pass against a half-edited one.
 */
const DECLARATION = /const TRANSLATABLE_FRONTMATTER: \[&str; (\d+)\] = \[([^\]]*)\]/;

it('lists the frontmatter keys cms i18n actually translates', () => {
	const source = readFileSync(
		fileURLToPath(new URL('../../../../../cms/src/i18n/segment.rs', import.meta.url)),
		'utf8',
	);

	const declaration = DECLARATION.exec(source);
	expect(declaration, 'the Rust declaration moved or changed shape').not.toBeNull();

	const [, count, body] = declaration!;
	const authoritative = [...body!.matchAll(/"([^"]+)"/g)].map((match) => match[1]);

	expect(authoritative).toHaveLength(Number(count));
	// Order matters as little as the count does, but comparing both is free and a reordering is
	// worth seeing: it usually means somebody edited one list and retyped the other.
	expect([...TRANSLATABLE_FRONTMATTER]).toEqual(authoritative);
});
