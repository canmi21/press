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
		/**
		 * How much bigger than its natural size to draw the sheet.
		 *
		 * Applied as `zoom` rather than a width, because the bar widths and gaps are inline rem
		 * values from `ARTICLE_THUMBNAIL_LINES` -- growing the box alone would leave a hand-tuned
		 * composition floating in a larger frame. `zoom` takes the layout box and everything in it
		 * together, so the sheet is the same drawing at another size.
		 */
		scale = 1,
	}: { focusRing?: boolean; scale?: number } = $props();
</script>

<!-- A4-ish sheet. Five bars carry the hand-tuned first-frame widths and gaps; `data-article-icon`
     is the handle the homepage list uses to animate them to a content-derived shape, and is inert
     anywhere outside one. See list.svelte. -->
<div
	data-article-icon
	aria-hidden="true"
	class="article-preview-thumbnail"
	class:focus-ring-inner={focusRing}
	style:zoom={scale === 1 ? null : scale}
>
	{#each ARTICLE_THUMBNAIL_LINES as line}
		<span data-icon-bar style:width={line.width} style:margin-top={line.marginTop}></span>
	{/each}
</div>
