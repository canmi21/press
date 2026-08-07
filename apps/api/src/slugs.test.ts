import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { articleSlugs, slugsModule } from '../scripts/slugs';
import { ARTICLE_SLUGS } from './slugs';

const MODULE = fileURLToPath(new URL('./slugs.ts', import.meta.url));

// The generated list is committed so the workspace type check, lint and test tasks work from
// a fresh clone without running this package's build. That trade is only safe while something
// notices it going stale, which is this.
it('matches the content tree', async () => {
	const slugs = await articleSlugs();
	expect([...ARTICLE_SLUGS]).toEqual(slugs);
	expect(await readFile(MODULE, 'utf8')).toBe(slugsModule(slugs));
});
