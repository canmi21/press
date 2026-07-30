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
