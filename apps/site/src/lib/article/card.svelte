<script lang="ts">
	import { ARTICLE_THUMBNAIL_LINES } from '@canmi/primitives';
	import { shortDate } from '$lib/format';

	let {
		title,
		subtitle,
		shortTitle,
		shortSubtitle,
		created,
		path,
	}: {
		title: string;
		subtitle: string;
		/** What a phone shows instead. Equal to the full form where none was written. */
		shortTitle: string;
		shortSubtitle: string;
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
			<!-- Both forms are in the document and CSS chooses, so the choice survives the server
			     render. Rendering one of them would mean deciding at build time what a reader's
			     screen is, and the row would be wrong for the first frame at every width. Where
			     no short form was written the two strings are equal, which costs the reader
			     nothing and one duplicated word in the markup.
			     Only one is ever read aloud: the other is hidden from the accessibility tree by
			     `display: none`, which is what these variants compile to. -->
			<h3 class="selectable article-preview-title max-sm:hidden">{title}</h3>
			<h3 class="selectable article-preview-title sm:hidden">{shortTitle}</h3>
			<div class="article-preview-leader"></div>
			<time datetime={created} class="article-preview-date">{date}</time>
		</div>
		<p class="selectable article-preview-subtitle max-sm:hidden">{subtitle}</p>
		<p class="selectable article-preview-subtitle sm:hidden">{shortSubtitle}</p>
	</div>
</a>
