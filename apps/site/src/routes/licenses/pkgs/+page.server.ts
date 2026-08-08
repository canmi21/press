import { URLS } from '@canmi/urls';
import { packageRows, REGISTRY_NAMES } from '$lib/licenses/directory';
import type { PageServerLoad } from './$types';

export const prerender = false;

export const load: PageServerLoad = ({ locals }) => {
	const rows = packageRows();
	const counts = new Map<string, number>();
	for (const { registry } of rows) counts.set(registry, (counts.get(registry) ?? 0) + 1);

	return {
		registries: [...counts]
			.map(([id, count]) => ({
				id,
				name: REGISTRY_NAMES[id] ?? id,
				count,
				href: `/licenses/pkgs/${id}`,
				externalHref: URLS.external.registries[id as keyof typeof URLS.external.registries],
			}))
			.toSorted((a, b) => b.count - a.count || a.name.localeCompare(b.name)),
		total: rows.length,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
