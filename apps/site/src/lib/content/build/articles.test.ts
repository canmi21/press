import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { buildArticles } from './articles';

const ROOT = new URL('../../../../../../', import.meta.url);

describe('article widget build inputs', () => {
	it('watches embed records and compiles every widget in the real article', async () => {
		const crates = fileURLToPath(new URL('data/build/crates.json', ROOT));
		const repos = fileURLToPath(new URL('data/build/repos.json', ROOT));
		const { articles, files } = await buildArticles({
			contents: fileURLToPath(new URL('contents', ROOT)),
			assets: fileURLToPath(new URL('data/metadata.json', ROOT)),
			media: fileURLToPath(new URL('data/media.yaml', ROOT)),
			segments: fileURLToPath(new URL('data/build/segments.json', ROOT)),
			crates,
			repos,
		});

		expect(files).toEqual(expect.arrayContaining([crates, repos]));
		const article = articles.find(
			(candidate) => candidate.path === 'development/rust-cargo-cranelift-tuning',
		);
		expect(article).toBeDefined();
		if (!article) throw new Error('missing rust-cargo-cranelift-tuning');
		for (const view of Object.values(article.views)) {
			expect(view.blocks.filter(({ type }) => type === 'tokei')).toHaveLength(1);
			expect(view.blocks.filter(({ type }) => type === 'github')).toHaveLength(1);
			expect(view.blocks.filter(({ type }) => type === 'cargo')).toHaveLength(2);
			expect(view.summary).toMatchObject({ provider: 'openai' });
		}
		expect(files).toContain(
			fileURLToPath(new URL('contents/development/rust-cargo-cranelift-tuning.summary.yaml', ROOT)),
		);
	});
});
