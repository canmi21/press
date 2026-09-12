import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { sourceFingerprint } from './assemble';
import { createDiagramResolver, EXTENSION } from './assets';

/**
 * The Rust function that names a published file, read out of its source.
 *
 * This table rebuilds a URL for a file `cms image` already named, so the two have to spell every
 * format the same way. Nothing connected them: both said `jpg` for a JPEG, and the CDN redirects
 * `.jpg` to `.jpeg` -- so agreeing was not enough, they had to agree on the spelling the CDN
 * serves directly. See spec/architecture/delivery.md.
 */
const ARM = /"(image\/[a-z+]+)" => (?:"([a-z0-9]+)"|(JPEG))/g;

it('names each format the way apps/cms names the file', () => {
	const source = readFileSync(
		fileURLToPath(new URL('../../../../../cms/src/extension.rs', import.meta.url)),
		'utf8',
	);
	const body = /pub fn for_variant\(mime: &str\) -> &'static str \{([\s\S]*?)\n\}/.exec(source);
	expect(body, 'for_variant moved or changed shape').not.toBeNull();

	// `JPEG` is a constant on the Rust side rather than a literal, so the spelling it holds is
	// resolved too -- the point of the constant is that both naming paths share one spelling.
	const jpeg = /const JPEG: &str = "([a-z]+)"/.exec(source)?.[1];
	const authoritative = Object.fromEntries(
		[...body![1]!.matchAll(ARM)].map((m) => [m[1], m[3] ? jpeg : m[2]]),
	);
	expect(Object.keys(authoritative).length).toBeGreaterThan(0);

	for (const [mime, extension] of Object.entries(authoritative)) {
		expect(EXTENSION[mime], `${mime} is spelled differently on each side`).toBe(extension);
	}
});

/**
 * The key both sides agree on, held to the Rust that writes it.
 *
 * The record's own key is a BLAKE3 content id this side cannot compute, so a diagram is found by
 * the cheap checksum the segment layout already uses -- over the block's whole source, which the
 * compiler reads back out of the article by the node's own position. If either side changed which
 * bytes it fingerprints, every description would go quietly missing and nothing else would break.
 */
it('finds a diagram by the checksum the CMS wrote, over the block source it fingerprints', () => {
	const payload = '```mermaid\ngraph TD\nA-->B\n```';
	const store = {
		diagrams: {
			'0123456789abcdef0123456789abcdef': {
				fingerprint: sourceFingerprint(new TextEncoder().encode(payload)),
				description: {
					'en-US': { text: 'A goes to B.' },
					'zh-CN': { text: 'A 指向 B。' },
				},
			},
		},
	};

	expect(createDiagramResolver(store, 'en-US')(payload)).toBe('A goes to B.');
	expect(createDiagramResolver(store, 'zh-CN')(payload)).toBe('A 指向 B。');
	// A locale nobody has translated into falls back to nothing rather than to English: the
	// caller's fallback is what the block said without a description at all.
	expect(createDiagramResolver(store, 'ko-KR')(payload)).toBeUndefined();
	// The payload alone is not the block. Getting this wrong is the failure this test exists for:
	// every description would go quietly missing and nothing else would break.
	expect(createDiagramResolver(store, 'en-US')('graph TD\nA-->B')).toBeUndefined();
	expect(createDiagramResolver({ diagrams: {} }, 'en-US')(payload)).toBeUndefined();
});
