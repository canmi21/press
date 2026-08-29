<script lang="ts">
	import Article from '$lib/article/article.svelte';
	import ArticleBody from '$lib/article/body.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	// Collected notes leave the block stream here: they render after the article's closing
	// rule rather than inside the body, because they are apparatus about the article, not part
	// of it. See spec/styling.md.
	const notes = $derived(
		data.blocks.flatMap((block) => (block.type === 'footnotes' ? block.notes : [])),
	);
</script>

<Article
	slug={data.slug}
	meta={data.meta}
	toc={data.toc}
	chars={data.chars}
	summary={data.summary}
	locale={data.locale}
	{notes}
>
	<ArticleBody blocks={data.blocks} locale={data.locale.code} />
</Article>
