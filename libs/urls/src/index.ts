export const URLS = {
	development: {
		web: 'http://localhost:26511',
		api: 'http://localhost:26512',
		res: 'http://localhost:26516',
		home: 'http://localhost:26518',
		app: 'https://canmi.app',
		dev: 'https://canmi.dev',
		prod: 'https://ffoni.com'
	},
	production: {
		web: 'https://canmi.net',
		api: 'https://api.ffoni.com',
		res: 'https://cdn.ffoni.com',
		home: 'https://ill.li',
		app: 'https://canmi.app',
		dev: 'https://canmi.dev',
		prod: 'https://ffoni.com'
	}
} as const;

export type AppName = keyof typeof URLS.development;
export type UrlMap = {
	readonly web: string;
	readonly api: string;
	readonly res: string;
	readonly home: string;
	readonly app: string;
	readonly dev: string;
	readonly prod: string;
};

export function pickUrls(isDev: boolean): UrlMap {
	return isDev ? URLS.development : URLS.production;
}

export function isDevHost(hostname: string): boolean {
	return hostname === 'localhost' || hostname === '127.0.0.1';
}

export const robotsTxt = `# https://www.robotstxt.org/robotstxt.html
User-agent: *
Disallow: /@/
Disallow: /cgi-bin/
Disallow: /cdn-cgi/

Sitemap: ${URLS.production.web}/sitemap.xml
`;
