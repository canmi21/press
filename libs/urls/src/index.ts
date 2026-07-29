export const URLS = {
	apps: {
		development: {
			site: 'http://localhost:26511',
			api: 'http://localhost:26512',
			cdn: 'http://localhost:26516',
		},
		production: {
			site: 'https://canmi.net',
			api: 'https://api.ffoni.com',
			cdn: 'https://cdn.ffoni.com',
		},
	},
	// Domains owned here but not built here. `infra` is the apex that api and cdn hang off;
	// `link` currently redirects to the site rather than serving content of its own.
	internal: {
		app: 'https://canmi.app',
		infra: 'https://ffoni.com',
		link: 'https://ill.li',
	},
	external: {
		github: {
			web: 'https://github.com',
			raw: 'https://raw.githubusercontent.com',
			avatars: 'https://avatars.githubusercontent.com',
			cdn: 'https://cdn.jsdelivr.net/gh',
		},
		robotstxt: 'https://www.robotstxt.org/robotstxt.html',
	},
} as const;

export type UrlEnvironment = keyof typeof URLS.apps;
export type AppName = keyof typeof URLS.apps.development;
export type UrlMap = (typeof URLS.apps)[UrlEnvironment];

export function pickUrls(isDev: boolean): UrlMap {
	return isDev ? URLS.apps.development : URLS.apps.production;
}

export function isDevHost(hostname: string): boolean {
	return hostname === 'localhost' || hostname === '127.0.0.1';
}

export const robotsTxtBase = [`# ${URLS.external.robotstxt}`, 'User-agent: *'] as const;

export type RobotsTxtOptions = {
	allow?: readonly string[];
	disallow?: readonly string[];
	sitemap?: string | readonly string[] | null;
};

export function robotsTxt(options: RobotsTxtOptions = {}): string {
	const lines = [...robotsTxtBase];

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

function toList(value: string | readonly string[] | null | undefined): readonly string[] {
	if (!value) return [];
	return Array.isArray(value) ? value : [value];
}
