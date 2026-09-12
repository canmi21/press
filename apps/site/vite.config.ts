import { readFileSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import { isAbsolute, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DEVELOPMENT_PORTS, DEVELOPMENT_PROXY_PATHS, developmentUrl, pageUrls } from '@canmi/urls';
import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import Icons from 'unplugin-icons/vite';
import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';
import { defineConfig, type UserConfig, type ViteDevServer } from 'vite';
import { parse as parseYaml } from 'yaml';
import { buildArticles, buildPages } from './src/lib/content/build/articles.ts';
import type { Article, Page } from './src/lib/content/types.ts';
import { packArticles, packPages } from './src/lib/content/packed.ts';
import { contentRefreshQueue } from './vite/content-refresh.ts';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const SITE_CONFIG = fileURLToPath(new URL('./site.config.yaml', import.meta.url));
const CONTENTS = fileURLToPath(new URL('../../contents', import.meta.url));
const ASSETS = fileURLToPath(new URL('../../data/metadata.json', import.meta.url));
const MEDIA = fileURLToPath(new URL('../../data/media.yaml', import.meta.url));
const DIAGRAMS = fileURLToPath(new URL('../../data/diagram.json', import.meta.url));
const SEGMENTS = fileURLToPath(new URL('../../data/build/segments.json', import.meta.url));
const MESSAGES = fileURLToPath(new URL('./messages', import.meta.url));
const CRATES = fileURLToPath(new URL('../../data/build/crates.json', import.meta.url));
const REPOS = fileURLToPath(new URL('../../data/build/repos.json', import.meta.url));
const TWEETS = fileURLToPath(new URL('../../data/build/twitter.json', import.meta.url));
const LICENSES = fileURLToPath(new URL('../../data/build/licenses.json', import.meta.url));
const execFileAsync = promisify(execFile);

function articleMarkdown(path: string): boolean {
	const fromContents = relative(CONTENTS, path);
	return !fromContents.startsWith('..') && !isAbsolute(fromContents) && path.endsWith('.md');
}

function messageCatalog(path: string): boolean {
	const fromMessages = relative(MESSAGES, path);
	return !fromMessages.startsWith('..') && !isAbsolute(fromMessages) && path.endsWith('.json');
}

// Built-in 301s, kept out of site.config.yaml because they are product behaviour rather than
// configuration: feed aliases and the favicon redirect to the CDN.
function builtinRedirects(cdnUrl: string): Record<string, string> {
	return {
		'/rss': '/atom.xml',
		'/rss.xml': '/atom.xml',
		'/feed': '/atom.xml',
		'/feed.xml': '/atom.xml',
		'/favicon.ico': `${cdnUrl}/favicon.ico`,
	};
}

// TODO: nothing reads this yet. It is kept, not deleted, because the footer that shows the
// deployed commit is planned rather than abandoned -- and the value has to be captured at build
// time, which is a thing this file can do and a component cannot.
//
// execFileSync takes no shell, so there is no injection surface. jj is colocated with git, which
// is why asking git still works.
const commitHash = (() => {
	try {
		return execFileSync('git', ['rev-parse', '--short', 'HEAD']).toString().trim();
	} catch {
		return 'unknown';
	}
})();

// Sitemap <lastmod> for routes like "/" that have no article of their own to date from.
const buildTime = new Date().toISOString();

// The syntax floor, and the only place it is written down. `browserslist` in package.json says
// which browsers the emitted JavaScript has to parse on, and esbuild compiles down to it here.
//
// It is set to the line the compatibility canary rescues to, and that is not a coincidence:
// `compatibility.ts` loads core-js for a browser without `Array.prototype.toSorted`, which is
// Chrome 110, Firefox 115 and Safari 16.0. A rescue only happens if the browser could parse the
// code doing the rescuing, so a target above that line would hand those readers a bundle that
// dies before the check runs. The two floors agree by construction. See spec/compat.md.
//
// Stated rather than left to Vite's default, which is a baseline of somebody else's choosing and
// moves under a major -- it was chrome111, edge111, firefox114, safari16.4 when this was written.
//
// Floors, never a relative query like `> 0.5%`. A relative query is resolved against
// caniuse-lite, so the compiled output would change on an unrelated dependency update and a
// rebuild of the same commit would not be the same bytes.
const BROWSERSLIST: string[] = JSON.parse(
	readFileSync(fileURLToPath(new URL('./package.json', import.meta.url)), 'utf8'),
).browserslist;

// esbuild wants `chrome110`; browserslist writes `chrome >= 110`. Same fact, two spellings,
// and this is the whole distance between them.
const esbuildTarget = BROWSERSLIST.map((query) => {
	const floor = /^(\S+)\s*>=\s*(\S+)$/.exec(query);
	if (!floor)
		throw new Error(`browserslist entry is not a floor, so esbuild cannot take it: ${query}`);
	return `${floor[1]}${floor[2]}`;
});

/**
 * Whether this build sends its source maps to Sentry.
 *
 * Two conditions, not one. The credential has to be there, and the skip must not be set --
 * because locally it *is* there. mise decrypts it out of `secrets.json` on entering the
 * directory, so a local production build was uploading maps for a worker nobody deploys.
 *
 * The skip lives in `mise.toml`, which is the point: CI does not read that file, so it is on
 * every machine that has the repository and on none that builds it for real. Neither side
 * configures anything to get the behaviour it wants -- see spec/architecture/workspace.md.
 *
 * **The answer drives `autoUploadSourceMaps`, not just the token.** Withholding the credential
 * is not enough: the plugin reads `SENTRY_AUTH_TOKEN` from the environment itself when the
 * option is undefined, so a build that passed it nothing still uploaded. That was measured, not
 * assumed.
 *
 * In CI a missing credential is still fatal. That build is going to be deployed, and skipping
 * the upload silently means every stack trace it ever produces is minified, discovered weeks
 * later while trying to read an error that no longer maps to any source. The check sits after
 * the skip so that setting both is a deliberate quiet build rather than a contradiction that
 * throws.
 */
function uploadsSourceMaps(): boolean {
	// Any non-empty value enables it, so `SENTRY_SKIP_UPLOAD= pnpm run build` is how one local
	// build uploads after all. A value of `0` or `false` still skips: this is a switch, and
	// reading words out of it would only invite the belief that it parses them.
	if (process.env.SENTRY_SKIP_UPLOAD) return false;

	const token = process.env.SENTRY_AUTH_TOKEN;
	if (!token && process.env.CI) {
		throw new Error(
			'SENTRY_AUTH_TOKEN is unset in CI. Add it as an encrypted build variable, or the ' +
				'deployed worker will report every error without a usable stack trace.',
		);
	}
	return Boolean(token);
}

export default defineConfig(async ({ command, mode }) => {
	// The page-facing map, because all three readers of it below end up in a document: the asset
	// URLs compiled into the corpus, the redirect targets a browser follows, and the font
	// stylesheet's `__CDN_URL__`. In development those must be the proxied paths, or a page opened
	// from another device asks that device for its own fonts. See libs/urls.
	const urls = pageUrls(mode !== 'production');
	// Asked once. It can throw, and a predicate that throws should do so at a point in the build
	// somebody can place, rather than from inside a plugin's option list.
	const uploadSourceMaps = uploadsSourceMaps();
	const articleInputs = new Set<string>();
	let generatedSegmentsMtime: number | undefined;
	let activeSegmentSync: Promise<void> | undefined;
	let devServer: ViteDevServer | undefined;
	const syncSegments = async (): Promise<void> => {
		const before = await stat(SEGMENTS).then(
			({ mtimeMs }) => mtimeMs,
			() => undefined,
		);
		const running = execFileAsync('cargo', ['run', '-q', '-p', 'cms', '--', 'segments'], {
			cwd: ROOT,
		}).then(async () => {
			const after = await stat(SEGMENTS).then(({ mtimeMs }) => mtimeMs);
			if (after !== before) generatedSegmentsMtime = after;
		});
		activeSegmentSync = running;
		try {
			await running;
		} finally {
			if (activeSegmentSync === running) activeSegmentSync = undefined;
		}
	};
	if (command === 'serve') await syncSegments();

	const compileContent = async () => {
		const [articleBuild, pageBuild] = await Promise.all([
			buildArticles(
				{
					contents: CONTENTS,
					cdnUrl: urls.cdn,
					messages: MESSAGES,
					assets: ASSETS,
					media: MEDIA,
					diagrams: DIAGRAMS,
					segments: SEGMENTS,
					crates: CRATES,
					repos: REPOS,
					tweets: TWEETS,
				},
				// The same discriminator the URLs above are picked by, for the same reason: what
				// this build is for. `vite build --mode development` therefore keeps drafts, which
				// is the one way to see one inside a real build.
				{ drafts: mode !== 'production' },
			),
			buildPages({ contents: CONTENTS, messages: MESSAGES, segments: SEGMENTS }),
		]);
		articleInputs.clear();
		for (const file of new Set([...articleBuild.files, ...pageBuild.files])) {
			articleInputs.add(file);
		}
		return { articleBuild, pageBuild };
	};

	type RuntimeArticles = {
		replaceContent: (articles: Article[], pages: Page[]) => void;
	};
	const refreshContent = contentRefreshQueue(async (segments) => {
		if (segments) await syncSegments();
		const { articleBuild, pageBuild } = await compileContent();
		if (!devServer) return;
		const runtime = (await devServer.ssrLoadModule('virtual:articles')) as RuntimeArticles;
		runtime.replaceContent(articleBuild.articles, pageBuild.pages);
	});
	return {
		plugins: [
			tailwindcss(),
			// One strategy, no built-in fallback: locale negotiation stays in the worker and
			// Paraglide is told the answer. `url` is deliberately absent -- a locale never appears
			// in a path here, so there is nothing to delocalize and no `reroute` hook.
			// See spec/locale.md.
			paraglideVitePlugin({
				// The SDK refuses any project path not ending in `.inlang`, so the whole name is
				// the suffix. See spec/locale.md.
				project: './.inlang',
				outdir: './src/lib/paraglide',
				strategy: ['custom-negotiated'],
			}),
			// Iconify sets compiled to Svelte components at build time, so a set contributes only
			// the icons actually imported rather than a runtime font or sprite sheet.
			Icons({ compiler: 'svelte' }),
			{
				// Content sources and sidecars are build inputs, not Worker work. Compile every
				// browser-facing view here and serialize the lookup tables into the server bundle.
				// Development replaces one stable runtime snapshot instead. See spec/i18n.md.
				name: 'virtual-articles',
				configureServer(server) {
					devServer = server;
					// Watch inputs directly without registering them as dependencies of the virtual
					// module. Vite invalidates dependencies before hotUpdate can replace the stable
					// snapshot, which would retain another full SSR generation. See spec/i18n.md.
					server.watcher.add([
						CONTENTS,
						MESSAGES,
						ASSETS,
						MEDIA,
						DIAGRAMS,
						SEGMENTS,
						CRATES,
						REPOS,
						TWEETS,
					]);
				},
				resolveId(id: string) {
					return id === 'virtual:articles' ? '\0virtual:articles' : null;
				},
				async load(id: string) {
					if (id !== '\0virtual:articles') return null;
					if (activeSegmentSync) await activeSegmentSync;
					const { articleBuild, pageBuild } = await compileContent();
					if (command === 'build') {
						for (const file of new Set([...articleBuild.files, ...pageBuild.files])) {
							this.addWatchFile(file);
						}
					}
					return [
						`import { unpackArticles, unpackPages } from '$lib/content/packed.ts';`,
						`import { contentSnapshot } from '$lib/content/snapshot.ts';`,
						`const articles = unpackArticles(${JSON.stringify(packArticles(articleBuild.articles))});`,
						`const pages = unpackPages(${JSON.stringify(packPages(pageBuild.pages))});`,
						`export let content = contentSnapshot(articles, pages);`,
						`export function replaceContent(articles, pages) { content = contentSnapshot(articles, pages); }`,
					].join('\n');
				},
				async hotUpdate(options) {
					const markdown = articleMarkdown(options.file);
					if (options.file === SEGMENTS) {
						const pending = refreshContent.active();
						if (pending) await pending;
						const mtime = await stat(SEGMENTS).then(({ mtimeMs }) => mtimeMs);
						// The Markdown event already owns this refresh. Swallow its derived write.
						if (mtime === generatedSegmentsMtime) return [];
					} else if (
						!markdown &&
						!messageCatalog(options.file) &&
						!articleInputs.has(options.file)
					) {
						return;
					}
					// The browser has no content module to update. Its reload is sent only after the
					// server has atomically replaced the current snapshot.
					if (this.environment.name === 'client') return [];
					if (this.environment.name !== 'ssr') return;
					const request = refreshContent.request(markdown);
					await request.settled;
					if (request.leader) {
						options.server.environments.client.hot.send({
							type: 'full-reload',
							path: '*',
							triggeredBy: options.file,
						});
					}
					return [];
				},
			},
			sentrySvelteKit({
				org: 'canmi',
				project: 'canmi',
				autoUploadSourceMaps: uploadSourceMaps,
				authToken: uploadSourceMaps ? process.env.SENTRY_AUTH_TOKEN : undefined,
				telemetry: false,
				// Maps are uploaded to Sentry and then deleted, so the deployed worker carries
				// none. Paired with `sourcemap: 'hidden'` below, which emits them without the
				// `sourceMappingURL` comment, nothing in the browser goes looking for a file
				// that is not there. A build that does not upload emits none at all, so this
				// list has nothing to match and nothing is left behind either way.
				sourcemaps: {
					filesToDeleteAfterUpload: ['.svelte-kit/cloudflare/**/*.map'],
				},
			}),
			sveltekit(),
			{
				// The merged redirect map, baked into a virtual module. The prerendered
				// [...path] route emits redirect() responses that each adapter translates to
				// its own format, so none of this is tied to Cloudflare. Server-only.
				name: 'virtual-redirects',
				resolveId(id: string) {
					return id === 'virtual:redirects' ? '\0virtual:redirects' : null;
				},
				load(id: string) {
					if (id !== '\0virtual:redirects') return null;
					this.addWatchFile(SITE_CONFIG);
					const { redirects = {} } = parseYaml(readFileSync(SITE_CONFIG, 'utf8')) as {
						redirects?: Record<string, string>;
					};
					const map = { ...builtinRedirects(urls.cdn), ...redirects };
					return `export const redirects = ${JSON.stringify(map)};`;
				},
			},
			{
				// The font stylesheets in @canmi/fonts carry a placeholder rather than a
				// host, because which CDN answers depends on the mode and a library cannot
				// know that. See spec/architecture/workspace.md on where URLs are declared.
				name: 'replace-cdn-url',
				transform(code: string, id: string) {
					if (/\.css($|\?)/.test(id) && code.includes('__CDN_URL__')) {
						return code.replaceAll('__CDN_URL__', urls.cdn);
					}
					return null;
				},
			},
			{
				// The dependency licence record, baked in. Only the metadata travels: the texts
				// themselves are published objects the CDN serves, so the Worker carries a few
				// hundred KB of names and ids rather than several megabytes of legal prose.
				name: 'virtual-licenses',
				resolveId(id: string) {
					return id === 'virtual:licenses' ? '\0virtual:licenses' : null;
				},
				load(id: string) {
					if (id !== '\0virtual:licenses') return null;
					this.addWatchFile(LICENSES);
					return `export const licenses = ${readFileSync(LICENSES, 'utf8')};`;
				},
			},
			{
				// site.config.yaml baked into the bundle, which keeps the YAML parser out of
				// the client and the file out of the deployed worker.
				name: 'virtual-site-config',
				resolveId(id: string) {
					return id === 'virtual:site' ? '\0virtual:site' : null;
				},
				load(id: string) {
					if (id !== '\0virtual:site') return null;
					this.addWatchFile(SITE_CONFIG);
					const { redirects: _redirects, ...data } = parseYaml(readFileSync(SITE_CONFIG, 'utf8'));
					return `export const site = ${JSON.stringify(data)};`;
				},
			},
		],
		server: {
			// Pinned, never auto-incremented; see spec/toolchain.md. The number itself lives in
			// the URL map, so moving the dev server stays a one-file edit.
			port: DEVELOPMENT_PORTS.site,
			strictPort: true,
			// Every interface, so a phone on the same network can open this. `::` rather than
			// `0.0.0.0` because Node leaves IPV6_V6ONLY off, so one value covers both stacks and
			// the loopback addresses inside them -- `0.0.0.0` alone would drop `[::1]`, which is
			// what `localhost` resolves to first on this machine.
			host: '::',
			// The other two workers, reached through this one. The prefix is stripped on the way
			// out, so each worker sees the paths it actually serves and needs no knowledge of
			// this. Both the prefix and the target come from libs/urls, which is where every
			// address in this repository is declared -- and where the reasoning lives for why
			// development collapses three origins into one and production does not.
			//
			// The target is the same address anything else would use to reach these two, so it is
			// the same function. It was its own, resolving to `127.0.0.1` on the grounds that one
			// hop should stay on one stack; the hop never varied by the family a request arrived
			// on, and both workers bind both stacks.
			proxy: Object.fromEntries(
				Object.entries(DEVELOPMENT_PROXY_PATHS).map(([app, prefix]) => [
					prefix,
					{
						target: developmentUrl(app as 'api' | 'cdn'),
						changeOrigin: true,
						rewrite: (path: string) => path.slice(prefix.length),
					},
				]),
			),
		},
		ssr: {
			// Bits UI publishes Svelte source. Leaving it external in dev hands its `.svelte`
			// imports to Node through Sentry's loader, which cannot transform them and turns every
			// article request into an otherwise silent 500. Production bundles it already; make the
			// development SSR path cross the same compilation boundary.
			noExternal: ['bits-ui', '@inlang/paraglide-js-svelte'],
		},
		build: {
			// Stated rather than left to Vite's default, which is a baseline of its own choosing
			// and can move under a major. See BROWSERSLIST above.
			target: esbuildTarget,
			// Only when they are going somewhere. `filesToDeleteAfterUpload` below cleans them up
			// after an upload and cannot clean up after a build that did not do one, so a build
			// that skips would otherwise leave 117 maps in the directory wrangler deploys -- the
			// site's own source, served as static assets. Not emitting them is the shorter answer
			// than emitting and sweeping, and on this machine they were never going to be read.
			sourcemap: uploadSourceMaps ? 'hidden' : false,
			rollupOptions: {
				output: {
					hashCharacters: 'hex',
				},
			},
		},
		// URLs are imported from @canmi/urls at their use sites rather than injected here, so
		// there is one spelling of each. What is left is the pair of values that genuinely
		// only exist at build time.
		define: {
			'import.meta.env.VITE_COMMIT_HASH': JSON.stringify(commitHash),
			'import.meta.env.VITE_BUILD_TIME': JSON.stringify(buildTime),
		},
		// The function is async for the probe above, and a promise loses the contextual typing
		// that kept 'hidden' and 'hex' literal; this puts it back.
	} satisfies UserConfig;
});
