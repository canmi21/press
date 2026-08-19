import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import Icons from 'unplugin-icons/vite';
import { execFileSync } from 'node:child_process';
import { defineConfig } from 'vite';
import { parse as parseYaml } from 'yaml';
import { buildArticles, buildPages } from './src/lib/content/build/articles.ts';

const SITE_CONFIG = fileURLToPath(new URL('./site.config.yaml', import.meta.url));
const CONTENTS = fileURLToPath(new URL('../../contents', import.meta.url));
const ASSETS = fileURLToPath(new URL('../../data/metadata.json', import.meta.url));
const MEDIA = fileURLToPath(new URL('../../data/media.yaml', import.meta.url));
const SEGMENTS = fileURLToPath(new URL('../../data/build/segments.json', import.meta.url));
const CRATES = fileURLToPath(new URL('../../data/build/crates.json', import.meta.url));
const REPOS = fileURLToPath(new URL('../../data/build/repos.json', import.meta.url));
const LICENSES = fileURLToPath(new URL('../../data/build/licenses.json', import.meta.url));

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

/**
 * The Sentry upload credential, if this build is allowed to proceed without one.
 *
 * Locally it is decrypted from `secrets.json` by mise, and a build without it is fine: the
 * maps are of no use to anyone on this machine anyway.
 *
 * In CI its absence is fatal. That build is going to be deployed, and skipping the upload
 * silently means every stack trace it ever produces is minified -- discovered weeks later,
 * while trying to read an error that no longer maps to any source. CI has no age private key,
 * so `secrets.json` cannot supply it there; it comes from the platform's own encrypted build
 * variables instead. See spec/architecture/workspace.md.
 */
function sentryToken(): string | undefined {
	const token = process.env.SENTRY_AUTH_TOKEN;
	if (!token && process.env.CI) {
		throw new Error(
			'SENTRY_AUTH_TOKEN is unset in CI. Add it as an encrypted build variable, or the ' +
				'deployed worker will report every error without a usable stack trace.',
		);
	}
	return token;
}

export default defineConfig(({ mode }) => {
	const urls = mode === 'production' ? URLS.apps.production : URLS.apps.development;
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
				name: 'virtual-articles',
				resolveId(id: string) {
					return id === 'virtual:articles' ? '\0virtual:articles' : null;
				},
				async load(id: string) {
					if (id !== '\0virtual:articles') return null;
					const [articleBuild, pageBuild] = await Promise.all([
						buildArticles({
							contents: CONTENTS,
							assets: ASSETS,
							media: MEDIA,
							segments: SEGMENTS,
							crates: CRATES,
							repos: REPOS,
						}),
						buildPages({ contents: CONTENTS, segments: SEGMENTS }),
					]);
					for (const file of new Set([...articleBuild.files, ...pageBuild.files])) {
						this.addWatchFile(file);
					}
					return [
						`export const articles = ${JSON.stringify(articleBuild.articles)};`,
						`export const pages = ${JSON.stringify(pageBuild.pages)};`,
					].join('\n');
				},
			},
			sentrySvelteKit({
				org: 'canmi',
				project: 'canmi',
				authToken: sentryToken(),
				telemetry: false,
				// Maps are uploaded to Sentry and then deleted, so the deployed worker carries
				// none. Paired with `sourcemap: 'hidden'` below, which emits them without the
				// `sourceMappingURL` comment, nothing in the browser goes looking for a file
				// that is not there.
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
			port: Number(new URL(URLS.apps.development.site).port),
			strictPort: true,
		},
		ssr: {
			// Bits UI publishes Svelte source. Leaving it external in dev hands its `.svelte`
			// imports to Node through Sentry's loader, which cannot transform them and turns every
			// article request into an otherwise silent 500. Production bundles it already; make the
			// development SSR path cross the same compilation boundary.
			noExternal: ['bits-ui', '@inlang/paraglide-js-svelte'],
		},
		build: {
			sourcemap: 'hidden',
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
	};
});
