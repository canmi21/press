<script lang="ts">
	import Article from '$lib/article/article.svelte';
	import ArticleBody from '$lib/article/body.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<Article
	slug={data.slug}
	meta={data.meta}
	toc={data.toc}
	chars={data.chars}
	summary={data.summary}
	locale={data.locale}
>
	<ArticleBody blocks={data.blocks} locale={data.locale.code} />
</Article>
