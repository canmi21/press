<script lang="ts">
	import { ARTICLE_THUMBNAIL_LINES } from '@canmi/primitives';

	let {
		/**
		 * Whether this thumbnail shows the focus ring for the link around it.
		 *
		 * The homepage anchor suppresses its own outline so the ring lands on the sheet; a boxed
		 * card rings the box instead, and `focus-ring-inner` only fires as a direct child of the
		 * focused element anyway. See apps/site/src/styles/utilities.css.
		 */
		focusRing = false,
	}: { focusRing?: boolean } = $props();
</script>

<!-- A4-ish sheet. Five bars carry the hand-tuned first-frame widths and gaps; `data-article-icon`
     is the handle the homepage list uses to animate them to a content-derived shape, and is inert
     anywhere outside one. See list.svelte. -->
<div
	data-article-icon
	aria-hidden="true"
	class="article-preview-thumbnail"
	class:focus-ring-inner={focusRing}
>
	{#each ARTICLE_THUMBNAIL_LINES as line}
		<span data-icon-bar style:width={line.width} style:margin-top={line.marginTop}></span>
	{/each}
</div>
