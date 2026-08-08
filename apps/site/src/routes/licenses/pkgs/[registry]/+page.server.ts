import { error } from '@sveltejs/kit';
import { URLS } from '@canmi/urls';
import { packageRows, REGISTRY_NAMES } from '$lib/licenses/directory';
import type { PageServerLoad } from './$types';

export const prerender = false;

export const load: PageServerLoad = ({ params, locals }) => {
	const rows = packageRows()
		.filter(({ registry }) => registry === params.registry)
		.toSorted((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
	if (rows.length === 0) error(404, 'Registry not found');

	return {
		registry: {
			id: params.registry,
			name: REGISTRY_NAMES[params.registry] ?? params.registry,
			href: URLS.external.registries[params.registry as keyof typeof URLS.external.registries],
		},
		rows,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
