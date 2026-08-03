import type { CrateDep } from './content/types';

export const CRATE_PALETTE = [
	'#5B8DEF',
	'#E06C75',
	'#56B6C2',
	'#C678DD',
	'#D19A66',
	'#61AFEF',
	'#98C379',
	'#E5C07B',
	'#BE5046',
	'#7C6ede',
	'#3DA588',
	'#D4976C',
	'#6B93D6',
	'#C95B83',
	'#4DBFAD',
	'#B07BCC',
	'#D6956B',
	'#5E9FD1',
	'#C2704E',
	'#72B07B',
	'#9A8FCC',
	'#C4A057',
	'#6EAFC9',
	'#CC7A7A',
	'#4FAF8E',
] as const;

export const KIND_COLORS = {
	normal: '#3178c6',
	optional: '#7c6ede',
	dev: '#b0ada6',
	build: '#34D399',
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
