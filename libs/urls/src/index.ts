/**
 * The base ports each app answers on in development.
 *
 * Slot 0 -- the base workspace -- binds exactly these. An overlay workspace with slot n binds
 * each one shifted by n * SLOT_STRIDE, so every overlay's addresses follow from its number and
 * nothing is looked up or negotiated. The stride leaves room for each app's inspector port,
 * which wrangler takes as port + 1, and keeps CMS_PORT (mise.toml) clear of every slot: the CMS
 * is a machine-wide singleton and does not shift. See spec/toolchain.md.
 */
export const DEVELOPMENT_PORTS = { site: 26511, api: 26512, cdn: 26516 } as const;
export const SLOT_STRIDE = 100;

export type AppName = keyof typeof DEVELOPMENT_PORTS;
export type DevelopmentUrls = Readonly<Record<AppName, string>>;

export function slotPort(app: AppName, slot: number): number {
	if (!Number.isInteger(slot) || slot < 0) {
		throw new Error(`A workspace slot is a non-negative integer, got ${String(slot)}`);
	}
	return DEVELOPMENT_PORTS[app] + slot * SLOT_STRIDE;
}

/**
 * The slot named by a WORKSPACE_SLOT value. mise sets the variable: "0" in the base checkout,
 * each overlay's number in its own mise.local.toml. Unset or empty -- a CI build, a shell
 * without mise -- means the base. Callers pass the value in rather than this reading the
 * environment, so the library stays free of `process` and bundles for the browser unchanged.
 */
export function parseSlot(raw: string | undefined): number {
	if (raw === undefined || raw === '') return 0;
	const slot = Number(raw);
	if (!Number.isInteger(slot) || slot < 0) {
		throw new Error(`WORKSPACE_SLOT must be a non-negative integer, got ${JSON.stringify(raw)}`);
	}
	return slot;
}

export function developmentUrl(app: AppName, slot: number): string {
	return `http://localhost:${slotPort(app, slot)}`;
}

/** Every app's address when one workspace runs all of them, at the given slot. */
export function developmentUrls(slot: number): DevelopmentUrls {
	return {
		site: developmentUrl('site', slot),
		api: developmentUrl('api', slot),
		cdn: developmentUrl('cdn', slot),
	};
}

declare global {
	/**
	 * Where each app is reached from the workspace a bundle was built in: the workspace's own
	 * slot for the apps it runs itself, the base for the rest. Only a dev server defines it,
	 * having probed its slot ports at startup (apps/site/vite.config.ts). Under bare node, in
	 * vitest, and inside the workers it is absent and the base addresses stand -- which is also
	 * what the Rust mirror renders, and why `mise run urls` is stable across workspaces.
	 * See spec/toolchain.md.
	 */
	// oxlint-disable-next-line no-underscore-dangle -- the shape bundler-defined globals take
	const __DEV_URLS__: DevelopmentUrls | undefined;
}

// `typeof` first: the identifier is undeclared wherever no bundler defined it, and reading an
// undeclared identifier throws where `typeof` merely answers 'undefined'.
// oxlint-disable-next-line no-underscore-dangle -- see the declaration above
const development: DevelopmentUrls =
	typeof __DEV_URLS__ === 'undefined' ? developmentUrls(0) : __DEV_URLS__;

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
 * The address a dev server binds to and is reached on.
 *
 * A bare hostname rather than a URL, because the two consumers want different shapes: a Vite
 * `server.host` takes the host alone, while everything else wants an origin from `loopbackUrl`.
 * Exported so neither has to write the literal, which is how the same four numbers came to sit
 * in this file twice and in a Vite config besides.
 */
export const LOOPBACK_HOST = '127.0.0.1';

export function isDevHost(hostname: string): boolean {
	return hostname === 'localhost' || hostname === LOOPBACK_HOST;
}

/**
 * Whether an Origin header names a development host, whatever its port.
 *
 * The port is deliberately not checked: an overlay workspace's site answers on its slot port
 * and still has to reach the base API, so the API cannot list one development origin and call
 * the rest strangers. Anything that is not a URL is not a development origin.
 */
export function isDevOrigin(origin: string): boolean {
	try {
		return isDevHost(new URL(origin).hostname);
	} catch {
		return false;
	}
}

export function loopbackUrl(port: number): string {
	const url = new URL(`http://${LOOPBACK_HOST}`);
	url.port = String(port);
	return url.origin;
}
