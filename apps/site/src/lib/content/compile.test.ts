import { expect, it } from 'vitest';
import { compile, compilePage } from './compile';

it('rejects malformed source lang metadata with the article file named', async () => {
	const raw = '---\ntitle: Test\nlang: zh_CN\n---\n\nBody.\n';
	await expect(
		compile(raw, '/article', {
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/bad-language.md',
		}),
	).rejects.toThrow('contents/bad-language.md: invalid BCP-47 lang frontmatter "zh_CN"');
});

it('keeps an email link working when its visible label is translated', () => {
	const page = compilePage('---\ntitle: Test\n---\n\n:link[メール]{to=t@ffoni.com}\n');
	const paragraph = page.blocks[0];
	if (paragraph?.type !== 'p') throw new Error('expected a paragraph');

	expect(paragraph.segments).toContainEqual({
		type: 'link',
		icon: 'email',
		href: 'mailto:t@ffoni.com',
		label: 'メール',
		newTab: false,
	});
});
