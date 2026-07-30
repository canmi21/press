import { URLS } from '@canmi/urls';

/**
 * The minimal definition every site shares. Sites append their own rules rather than
 * restating this, so a change to the common policy reaches all of them at once.
 */
export const robotsTxtBase = [`# ${URLS.external.robotstxt}`, 'User-agent: *'] as const;

export type RobotsTxtOptions = {
	allow?: readonly string[];
	disallow?: readonly string[];
	sitemap?: string | readonly string[] | null;
};

export function robotsTxt(options: RobotsTxtOptions = {}): string {
	// Annotated as string[]: robotsTxtBase is `as const`, so spreading it without this infers
	// a tuple of literal types that nothing else can be pushed into.
	const lines: string[] = [...robotsTxtBase];

	for (const path of options.allow ?? []) {
		lines.push(`Allow: ${path}`);
	}

	for (const path of options.disallow ?? []) {
		lines.push(`Disallow: ${path}`);
	}

	const sitemaps = toList(options.sitemap);
	if (sitemaps.length > 0) {
		lines.push('');
		for (const sitemap of sitemaps) {
			lines.push(`Sitemap: ${sitemap}`);
		}
	}

	return `${lines.join('\n')}\n`;
}

// Narrow on `typeof value === 'string'` rather than Array.isArray: Array.isArray narrows to
// the mutable `any[]`, which leaves a `readonly string[]` sitting in the false branch.
function toList(value: string | readonly string[] | null | undefined): readonly string[] {
	if (!value) return [];
	return typeof value === 'string' ? [value] : value;
}
