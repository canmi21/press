import { URLS } from '@canmi/urls';
import { licenseDirectory, packageRows, REGISTRY_NAMES } from '$lib/licenses/directory';
import type { PageServerLoad } from './$types';

export const prerender = false;

export const load: PageServerLoad = ({ locals }) => {
	const rows = packageRows();
	const registries = [...new Set(rows.map(({ registry }) => registry))].toSorted().map((id) => ({
		id,
		name: REGISTRY_NAMES[id] ?? id,
		href: URLS.external.registries[id as keyof typeof URLS.external.registries],
	}));

	return {
		licenses: licenseDirectory(),
		total: rows.length,
		registries,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
