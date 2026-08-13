import tailwindcss from '@tailwindcss/vite';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { parse as parseYaml } from 'yaml';

const SITE_CONFIG = fileURLToPath(new URL('../site/site.config.yaml', import.meta.url));

function siteTitle(): string {
	const config = parseYaml(readFileSync(SITE_CONFIG, 'utf8')) as { name?: unknown };
	if (typeof config.name !== 'string' || config.name.length === 0) {
		throw new Error('site.config.yaml must define a non-empty name');
	}
	return config.name;
}

const port = Number(process.env.CMS_PORT);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
	throw new Error('CMS_PORT must be an integer between 1 and 65535');
}

export default defineConfig({
	clearScreen: false,
	plugins: [
		tailwindcss(),
		{
			name: 'site-title',
			configureServer(server) {
				server.watcher.add(SITE_CONFIG);
				server.watcher.on('change', (path) => {
					if (path === SITE_CONFIG) server.ws.send({ type: 'full-reload' });
				});
			},
			transformIndexHtml(html) {
				return html.replaceAll('%SITE_TITLE%', siteTitle());
			},
		},
	],
	server: {
		host: '127.0.0.1',
		port,
		strictPort: true,
	},
});
