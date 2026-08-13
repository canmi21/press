import { readFile } from 'node:fs/promises';

// Node's native TypeScript loader requires the source suffix, while the repository's bundler
// module resolution rejects it in static imports.
const { createMarkdownNormalizer } = (await import(
	new URL('../client/markdown.ts', import.meta.url).href
)) as typeof import('../client/markdown');

const path = process.argv[2];
if (!path) throw new Error('usage: node normalize-markdown.ts ARTICLE');

const normalise = await createMarkdownNormalizer();
const source = await readFile(path, 'utf8');
process.stdout.write(normalise(source).markdown);
