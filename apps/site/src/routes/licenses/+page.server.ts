import { licenseTerms, packages, routePath } from '$lib/licenses';
import type { PageServerLoad } from './$types';

// Locale is negotiated per request like every other page here, so this one is rendered rather
// than baked. The plain-text routes beside it are the prerendered ones, because a licence is
// not translated and they vary on nothing. See spec/locale.md.
export const prerender = false;

type Entry = {
	/** `serde 1.0.219` -- the registry is the group's, so the row does not repeat it. */
	name: string;
	version: string;
	href: string;
	authors: string;
	/** The whole expression, shown when the group heading is only part of it. */
	spdx: string;
	texts: number;
	/** The package declared nothing; the expression was read off what it ships. */
	asserted: boolean;
};

type Group = {
	license: string;
	anchor: string;
	entries: Entry[];
};

/** A heading id from an SPDX expression: `MIT OR Apache-2.0` becomes `mit-or-apache-2-0`. */
function anchor(license: string): string {
	return license
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
}

/** Split a purl back into the parts a reader reads, without going through the route path. */
function coordinates(purl: string): { registry: string; name: string; version: string } {
	const [registry = '', rest = ''] = routePath(purl).split(/\/(.*)/s);
	const at = rest.lastIndexOf('@');
	return {
		registry,
		name: at === -1 ? rest : rest.slice(0, at),
		version: at === -1 ? '' : rest.slice(at + 1),
	};
}

/**
 * The package list, grouped by the licence it is under and ordered by how much of the tree
 * each licence accounts for.
 *
 * Grouping rather than tabulating, because the licence is the one column that repeats itself
 * hundreds of times: as a heading it is written once and the rows underneath get shorter. It
 * also makes the page answer the question somebody arriving actually has, which is what this
 * is all standing on rather than what any single package is.
 */
export const load: PageServerLoad = ({ locals }) => {
	const groups = new Map<string, Entry[]>();
	const registries = new Set<string>();

	for (const [purl, entry] of packages()) {
		const { registry, name, version } = coordinates(purl);
		registries.add(registry);
		const spdx = entry.spdx ?? '';
		const row = {
			name,
			version,
			href: `/licenses/${routePath(purl)}.txt`,
			authors: entry.authors?.join(', ') ?? '',
			spdx,
			texts: entry.texts?.length ?? 0,
			asserted: entry.asserted === true,
		};
		// Filed under every licence the expression names, so somebody looking for what is
		// Apache-licensed here finds the packages that merely offer it as one of two. An
		// asserted licence is not a group of its own: those packages are MIT and simply never
		// said so, and where that is known from belongs on the row.
		for (const license of licenseTerms(spdx)) {
			groups.set(license, [...(groups.get(license) ?? []), row]);
		}
	}

	const grouped: Group[] = [...groups]
		// Commonest first, and alphabetically where two licences cover the same amount, so the
		// order is stable across runs rather than dependent on which package was seen first.
		.toSorted(([aName, a], [bName, b]) => b.length - a.length || aName.localeCompare(bName))
		.map(([license, entries]) => ({
			license,
			anchor: anchor(license),
			entries: entries.toSorted((a, b) => a.name.localeCompare(b.name)),
		}));

	return {
		groups: grouped,
		// The package count, not the sum of the groups: a package offering a choice is in more
		// than one of them, so the groups deliberately add up to more than this.
		total: packages().length,
		registries: registries.size,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
