import { hash } from 'blake3-wasm';
import { PUBLIC_LANGUAGE } from '$lib/locale';

export type TranslationLocale = (typeof PUBLIC_LANGUAGE)[keyof typeof PUBLIC_LANGUAGE];

export type TranslationSidecar = {
	segments?: Record<string, Partial<Record<TranslationLocale, { text: string }>>>;
};

type Segment = { id: string; source: string; translatable: boolean };

function articleBody(raw: string): { prefix: string; body: string } {
	if (!raw.startsWith('---\n')) return { prefix: '', body: raw };
	const end = raw.indexOf('\n---', 4);
	if (end < 0) return { prefix: '', body: raw };
	const bodyStart = end + 4;
	return { prefix: raw.slice(0, bodyStart), body: raw.slice(bodyStart) };
}

function normaliseSegment(source: string): string {
	return source.trim().split(/\s+/u).join(' ');
}

export function segmentId(source: string): string {
	return hash(normaliseSegment(source)).toString('hex').slice(0, 32);
}

/** Keep this byte-for-byte aligned with the CMS block splitter that addresses the sidecar. */
export function splitSegments(raw: string): Segment[] {
	const { body } = articleBody(raw);
	const segments: Segment[] = [];
	let block: string[] = [];
	let fenced = false;

	const push = () => {
		if (block.length === 0) return;
		const source = block.join('\n');
		block = [];
		const trimmed = source.trim();
		if (!trimmed) return;
		segments.push({
			id: segmentId(source),
			source,
			translatable: !trimmed.startsWith('```'),
		});
	};

	for (const line of body.split('\n')) {
		if (line.trimStart().startsWith('```')) {
			if (fenced) {
				block.push(line);
				push();
				fenced = false;
				continue;
			}
			push();
			fenced = true;
			block.push(line);
			continue;
		}
		if (fenced) {
			block.push(line);
			continue;
		}
		if (!line.trim()) {
			push();
			continue;
		}
		block.push(line);
	}
	push();
	return segments;
}

export function assemble(
	raw: string,
	sidecar: TranslationSidecar,
	locale: TranslationLocale,
): { raw: string; missing: string[] } {
	const { prefix, body } = articleBody(raw);
	const missing: string[] = [];
	let cursor = 0;
	let translated = '';

	for (const segment of splitSegments(raw)) {
		const offset = body.indexOf(segment.source, cursor);
		if (offset < 0) throw new Error(`cannot locate article segment ${segment.id}`);
		translated += body.slice(cursor, offset);
		const entry = sidecar.segments?.[segment.id]?.[locale];
		if (segment.translatable && !entry) missing.push(segment.id);
		translated += segment.translatable && entry ? entry.text : segment.source;
		cursor = offset + segment.source.length;
	}
	translated += body.slice(cursor);
	return { raw: prefix + translated, missing };
}

function normaliseArticle(raw: string): string[] {
	const { body } = articleBody(raw);
	return Array.from(
		body
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
	const a = normaliseArticle(left);
	const b = normaliseArticle(right);
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
