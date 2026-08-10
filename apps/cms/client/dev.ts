import { spawnSync } from 'node:child_process';
import { loopbackUrl } from '@canmi/urls';

const port = Number(process.env.CMS_PORT);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
	throw new Error('CMS_PORT must be an integer between 1 and 65535');
}

const config = JSON.stringify({
	app: {
		security: {
			capabilities: [
				{
					identifier: 'main-development',
					description: 'Expose the CMS development window to local MCP tooling.',
					windows: ['main'],
					permissions: ['core:window:allow-set-title', 'mcp-bridge:default'],
				},
			],
		},
		withGlobalTauri: true,
	},
	build: { devUrl: loopbackUrl(port) },
});
const executable = process.platform === 'win32' ? 'tauri.cmd' : 'tauri';
const result = spawnSync(
	executable,
	['dev', '--features', 'mcp', '--config', config, ...process.argv.slice(2)],
	{ stdio: 'inherit' },
);

if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
