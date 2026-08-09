import { error } from '@sveltejs/kit';
import { URLS } from '@canmi/urls';
import {
	coordinates,
	githubRepository,
	licenseSlug,
	licenseTerms,
	packagePagePath,
	registryPackageUrl,
} from '$lib/licenses';
import { packageForRoute, REGISTRY_NAMES } from '$lib/licenses/directory';
import type { PageServerLoad } from './$types';

export const prerender = false;

export const load: PageServerLoad = ({ params, locals }) => {
	const found = packageForRoute(`${params.registry}/${params.package}`);
	if (!found) error(404, 'Package not found');

	return {
		purl: found.purl,
		entry: found.package,
		coordinates: found.coordinates,
		textHref: found.row.textHref,
		registry: {
			name: REGISTRY_NAMES[found.coordinates.registry] ?? found.coordinates.registry,
			directoryHref: `/licenses/pkgs/${found.coordinates.registry}`,
			packageHref: registryPackageUrl(found.coordinates),
		},
		repository: {
			href: found.package.repository,
			github: githubRepository(found.package.repository),
		},
		licenses: licenseTerms(found.package.spdx ?? '').map((license) => ({
			license,
			href: `/licenses/${licenseSlug(license)}`,
		})),
		origins: Object.entries(found.package.origins ?? {}).map(([root, path]) => ({
			root,
			nodes: path.map((node) => {
				if (node.startsWith('workspace:')) {
					return { id: node, name: node.slice('workspace:'.length), version: '', href: '' };
				}
				const parts = coordinates(node);
				return {
					id: node,
					name: parts.name,
					version: parts.version,
					href: node === found.purl ? '' : packagePagePath(node),
				};
			}),
		})),
		githubHref: URLS.external.github.web,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
