import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const port = Number(process.env.CMS_PORT);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
	throw new Error('CMS_PORT must be an integer between 1 and 65535');
}

export default defineConfig({
	clearScreen: false,
	plugins: [tailwindcss()],
	server: {
		host: '127.0.0.1',
		port,
		strictPort: true,
	},
});
