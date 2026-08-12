import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { fontSplit } = require('cn-font-split/dist/node/index.js');

const [configPath] = process.argv.slice(2);
if (!configPath) {
	throw new Error('usage: font-split.mjs <config.json>');
}

const config = JSON.parse(await readFile(configPath, 'utf8'));
config.input = new Uint8Array(await readFile(config.input));

await fontSplit(config);

// cn-font-split has awaited every output write when its promise resolves, but its Koffi callback
// can leave Node hanging or abort during teardown. This child owns no work after the split.
process.exit(0);
