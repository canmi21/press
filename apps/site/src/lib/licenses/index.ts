import { licenses } from 'virtual:licenses';
import { routeTable, type LicensePackage, type LicenseRecord } from './record.ts';

export {
	HEADER,
	TEXT_HEADERS,
	fullUrl,
	licenseOf,
	licenseTerms,
	routePath,
	textUrl,
	type LicensePackage,
	type LicenseRecord,
	type LicenseText,
} from './record.ts';

// The data binding, kept apart from record.ts so the logic above it can be exercised without
// Vite -- the same split the content library uses between its build and its lookups.
const record = licenses as LicenseRecord;
const byRoute = routeTable(record);

export function packages(): [string, LicensePackage][] {
	return Object.entries(record.packages);
}

export function find(route: string): { purl: string; package: LicensePackage } | undefined {
	const purl = byRoute.get(route);
	if (purl === undefined) return undefined;
	const found = record.packages[purl];
	return found && { purl, package: found };
}
