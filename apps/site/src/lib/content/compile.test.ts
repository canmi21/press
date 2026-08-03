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

it('renders a translator note as an explicit control instead of a native tooltip', async () => {
	const compiled = await compile(
		'---\ntitle: Test\nlang: en-US\n---\n\nA :tn[local phrase]{is="Its meaning needs context."}.\n',
		'/article',
		{
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/example.md',
		},
	);
	const prose = compiled.blocks[0];
	if (prose?.type !== 'prose') throw new Error('expected prose');

	expect(prose.html).toContain('<button type="button" class="tn-trigger"');
	expect(prose.html).toContain('data-tn-note="Its meaning needs context."');
	expect(prose.html).toContain('aria-controls="translator-note" aria-expanded="false"');
	expect(prose.html).toContain(
		'<svg class="tn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"',
	);
	expect(prose.html).toContain('<circle cx="12" cy="12" r="10"></circle>');
	expect(prose.html).toContain('<path d="M12 16v-4"></path>');
	expect(prose.html).not.toContain('title=');
});

it('rejects translator notes in translated frontmatter with the article named', async () => {
	const raw = '---\ntitle: ":tn[Translated title]{is=\\"a gloss\\"}"\nlang: en-US\n---\n\nBody.\n';
	await expect(
		compile(raw, '/article', {
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/bad-title.md',
		}),
	).rejects.toThrow(
		"contents/bad-title.md: translator's notes are not allowed in frontmatter title",
	);
});

it('compiles repository, crate, and tokei presentation controls into live widgets', async () => {
	const raw = `---
title: Test
lang: en-US
---

\`\`\`tokei title="Language statistics" view="bar"
 Language  Files  Lines  Code  Comments  Blanks
 Rust      1      10     8     1         1
\`\`\`

::github{repo="canmi21/seam" ref="abc123" title="Seam" align="right"}

::cargo{crate="seam-cli" view="table"}
`;
	const compiled = await compile(raw, '/article', {
		resolveAsset: () => null,
		highlight: async () => '',
		sourceFile: 'contents/widgets.md',
		embeds: {
			repos: {
				'canmi21/seam': {
					full_name: 'canmi21/seam',
					description: 'A repository.',
					language: 'Rust',
					stars: 1,
					forks: 2,
					open_issues: 3,
					license: 'MIT',
					pushed_at: '2026-01-01T00:00:00Z',
				},
			},
			crates: {
				'seam-cli': {
					name: 'seam-cli',
					version: '1.0.0',
					rust_version: null,
					features: {},
					deps: [],
					total_dep_size: 0,
				},
			},
		},
	});

	expect(compiled.blocks).toEqual(
		expect.arrayContaining([
			expect.objectContaining({ type: 'tokei', title: 'Language statistics', view: 'bar' }),
			expect.objectContaining({
				type: 'github',
				gitRef: 'abc123',
				title: 'Seam',
				align: 'right',
			}),
			expect.objectContaining({ type: 'cargo', view: 'table' }),
		]),
	);
});
