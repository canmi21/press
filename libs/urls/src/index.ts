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
	// Where everything here that is not a dependency comes from. Named at the top of the
	// licence routes, which have to state the terms of the code around the credits as well as
	// the credits themselves -- so it is a published fact, not a convenience, and belongs
	// beside the other URLs rather than written into a route.
	source: 'https://github.com/canmi21/workspace',
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
		google: {
			sourcePreferences: 'https://www.google.com/preferences/source',
		},
		// Where the dependencies come from, named on the licence page. Keyed by purl type, which
		// is what the record uses, so the page looks a registry up rather than mapping names.
		registries: {
			npm: 'https://www.npmjs.com',
			cargo: 'https://crates.io',
		},
		// The canonical page for a licence, joined with `/{id}.html`. SPDX rather than any of the
		// stewards' own sites, because the whole licence record is keyed by SPDX identifier and
		// this is the one address that exists for every one of them.
		spdx: 'https://spdx.org/licenses',
		robotstxt: 'https://www.robotstxt.org/robotstxt.html',
		// A Sentry DSN only permits *sending* events to one project -- it grants no read
		// access -- and the browser SDK compiles it into the bundle, where anyone can read it
		// out of devtools. It is therefore public by construction, and declaring it here is
		// honest about that rather than pretending a secret store could hide it.
		//
		// The API worker's DSN is a different project that never reaches a browser, so it
		// stays a wrangler secret. Each is treated according to whether it is exposed.
		sentry: {
			site: 'https://a7f2f790ed2fa4f8e0c4310d26d9c39f@o4511131162116096.ingest.us.sentry.io/4511380121976832',
		},
		// Analytics beacon, loaded by the browser from the page.
		insights: 'https://static.cloudflareinsights.com/beacon.min.js',
		// Named as the feed's generator. Nothing fetches it, but it is emitted into published
		// output, so it belongs with the other URLs rather than inline in a route.
		feedsmith: 'https://feedsmith.dev',
		// Bases for social profile links. `x` and `twitter` are the same
		// service under two hostnames: profiles moved and the intent endpoint did not, so both
		// are still live and both are still needed.
		social: {
			telegram: 'https://t.me',
			x: 'https://x.com',
			twitterIntent: 'https://twitter.com/intent/follow',
		},
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
