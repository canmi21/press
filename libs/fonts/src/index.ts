import manifest from '../../../data/fonts.json' with { type: 'json' };

export interface FontFamily {
	readonly id: string;
	readonly displayName: string;
	readonly stylesheet: string;
	readonly stack: readonly string[];
}

export const fontFamilies: readonly FontFamily[] = manifest.families.map((family) => ({
	id: family.id,
	displayName: family.family,
	stylesheet: family.stylesheetExport,
	stack: [family.family, family.generic],
}));
