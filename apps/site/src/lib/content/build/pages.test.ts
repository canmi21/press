import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { homepageContent } from '../../home/content';
import { buildPages } from './articles';

const ROOT = new URL('../../../../../../', import.meta.url);

describe('standalone page locale views', () => {
	it('keeps the bio in English while localising the writing heading', async () => {
		const { pages } = await buildPages({
			contents: fileURLToPath(new URL('contents', ROOT)),
			segments: fileURLToPath(new URL('data/build/segments.json', ROOT)),
		});
		const homepage = pages.find((page) => page.path === 'homepage');
		expect(homepage).toBeDefined();
		if (!homepage) throw new Error('missing homepage');

		const japanese = homepageContent(homepage, 'ja');
		expect(JSON.stringify(japanese.bio)).toContain('I build things.');
		expect(JSON.stringify(japanese.bio)).not.toContain('私はものを作ります。');
		expect(japanese.writing).toBe('記事');
	});
});
