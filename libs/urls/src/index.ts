/**
 * The ports each app answers on in development.
 *
 * Pinned, and bound by exactly one checkout at a time. The gaps are the inspector ports, which
 * wrangler takes as port + 1, and they keep clear of CMS_PORT (mise.toml). A second copy of an
 * app collides here rather than drifting to a free port, which is the cheapest mutex there is.
 * See spec/toolchain.md.
 */
export const DEVELOPMENT_PORTS = { site: 26511, api: 26512, cdn: 26516 } as const;

export type AppName = keyof typeof DEVELOPMENT_PORTS;
export type DevelopmentUrls = Readonly<Record<AppName, string>>;

/**
 * Where the API and the CDN are reached *from a page* in development: through the site.
 *
 * The site's dev server proxies these two prefixes to the two workers, so a page carries no host
 * of its own for them. That is what lets a phone on the same network open the site by its LAN
 * address and have every request go back to the address it was loaded from -- `localhost` on that
 * phone is the phone. It also covers what the browser never asks for directly: fonts, avatars and
 * the OpenGraph card are rendered into the HTML by the worker, so an absolute `localhost` there is
 * already wrong before any script runs.
 *
 * Production has three domains and no proxy. Only development collapses them, and only because
 * in development they are three processes on one machine. See spec/toolchain.md.
 */
export const DEVELOPMENT_PROXY_PATHS = { api: '/api', cdn: '/cdn' } as const;

export function developmentUrl(app: AppName): string {
	return `http://localhost:${DEVELOPMENT_PORTS[app]}`;
}

/** Every app's address in development. */
export function developmentUrls(): DevelopmentUrls {
	return {
		site: developmentUrl('site'),
		api: developmentUrl('api'),
		cdn: developmentUrl('cdn'),
	};
}

const development: DevelopmentUrls = developmentUrls();

/**
 * The GitHub account everything here is published under.
 *
 * An identity rather than an address, which is why it sits beside the map instead of inside it:
 * the map is mirrored into Rust as URL constants, and this is the one string in it that is not
 * a URL. The CDN's release proxy fetches from this account and no other -- there is no place in
 * its URL to name a different one. See spec/architecture/delivery.md.
 */
export const GITHUB_OWNER = 'canmi21';

export const URLS = {
	apps: {
		development,
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
	source: `https://github.com/${GITHUB_OWNER}/press`,
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
			api: 'https://api.github.com',
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
			// The sparse index the embed collector reads crate metadata from.
			cargoIndex: 'https://index.crates.io',
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
		// Named as the feed's generator. Nothing fetches it, but it is emitted into published
		// output, so it belongs with the other URLs rather than inline in a route.
		feedsmith: 'https://feedsmith.dev',
		// Where changed URLs are announced. The shared endpoint rather than one engine's own:
		// participants agree to forward what they receive, so submitting here reaches all of
		// them and picking one would be choosing which of them to tell. See spec/indexing.md.
		indexnow: 'https://api.indexnow.org/IndexNow',
		// Bases for social profile links. Handles stay in `site.config.yaml`; these are only
		// where a handle is reachable.
		//
		// `twitter.com` rather than `x.com`, on both. The service renamed itself and kept the
		// old host as a permanent redirect, which it will go on keeping -- too much of the web
		// points at it to drop. So the choice is between a name its owner picked and the name
		// everybody uses, at the cost of one redirect nobody waits on. See spec/twitter.md.
		social: {
			telegram: 'https://t.me',
			twitter: 'https://twitter.com',
			twitterIntent: 'https://twitter.com/intent/follow',
			fediverse: 'https://nya.one',
			bluesky: 'https://bsky.app/profile',
		},
		// Companion sites the cargo widget links a crate to, beside the registry above. Keyed by
		// what each serves, joined with `/{crate}` (docs) and `/crates/{crate}` (lib).
		rust: {
			docs: 'https://docs.rs',
			lib: 'https://lib.rs',
		},
		// Webring gateways the homepage footer links into. Whole navigation URLs rather than
		// bases: the path and query are the gateway's interface, not something assembled here.
		webring: {
			travellings: 'https://www.travellings.cn/go.html',
			moe: 'https://travel.moe/go?travel=on',
		},
		// Registration directory behind the homepage badge, joined with `?keyword={id}`.
		icpmoe: 'https://icp.gov.moe',
		// Analytics loader fetched by the browser. The website id rides on the script tag: it is
		// an identity, not an address.
		umami: 'https://cloud.umami.is/script.js',
		// Where the two analytics clients report to, which is not where either comes from.
		// Neither host is written in this repository's own code -- umami's is a constant inside
		// the script it downloads, and OpenPanel's is the default baked into `@openpanel/sdk` --
		// so both are recorded here from having been read out of them. They are declared to be
		// resolved early rather than to be fetched. See spec/analytics.md.
		umamiGateway: 'https://gateway.umami.is',
		openpanel: 'https://api.openpanel.dev',
		// Hosts the Latin webfont stylesheet resolves through; preconnected before it is fetched.
		googleFonts: {
			css: 'https://fonts.googleapis.com',
			static: 'https://fonts.gstatic.com',
		},
	},
} as const;

export type UrlEnvironment = keyof typeof URLS.apps;
export type UrlMap = (typeof URLS.apps)[UrlEnvironment];

export function pickUrls(isDev: boolean): UrlMap {
	return isDev ? URLS.apps.development : URLS.apps.production;
}

/**
 * The same map as `pickUrls`, as a page served by the site should ask for it.
 *
 * Two consumers want opposite things from the development entry, which is why there are two
 * functions. A worker wants origins: the API's CORS list names the site, and its redirects to the
 * site and the CDN have to be addresses somebody can follow. A page wants paths, because the host
 * it should ask is whichever one it was opened from -- `localhost` is only right when that host
 * is this machine, and the whole point of the proxy is that it need not be.
 *
 * Identical to `pickUrls` in production, where the three are three domains and nothing is
 * proxied. See spec/toolchain.md.
 */
export function pageUrls(isDev: boolean): UrlMap {
	return isDev ? { ...URLS.apps.development, ...DEVELOPMENT_PROXY_PATHS } : URLS.apps.production;
}

/**
 * The address the CMS dev server binds to, and is therefore reached at.
 *
 * The one place here a literal is right, and it is right because of what it binds rather than in
 * spite of it. Local development is `localhost` (spec/toolchain.md), but a listen address is a
 * separate question: this server binds a single address on purpose, so that a desktop app's dev
 * server is not on the network, and Node binds exactly one address when given a name. `localhost`
 * resolves to `::1` first on macOS, which nothing is listening on -- so the URL has to name the
 * address that was bound rather than a name that can resolve past it.
 *
 * A bare hostname rather than a URL, because the two consumers want different shapes: a Vite
 * `server.host` takes the host alone, while the Tauri dev URL wants an origin from `loopbackUrl`.
 */
export const LOOPBACK_HOST = '127.0.0.1';

/**
 * The hostnames that mean this machine.
 *
 * `[::1]` in brackets, which is the form every caller here has: `URL.hostname` normalises each
 * spelling of the IPv6 loopback to that one, and a `Host` header brackets it too. The bare form
 * sits beside it for a caller that takes a host apart itself. Without them a request that arrived
 * over IPv6 -- which it can, since the site binds `::` -- was read as production and answered
 * with production addresses.
 *
 * The IPv4 literal is `LOOPBACK_HOST` because it is the same address, not because recognising a
 * host and binding one are the same job.
 */
const DEV_HOSTS: ReadonlySet<string> = new Set(['localhost', LOOPBACK_HOST, '[::1]', '::1']);

export function isDevHost(hostname: string): boolean {
	return DEV_HOSTS.has(hostname);
}

export function loopbackUrl(port: number): string {
	const url = new URL(`http://${LOOPBACK_HOST}`);
	url.port = String(port);
	return url.origin;
}
