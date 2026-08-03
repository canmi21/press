<script lang="ts">
	import { URLS } from '@canmi/urls';

	let {
		src,
		alt = '',
		width,
		height,
		preview,
		srcset,
		crop,
		align,
		el = $bindable(),
	}: {
		src: string;
		alt?: string;
		width?: number;
		height?: number;
		preview?: string;
		srcset?: string;
		/** A CSS aspect-ratio to crop to, e.g. `16 / 9`. Absent means show the whole image. */
		crop?: string;
		/** `object-position` within that crop. Absent means centred. */
		align?: string;
		el?: HTMLImageElement;
	} = $props();

	// Sized against the article column, which is what actually bounds these.
	const SIZES = '(max-width: 48rem) 100vw, 48rem';

	// Only AVIF is stored. Asking for any other extension is what tells the CDN to re-encode:
	// the worker serves `.avif` straight from the bucket and decodes anything else itself,
	// with WASM codecs. The extension already means "this format", so a query parameter would
	// be a second way to say the same thing -- and one that fragments the cache key.
	//
	// These two fallbacks are only ever fetched by a browser that cannot read AVIF, and the
	// result is cached at the edge under a name that carries a hash. See spec/architecture.md.
	function asFormat(set: string | undefined, extension: string): string | undefined {
		return set?.replaceAll('.avif ', `.${extension} `);
	}

	const webp = $derived(asFormat(srcset, 'webp'));
	const jpeg = $derived(asFormat(srcset, 'jpeg'));

	// Cropping is done here rather than by storing another object: a variant per ratio and
	// alignment would multiply the bucket, and would make a content id mean "this image as
	// shown here" rather than "this image". The cost is that the hidden part is still
	// downloaded, which is the cheaper of the two.
	const style = $derived(
		[
			preview && `background-image:url(${preview})`,
			preview && 'background-size:cover',
			preview && 'background-position:center',
			crop && `aspect-ratio:${crop}`,
			align && `object-position:${align}`,
		]
			.filter(Boolean)
			.join(';') || undefined,
	);

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
		style={style}
	/>
</picture>
