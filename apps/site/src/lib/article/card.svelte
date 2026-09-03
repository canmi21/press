<script lang="ts">
	import { ARTICLE_THUMBNAIL_LINES } from '@canmi/primitives';
	import { shortDate } from '$lib/format';

	let {
		title,
		subtitle,
		created,
		path,
	}: {
		title: string;
		subtitle: string;
		created: string;
		path: string;
	} = $props();

	const date = $derived(shortDate(created));
</script>

<a href="/{path}" class="article-preview group focus-visible:outline-none">
	<!-- A4-ish sheet. Five bars carry the hand-tuned first-frame widths/gaps; after
	hydration the article list measures the corpus and animates them to a content-derived
	shape (normalized list-wide, see list.svelte). -->
	<div data-article-icon aria-hidden="true" class="article-preview-thumbnail focus-ring-inner">
		{#each ARTICLE_THUMBNAIL_LINES as line}
			<span data-icon-bar style:width={line.width} style:margin-top={line.marginTop}></span>
		{/each}
	</div>

	<div class="article-preview-copy">
		<!-- Title shares its line with the dotted leader and date, so the leader
		starts at the title's end rather than the (often longer) subtitle below. -->
		<div class="article-preview-heading">
			<h3 class="article-preview-title">{title}</h3>
			<div class="article-preview-leader"></div>
			<time datetime={created} class="article-preview-date">{date}</time>
		</div>
		<p class="article-preview-subtitle">{subtitle}</p>
	</div>
</a>
