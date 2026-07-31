<script lang="ts">
	import { URLS } from '@canmi/urls';

	let {
		src,
		alt = '',
		width,
		height,
		preview,
		srcset,
		el = $bindable(),
	}: {
		src: string;
		alt?: string;
		width?: number;
		height?: number;
		preview?: string;
		srcset?: string;
		el?: HTMLImageElement;
	} = $props();

	// Sized against the article column, which is what actually bounds these.
	const SIZES = '(max-width: 48rem) 100vw, 48rem';

	// Only AVIF is stored. Asking for any other extension is what tells the CDN to convert:
	// the worker serves `.avif` straight from the bucket and translates everything else into a
	// transformation. The extension already means "this format", so a query parameter would be
	// a second way to say the same thing -- and one that fragments the cache key.
	//
	// Cloudflare counts a format conversion once per image however many formats it hands out,
	// so both fallbacks together cost one transformation rather than two more copies of the
	// whole library. See spec/architecture.md.
	function asFormat(set: string | undefined, extension: string): string | undefined {
		return set?.replaceAll('.avif ', `.${extension} `);
	}

	const webp = $derived(asFormat(srcset, 'webp'));
	const jpeg = $derived(asFormat(srcset, 'jpeg'));

	// An article can name an image that has not been imported yet. That should cost a
	// placeholder rather than a build, so an unresolved reference still renders.
	const fallback = $derived(`${URLS.apps.production.cdn}/image/${src}`);
	// The `img` is what a browser understanding none of the sources falls back to, so it names
	// the widest-supported format rather than the best one.
	const largestJpeg = $derived(jpeg?.split(', ').pop()?.split(' ')[0] ?? fallback);
</script>

<picture>
	{#if srcset}
		<source type="image/avif" {srcset} sizes={SIZES} />
		<source type="image/webp" srcset={webp} sizes={SIZES} />
	{/if}
	<img
		bind:this={el}
		src={largestJpeg}
		srcset={jpeg}
		sizes={srcset ? SIZES : undefined}
		{alt}
		{width}
		{height}
		loading="lazy"
		decoding="async"
		crossorigin="anonymous"
		class="block w-full rounded-2xl border-2 border-border object-cover"
		style={preview
			? `background-image:url(${preview});background-size:cover;background-position:center`
			: undefined}
	/>
</picture>
