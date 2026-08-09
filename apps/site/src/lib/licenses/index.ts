import { licenses } from 'virtual:licenses';
import {
	dependentsOf,
	routeTable,
	type Dependents,
	type LicensePackage,
	type LicenseRecord,
} from './record.ts';

export {
	HEADER,
	TEXT_HEADERS,
	coordinates,
	fullUrl,
	githubAvatar,
	githubRepository,
	licenseOf,
	licenseSlug,
	licenseTerms,
	packagePagePath,
	registryPackageUrl,
	routePath,
	textUrl,
	type Dependents,
	type GithubRepository,
	type LicensePackage,
	type LicensePerson,
	type LicenseRecord,
	type LicenseText,
	type PackageCoordinates,
} from './record.ts';

// The data binding, kept apart from record.ts so the logic above it can be exercised without
// Vite -- the same split the content library uses between its build and its lookups.
const record = licenses as LicenseRecord;
const byRoute = routeTable(record);

export function packages(): [string, LicensePackage][] {
	return Object.entries(record.packages);
}

export function dependents(purl: string): Dependents {
	return dependentsOf(record, purl);
}

export function find(route: string): { purl: string; package: LicensePackage } | undefined {
	const purl = byRoute.get(route);
	if (purl === undefined) return undefined;
	const found = record.packages[purl];
	return found && { purl, package: found };
}
