import {
	coordinates,
	find,
	licenseSlug,
	licenseTerms,
	packagePagePath,
	packages,
	routePath,
	type LicensePackage,
	type PackageCoordinates,
} from '$lib/licenses';

export const REGISTRY_NAMES: Record<string, string> = {
	cargo: 'crates.io',
	npm: 'npm',
};

export type PackageRow = PackageCoordinates & {
	purl: string;
	href: string;
	textHref: string;
	spdx: string;
	asserted: boolean;
	authors: string;
	texts: number;
};

export type LicenseDirectoryEntry = {
	license: string;
	slug: string;
	count: number;
};

function row(purl: string, entry: LicensePackage): PackageRow {
	return {
		purl,
		...coordinates(purl),
		href: packagePagePath(purl),
		textHref: `/licenses/${routePath(purl)}.txt`,
		spdx: entry.spdx ?? '',
		asserted: entry.asserted === true,
		authors: entry.authors?.map(({ name }) => name).join(', ') ?? '',
		texts: entry.texts?.length ?? 0,
	};
}

const allRows = packages().map(([purl, entry]) => row(purl, entry));
const licenseRows = new Map<string, PackageRow[]>();

for (const item of allRows) {
	for (const license of licenseTerms(item.spdx)) {
		licenseRows.set(license, [...(licenseRows.get(license) ?? []), item]);
	}
}

const licenses = [...licenseRows]
	.map(([license, rows]) => ({ license, slug: licenseSlug(license), count: rows.length }))
	.toSorted((a, b) => b.count - a.count || a.license.localeCompare(b.license));
const licensesBySlug = new Map(licenses.map((entry) => [entry.slug, entry]));

if (licensesBySlug.size !== licenses.length) {
	throw new Error('Two license terms produce the same route slug');
}

export function packageRows(): PackageRow[] {
	return allRows;
}

export function licenseDirectory(): LicenseDirectoryEntry[] {
	return licenses;
}

export function packagesForLicense(slug: string):
	| {
			license: LicenseDirectoryEntry;
			rows: PackageRow[];
	  }
	| undefined {
	const license = licensesBySlug.get(slug);
	if (!license) return undefined;
	return {
		license,
		rows: (licenseRows.get(license.license) ?? []).toSorted(
			(a, b) => a.registry.localeCompare(b.registry) || a.name.localeCompare(b.name),
		),
	};
}

export function packageForRoute(route: string) {
	const found = find(route);
	if (!found) return undefined;
	return {
		...found,
		coordinates: coordinates(found.purl),
		row: row(found.purl, found.package),
	};
}
