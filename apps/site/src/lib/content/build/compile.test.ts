import { expect, it } from 'vitest';
import { compile, compilePage } from './compile';

it('rejects malformed source lang metadata with the article file named', async () => {
	const raw = '---\ntitle: Test\nlang: zh_CN\n---\n\nBody.\n';
	await expect(
		compile(raw, '/article', {
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/bad-language.md',
		}),
	).rejects.toThrow('contents/bad-language.md: invalid BCP-47 lang frontmatter "zh_CN"');
});

it('keeps an email link working when its visible label is translated', () => {
	const page = compilePage(
		'---\ntitle: Test\n---\n\n:link[メール]{to=t@ffoni.com}\n',
		'opens in new tab',
	);
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
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/example.md',
		},
	);
	const prose = compiled.blocks[0];
	if (prose?.type !== 'prose') throw new Error('expected prose');

	expect(prose.html).toContain('<button type="button" class="tn-trigger focus-link"');
	expect(prose.html).toContain('data-tn-note="Its meaning needs context."');
	expect(prose.html).toContain('aria-controls="translator-note" aria-expanded="false"');
	expect(prose.html).toContain(
		'<svg class="tn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"',
	);
	expect(prose.html).toContain('<circle cx="12" cy="12" r="10"></circle>');
	expect(prose.html).toContain('<path d="M12 16v-4"></path>');
	expect(prose.html).not.toContain('title=');
});

it('marks prose links with the shared keyboard focus treatment', async () => {
	const compiled = await compile(
		'---\ntitle: Test\nlang: en-US\n---\n\nRead [the notes](https://example.com).\n',
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/example.md',
		},
	);
	const prose = compiled.blocks[0];
	if (prose?.type !== 'prose') throw new Error('expected prose');

	expect(prose.html).toContain(
		'<a href="https://example.com" class="focus-link spring-underline article-link">',
	);
});

it('rejects translator notes in translated frontmatter with the article named', async () => {
	const raw = '---\ntitle: ":tn[Translated title]{is=\\"a gloss\\"}"\nlang: en-US\n---\n\nBody.\n';
	await expect(
		compile(raw, '/article', {
			newTabNote: 'opens in new tab',
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

::twitter{tweet="2088060180290302397"}
`;
	const compiled = await compile(raw, '/article', {
		newTabNote: 'opens in new tab',
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
			tweets: {
				'2088060180290302397': {
					id: '2088060180290302397',
					author: 'canmi21',
					text: 'A tweet.',
					created: '2026-08-14T00:28:35Z',
					likes: 24,
					reposts: 0,
					replies: 4,
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
			expect.objectContaining({
				type: 'twitter',
				tweet: expect.objectContaining({ id: '2088060180290302397' }),
			}),
		]),
	);
});

it('routes a Mermaid fence to the client renderer without highlighting it', async () => {
	const source = `quadrantChart
  accTitle: Compiler trade-offs
  accDescr: A conceptual comparison of compiler visibility and ecosystem maturity.
  Svelte: [0.82, 0.84]`;
	const compiled = await compile(
		`---\ntitle: Test\nlang: en-US\n---\n\n\`\`\`Mermaid ratio="2.77366"\n${source}\n\`\`\`\n`,
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => {
				throw new Error('Mermaid source must not reach Shiki');
			},
			sourceFile: 'contents/diagram.md',
		},
	);

	expect(compiled.blocks).toContainEqual({ type: 'mermaid', source, ratio: 2.77366 });
	expect(compiled.feed).toContain('<code class="language-mermaid">quadrantChart');
	expect(compiled.markdown).toContain(`\`\`\`mermaid\n${source}\n\`\`\``);
});

it('rejects a Mermaid ratio that cannot become a safe aspect ratio', async () => {
	await expect(
		compile(
			'---\ntitle: Test\nlang: en-US\n---\n\n```mermaid ratio="wide"\nA --> B\n```\n',
			'/article',
			{
				newTabNote: 'opens in new tab',
				resolveAsset: () => null,
				highlight: async () => '',
				sourceFile: 'contents/broken-diagram.md',
			},
		),
	).rejects.toThrow('contents/broken-diagram.md: Mermaid ratio must be a positive decimal');
});

it('leaves Mermaid ratio optional', async () => {
	const source = 'flowchart LR\nA --> B';
	const compiled = await compile(
		`---\ntitle: Test\nlang: en-US\n---\n\n\`\`\`mermaid\n${source}\n\`\`\`\n`,
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/diagram.md',
		},
	);

	expect(compiled.blocks).toContainEqual({ type: 'mermaid', source });
});

it('compiles a categorical quadrant without inventing numeric positions', async () => {
	const compiled = await compile(
		`---
title: Test
lang: en-US
---

:::quadrant{title="UI stack trade-offs" description="Relative regions only." left="Smaller ecosystem" right="Broader ecosystem" top="More compile-time leverage" bottom="More runtime dependence"}
::quadrant-item{at="top-left" title="Solid" note="compiler-first"}
::quadrant-item{at="top-left" title="Marko"}
::quadrant-item{at="top-right" title="Svelte" note="visible structure + reach"}
:::
`,
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/quadrant.md',
		},
	);

	expect(compiled.blocks).toContainEqual({
		type: 'quadrant',
		title: 'UI stack trade-offs',
		description: 'Relative regions only.',
		axes: {
			top: 'More compile-time leverage',
			right: 'Broader ecosystem',
			bottom: 'More runtime dependence',
			left: 'Smaller ecosystem',
		},
		items: [
			{ at: 'top-left', title: 'Solid', note: 'compiler-first' },
			{ at: 'top-left', title: 'Marko' },
			{ at: 'top-right', title: 'Svelte', note: 'visible structure + reach' },
		],
	});
	expect(compiled.feed).toContain('<strong>Solid</strong> — compiler-first');
	expect(compiled.feed).toContain('<strong>Marko</strong>');
	expect(compiled.markdown).toContain(
		'> - More compile-time leverage / Smaller ecosystem: Solid — compiler-first',
	);
	expect(compiled.text).toContain('UI stack trade-offs\nRelative regions only.');
});

it('compiles a categorical quadrant without items', async () => {
	const compiled = await compile(
		`---
title: Test
lang: en-US
---

:::quadrant{title="Empty comparison" left="Left" right="Right" top="Top" bottom="Bottom"}
:::
`,
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/quadrant.md',
		},
	);

	expect(compiled.blocks).toContainEqual({
		type: 'quadrant',
		title: 'Empty comparison',
		axes: { top: 'Top', right: 'Right', bottom: 'Bottom', left: 'Left' },
		items: [],
	});
});

it('rejects a quadrant item outside the four categorical regions', async () => {
	await expect(
		compile(
			`---
title: Test
lang: en-US
---

:::quadrant{title="Broken" left="Left" right="Right" top="Top" bottom="Bottom"}
::quadrant-item{at="center" title="Nowhere"}
:::
`,
			'/article',
			{
				newTabNote: 'opens in new tab',
				resolveAsset: () => null,
				highlight: async () => '',
				sourceFile: 'contents/broken-quadrant.md',
			},
		),
	).rejects.toThrow(
		'contents/broken-quadrant.md: quadrant-item at must be one of top-left, top-right, bottom-left, bottom-right',
	);
});

it('rejects a quadrant item without its container', async () => {
	await expect(
		compile(
			'---\ntitle: Test\nlang: en-US\n---\n\n::quadrant-item{at="top-left" title="Loose"}\n',
			'/article',
			{
				newTabNote: 'opens in new tab',
				resolveAsset: () => null,
				highlight: async () => '',
				sourceFile: 'contents/loose-quadrant.md',
			},
		),
	).rejects.toThrow('contents/loose-quadrant.md: quadrant-item must be inside a quadrant');
});

it('compiles titled code fences into explicit disclosure states', async () => {
	const compiled = await compile(
		`---
title: Test
lang: en-US
---

\`\`\`ts title="Reference"
const open = true;
\`\`\`

\`\`\`ts title="Closed" default="collapsed"
const open = false;
\`\`\`

\`\`\`ts title="Fixed" collapsible="false"
const fixed = true;
\`\`\`
`,
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async (code) => `<pre>${code}</pre>`,
			sourceFile: 'contents/code-disclosures.md',
		},
	);

	expect(compiled.blocks).toEqual(
		expect.arrayContaining([
			expect.objectContaining({
				type: 'code',
				title: 'Reference',
				collapsible: true,
				defaultExpanded: true,
			}),
			expect.objectContaining({
				type: 'code',
				title: 'Closed',
				collapsible: true,
				defaultExpanded: false,
			}),
			expect.objectContaining({
				type: 'code',
				title: 'Fixed',
				collapsible: false,
				defaultExpanded: true,
			}),
		]),
	);
});

it('rejects contradictory code disclosure metadata', async () => {
	await expect(
		compile(
			'---\ntitle: Test\nlang: en-US\n---\n\n```ts title="Fixed" collapsible="false" default="collapsed"\ncode\n```\n',
			'/article',
			{
				newTabNote: 'opens in new tab',
				resolveAsset: () => null,
				highlight: async () => '',
				sourceFile: 'contents/broken-code-disclosure.md',
			},
		),
	).rejects.toThrow(
		'contents/broken-code-disclosure.md: a code fence cannot be fixed open and default collapsed',
	);
});

it('leaves an unfetched tweet visible as a directive placeholder', async () => {
	const compiled = await compile(
		'---\ntitle: Test\nlang: en-US\n---\n\n::twitter{tweet="2088060180290302397"}\n',
		'/article',
		{
			newTabNote: 'opens in new tab',
			resolveAsset: () => null,
			highlight: async () => '',
			sourceFile: 'contents/widgets.md',
		},
	);

	expect(compiled.blocks).toContainEqual({
		type: 'placeholder',
		kind: 'twitter',
		meta: { tweet: '2088060180290302397' },
	});
});
