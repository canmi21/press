import { PUBLIC_LANGUAGE } from '$lib/locale';

export type TranslationLocale = (typeof PUBLIC_LANGUAGE)[keyof typeof PUBLIC_LANGUAGE];

export type TranslationSidecar = {
	segments?: Record<string, Partial<Record<TranslationLocale, { text: string }>>>;
};

export type SegmentSpan = {
	id: string;
	start: number;
	end: number;
	fingerprint: string;
	region: 'frontmatter' | 'body';
};

export type SegmentLayout = {
	version: number;
	articles: Record<string, SegmentSpan[]>;
};

/** FNV-1a over source bytes: a drift detector, not a content address. */
export function sourceFingerprint(bytes: Uint8Array): string {
	let checksum = 0x811c9dc5;
	for (const byte of bytes) checksum = Math.imul(checksum ^ byte, 0x01000193);
	return (checksum >>> 0).toString(16).padStart(8, '0');
}

export function assemble(
	raw: string,
	spans: readonly SegmentSpan[],
	sidecar: TranslationSidecar,
	locale: TranslationLocale,
	article: string,
): {
	raw: string;
	missing: string[];
	translatable: { source: string; translated: string };
} {
	const bytes = new TextEncoder().encode(raw);
	const decoder = new TextDecoder('utf-8', { fatal: true });
	const missing: string[] = [];

	let previousEnd = 0;
	for (const span of spans) {
		if (
			!Number.isSafeInteger(span.start) ||
			!Number.isSafeInteger(span.end) ||
			span.start < previousEnd ||
			span.end <= span.start ||
			span.end > bytes.length ||
			(span.region !== 'frontmatter' && span.region !== 'body')
		) {
			throw new Error(`${article}: invalid source range for article segment ${span.id}`);
		}
		if (sourceFingerprint(bytes.subarray(span.start, span.end)) !== span.fingerprint) {
			throw new Error(`${article}: stale segment layout at ${span.id}; run \`cms segments\``);
		}
		previousEnd = span.end;
	}

	let cursor = 0;
	let translated = '';
	const sourceContent: string[] = [];
	const translatedContent: string[] = [];
	for (const span of spans) {
		try {
			translated += decoder.decode(bytes.subarray(cursor, span.start));
			const source = decoder.decode(bytes.subarray(span.start, span.end));
			const entry = sidecar.segments?.[span.id]?.[locale];
			if (!entry && span.region === 'body') missing.push(span.id);
			sourceContent.push(source);
			translatedContent.push(entry?.text ?? source);
			translated += entry
				? span.region === 'frontmatter'
					? ` ${JSON.stringify(entry.text)}`
					: entry.text
				: source;
		} catch {
			throw new Error(
				`${article}: source range splits a UTF-8 character for article segment ${span.id}`,
			);
		}
		cursor = span.end;
	}
	translated += decoder.decode(bytes.subarray(cursor));
	return {
		raw: translated,
		missing,
		translatable: {
			source: sourceContent.join('\n\n'),
			translated: translatedContent.join('\n\n'),
		},
	};
}

function normaliseText(raw: string): string[] {
	return Array.from(
		raw
			.normalize('NFKC')
			.toLocaleLowerCase()
			.replace(/[“”]/gu, '"')
			.replace(/[‘’]/gu, "'")
			.replace(/\s+/gu, ' ')
			.trim(),
	);
}

/** Sørensen-Dice over character pairs: punctuation drift is cheap; rewritten prose is not. */
export function similarity(left: string, right: string): number {
	const a = normaliseText(left);
	const b = normaliseText(right);
	if (a.length < 2 || b.length < 2) return a.join('') === b.join('') ? 1 : 0;

	const pairs = new Map<string, number>();
	for (let index = 0; index < a.length - 1; index += 1) {
		const pair = `${a[index]}${a[index + 1]}`;
		pairs.set(pair, (pairs.get(pair) ?? 0) + 1);
	}
	let shared = 0;
	for (let index = 0; index < b.length - 1; index += 1) {
		const pair = `${b[index]}${b[index + 1]}`;
		const remaining = pairs.get(pair) ?? 0;
		if (remaining === 0) continue;
		shared += 1;
		pairs.set(pair, remaining - 1);
	}
	return (2 * shared) / (a.length + b.length - 2);
}
