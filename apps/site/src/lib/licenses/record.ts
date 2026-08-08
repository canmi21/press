import { URLS } from '@canmi/urls';

export type LicenseText = {
	/** The file name the package shipped the text under, such as `LICENSE-MIT`. */
	name: string;
	/** BLAKE3-128 of the text's own bytes; the object the CDN serves it as. */
	cid: string;
};

export type LicensePackage = {
	spdx?: string;
	/** True when the expression was worked out by reading the package, not declared by it. */
	asserted?: boolean;
	authors?: string[];
	texts?: LicenseText[];
};

export type LicenseRecord = {
	version: number;
	/** Keyed by purl: `pkg:npm/%40sveltejs/kit@2.0.0`, `pkg:cargo/serde@1.0.219`. */
	packages: Record<string, LicensePackage>;
};

/**
 * The sentence every licence route opens with.
 *
 * The routes credit other people's work, so they have to say what the code around that credit
 * is, or a reader has the terms of the dependencies and nothing about the thing using them.
 */
export const HEADER = `Released under the MIT License. Source: ${URLS.source}`;

/**
 * A purl as a route path: `pkg:npm/%40sveltejs/kit@2.0.0` becomes `npm/@sveltejs/kit@2.0.0`.
 *
 * A purl is precise and unpleasant to type. The route drops the scheme and undoes the escaping
 * that only exists to survive being one opaque string, which leaves a path that reads like the
 * package it names. Nothing parses this back -- the reverse lookup is a table built from the
 * same function, so the two spellings cannot drift into disagreeing about an edge case such as
 * the slash inside a scoped npm name.
 */
export function routePath(purl: string): string {
	return decodeURIComponent(purl.replace(/^pkg:/, ''));
}

export function routeTable(record: LicenseRecord): Map<string, string> {
	return new Map(Object.keys(record.packages).map((purl) => [routePath(purl), purl]));
}

/**
 * The distinct licences an SPDX expression names, in the order it names them.
 *
 * A package offering `MIT OR Apache-2.0` is under both as far as finding it goes -- somebody
 * looking for what is Apache-licensed here wants it in that list -- so the expression is
 * flattened to its leaves and the package is filed under each. Which of them actually applies
 * is a choice the reader makes, and the unflattened expression stays on the row so the choice
 * is never hidden behind the grouping.
 *
 * `AND` flattens the same way while meaning the opposite: a conjunction has to be satisfied in
 * full rather than picked from. Both still put the package in both lists, which is what the
 * grouping is for; the expression beside it is what says which kind it is.
 *
 * Four things this must not do, each of them present in the current tree:
 *
 * - split `Apache-2.0 WITH LLVM-exception`, which is one licence and not two
 * - split `FSL-1.1-MIT` or `MIT-0`, whose identifiers merely contain a shorter one
 * - read the `or` inside `LGPL-2.1-or-later` as the operator, which is why matching is done on
 *   whole tokens and case-sensitively, the way SPDX writes its operators
 * - keep the parentheses of `(MIT OR Apache-2.0) AND NCSA` attached to an identifier
 *
 * `/` is Cargo's deprecated spelling of `OR` and is treated as one. Nothing else uses it.
 */
export function licenseTerms(expression: string): string[] {
	const tokens = expression
		.replace(/[()]/g, ' ')
		.replace(/\//g, ' OR ')
		.split(/\s+/)
		.filter(Boolean);

	const terms: string[] = [];
	for (const token of tokens) {
		if (token === 'OR' || token === 'AND') continue;
		// `WITH` binds an exception onto the identifier before it, producing one term rather
		// than joining two.
		if (token === 'WITH') {
			const previous = terms.pop();
			if (previous !== undefined) terms.push(`${previous} WITH`);
			continue;
		}
		const last = terms.at(-1);
		if (last?.endsWith(' WITH')) {
			terms[terms.length - 1] = `${last} ${token}`;
			continue;
		}
		terms.push(token);
	}

	return [...new Set(terms)];
}

/** How the licence is known, said in the shortest form that stays true. */
export function licenseOf(entry: LicensePackage): string {
	if (!entry.spdx) return 'not declared';
	return entry.asserted ? `${entry.spdx} (asserted)` : entry.spdx;
}

export function textUrl(cdn: string, cid: string): string {
	return `${cdn}/license/${cid.slice(0, 2)}/${cid.slice(2, 4)}/${cid}.txt`;
}

export function fullUrl(cdn: string): string {
	return `${cdn}/license/full.txt`;
}

/**
 * Plain text, never a page, and never negotiated by locale.
 *
 * A licence is not translated -- a translated one is a different licence -- so unlike every
 * other route here these vary on nothing. That also keeps them cacheable by one shared key.
 */
export const TEXT_HEADERS = {
	'Content-Type': 'text/plain; charset=utf-8',
	'Cache-Control': 'public, max-age=300, s-maxage=3600',
} as const;
