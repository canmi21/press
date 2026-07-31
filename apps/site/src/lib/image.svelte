<script lang="ts">
	import { URLS } from '@canmi/urls';
	let {
		src,
		alt = '',
		el = $bindable()
	}: { src: string; alt?: string; el?: HTMLImageElement } = $props();
	const imgSrc = $derived(`${URLS.apps.production.cdn}/image/${src}`);
</script>

<img
	bind:this={el}
	src={imgSrc}
	{alt}
	loading="lazy"
	crossorigin="anonymous"
	class="block aspect-video w-full rounded-2xl border-2 border-border object-cover"
/>
