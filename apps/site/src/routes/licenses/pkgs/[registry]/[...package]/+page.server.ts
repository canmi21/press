import { error } from '@sveltejs/kit';
import { URLS } from '@canmi/urls';
import {
	coordinates,
	dependents,
	githubRepository,
	licenseSlug,
	licenseTerms,
	packagePagePath,
	registryPackageUrl,
} from '$lib/licenses';
import { packageForRoute, REGISTRY_NAMES } from '$lib/licenses/directory';
import type { PageServerLoad } from './$types';

export const prerender = false;

type Node = {
	id: string;
	name: string;
	version: string;
	href: string;
};

/**
 * One label from the record as something a reader can look at.
 *
 * `workspace:` marks this project's own apps and libraries, which have no licence page of
 * their own -- they are the thing the page is crediting dependencies to, not a dependency.
 * They carry no version either, because a version is what pins somebody else's release.
 */
function node(label: string, self: string): Node {
	if (label.startsWith('workspace:')) {
		return { id: label, name: label.slice('workspace:'.length), version: '', href: '' };
	}
	const parts = coordinates(label);
	return {
		id: label,
		name: parts.name,
		version: parts.version,
		href: label === self ? '' : packagePagePath(label),
	};
}

const own = (entry: Node) => Number(entry.id.startsWith('workspace:'));

/** Own code first, then alphabetically: a reader recognises their own app names. */
function ordered(labels: string[], self: string): Node[] {
	return labels
		.map((label) => node(label, self))
		.toSorted(
			(a, b) =>
				own(b) - own(a) || a.name.localeCompare(b.name) || a.version.localeCompare(b.version),
		);
}

export const load: PageServerLoad = ({ params, locals }) => {
	const found = packageForRoute(`${params.registry}/${params.package}`);
	if (!found) error(404, 'Package not found');

	const reverse = dependents(found.purl);

	return {
		// The one page of the licence surface kept out of the index. There are several hundred
		// of them and each is one row of a directory that is indexed; `follow` because the links
		// out of a package -- its repository, its licence, its dependents -- still count.
		robots: 'noindex, follow',
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
			nodes: path.map((label) => node(label, found.purl)),
		})),
		dependents: {
			direct: ordered(reverse.direct, found.purl),
			indirect: ordered(reverse.indirect, found.purl),
		},
		githubHref: URLS.external.github.web,
		locale: { code: locals.locale?.code ?? 'mw' },
	};
};
