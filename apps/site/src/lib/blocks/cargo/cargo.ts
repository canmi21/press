import type { CrateDep } from '../../content/types';

export const CRATE_PALETTE = [
	'var(--cargo-1)',
	'var(--cargo-2)',
	'var(--cargo-3)',
	'var(--cargo-4)',
	'var(--cargo-5)',
	'var(--cargo-6)',
	'var(--cargo-7)',
	'var(--cargo-8)',
	'var(--cargo-9)',
	'var(--cargo-10)',
	'var(--cargo-11)',
	'var(--cargo-12)',
	'var(--cargo-13)',
	'var(--cargo-14)',
	'var(--cargo-15)',
	'var(--cargo-16)',
	'var(--cargo-17)',
	'var(--cargo-18)',
	'var(--cargo-19)',
	'var(--cargo-20)',
	'var(--cargo-21)',
	'var(--cargo-22)',
	'var(--cargo-23)',
	'var(--cargo-24)',
	'var(--cargo-25)',
] as const;

export const KIND_COLORS = {
	normal: 'var(--cargo-kind-normal)',
	optional: 'var(--cargo-kind-optional)',
	dev: 'var(--cargo-kind-dev)',
	build: 'var(--cargo-kind-build)',
} as const;

export type DependencyItem = { dep: CrateDep; key: string };

/** Stable identities for repeated crate names and versions in a real Cargo graph. */
export function dependencyItems(deps: CrateDep[]): DependencyItem[] {
	const seen = new Map<string, number>();
	return deps.map((dep) => {
		const identity = [dep.name, dep.version, dep.kind, dep.target ?? '', dep.depth.toString()].join(
			'\0',
		);
		const occurrence = seen.get(identity) ?? 0;
		seen.set(identity, occurrence + 1);
		return { dep, key: `${identity}\0${occurrence}` };
	});
}

export function crateColors(deps: CrateDep[]): Map<string, string> {
	const colors = new Map<string, string>();
	for (const dep of deps) {
		if (!colors.has(dep.name)) {
			colors.set(dep.name, CRATE_PALETTE[colors.size % CRATE_PALETTE.length]!);
		}
	}
	return colors;
}

export function kindColor(dep: CrateDep): string {
	if (dep.optional) return KIND_COLORS.optional;
	return KIND_COLORS[dep.kind as keyof typeof KIND_COLORS] ?? KIND_COLORS.normal;
}

export function formatBytes(value: number): string {
	if (value >= 1_048_576) return `${(value / 1_048_576).toFixed(1).replace(/\.0$/, '')} MB`;
	if (value >= 1024) return `${(value / 1024).toFixed(1).replace(/\.0$/, '')} KB`;
	return `${value} B`;
}
