import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { EXTENSION } from './assets';

/**
 * The Rust function that names a published file, read out of its source.
 *
 * This table rebuilds a URL for a file `cms image` already named, so the two have to spell every
 * format the same way. Nothing connected them: both said `jpg` for a JPEG, and the CDN redirects
 * `.jpg` to `.jpeg` -- so agreeing was not enough, they had to agree on the spelling the CDN
 * serves directly. See spec/architecture/delivery.md.
 */
const ARM = /"(image\/[a-z+]+)" => "([a-z0-9]+)"/g;

it('names each format the way apps/cms names the file', () => {
	const source = readFileSync(
		fileURLToPath(new URL('../../../../../cms/src/image/run.rs', import.meta.url)),
		'utf8',
	);
	const body = /fn extension_of\(mime: &str\) -> &'static str \{([\s\S]*?)\n\}/.exec(source);
	expect(body, 'extension_of moved or changed shape').not.toBeNull();

	const authoritative = Object.fromEntries([...body![1]!.matchAll(ARM)].map((m) => [m[1], m[2]]));
	expect(Object.keys(authoritative).length).toBeGreaterThan(0);

	for (const [mime, extension] of Object.entries(authoritative)) {
		expect(EXTENSION[mime], `${mime} is spelled differently on each side`).toBe(extension);
	}
});
