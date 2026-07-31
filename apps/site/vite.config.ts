import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { URLS } from '@canmi/urls';
import { sentrySvelteKit } from '@sentry/sveltekit';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { parse as parseYaml } from 'yaml';

const SITE_CONFIG = fileURLToPath(new URL('./site.config.yaml', import.meta.url));
const ASSETS = fileURLToPath(new URL('../../assets.json', import.meta.url));

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

// Baked in so the footer can show the deployed commit. execFileSync takes no shell, so there
// is no injection surface. jj is colocated with git, which is why this still works.
const commitHash = (() => {
	try {
		return execFileSync('git', ['rev-parse', '--short', 'HEAD']).toString().trim();
	} catch {
		return 'unknown';
	}
})();

// Sitemap <lastmod> for routes like "/" that have no article of their own to date from.
const buildTime = new Date().toISOString();

export default defineConfig(({ mode }) => {
	const urls = mode === 'production' ? URLS.apps.production : URLS.apps.development;
	return {
		plugins: [
			tailwindcss(),
			sentrySvelteKit({
				org: 'canmi',
				project: 'canmi',
				// Build-time credential, decrypted from secrets.json by mise. Absent, the
				// upload is skipped and the build still succeeds -- which is what a local
				// build without the key should do.
				authToken: process.env.SENTRY_AUTH_TOKEN,
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
				// The font stylesheet in @canmi/tokens carries a placeholder rather than a
				// host, because which CDN answers depends on the mode and a library cannot
				// know that. See spec/architecture.md on where URLs are declared.
				name: 'replace-cdn-url',
				transform(code: string, id: string) {
					if (/\.css($|\?)/.test(id) && code.includes('__CDN_URL__')) {
						return code.replaceAll('__CDN_URL__', urls.cdn);
					}
					return null;
				},
			},
			{
				// The asset manifest, baked in so an article can carry its own placeholders and
				// variant list. The images themselves are not in the repository, which is
				// exactly why this file is: a CI build has the manifest and needs nothing else.
				name: 'virtual-assets',
				resolveId(id: string) {
					return id === 'virtual:assets' ? '\0virtual:assets' : null;
				},
				load(id: string) {
					if (id !== '\0virtual:assets') return null;
					this.addWatchFile(ASSETS);
					return `export const assets = ${readFileSync(ASSETS, 'utf8')};`;
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
			// Pinned, never auto-incremented; see spec/toolchain.md.
			port: 26511,
			strictPort: true,
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
