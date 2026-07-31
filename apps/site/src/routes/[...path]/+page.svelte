<script lang="ts">
	import Article from '$lib/article.svelte';
	import ArticleBody from '$lib/content/article-body.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<Article meta={data.meta} chars={data.chars}>
	<ArticleBody blocks={data.blocks} />
</Article>
