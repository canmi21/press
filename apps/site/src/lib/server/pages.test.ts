import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { buildPages } from './articles';

const ROOT = new URL('../../../../../', import.meta.url);

describe('standalone page locale views', () => {
	it('compiles the paid homepage translation for request-time selection', async () => {
		const { pages } = await buildPages({
			contents: fileURLToPath(new URL('contents', ROOT)),
			segments: fileURLToPath(new URL('data/build/segments.json', ROOT)),
		});
		const homepage = pages.find((page) => page.path === 'homepage');
		expect(homepage).toBeDefined();

		const source = JSON.stringify(homepage?.views.mw.blocks);
		const japanese = JSON.stringify(homepage?.views.ja.blocks);
		expect(source).toContain('I build things.');
		expect(japanese).toContain('私はものを作ります。');
		expect(japanese).not.toContain('I build things.');
	});
});
