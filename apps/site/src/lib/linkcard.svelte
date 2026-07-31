<script lang="ts">
	import { dev } from '$app/environment';
	import { imgsrc } from '@canmi/imgsrc';
	import { pickUrls } from '@canmi/urls';
	import Image from '$lib/image.svelte';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';

	type Props = {
		src: string;
		url: string;
		title: string;
		tone?: 'light' | 'dark';
		favicon?: string;
	};
	let { src, url, title, tone, favicon }: Props = $props();

	const cdnUrl = pickUrls(dev).cdn;
	const domain = $derived(new URL(url).hostname);
	const faviconSrc = $derived(
		favicon
			? imgsrc(favicon, { cdnUrl })
			: `${cdnUrl}/favicon/${domain}${tone ? `?tone=${tone}` : ''}`
	);

	let imgEl = $state<HTMLImageElement | undefined>();
	let hoverTint = $state<'black' | 'white' | null>(null);

	function computeHoverTint(img: HTMLImageElement): 'black' | 'white' | null {
		const size = 32;
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;
		const ctx = canvas.getContext('2d');
		if (!ctx) return null;
		try {
			ctx.drawImage(img, 0, 0, size, size);
			const { data } = ctx.getImageData(0, 0, size, size);
			const buckets = new Map<number, { r: number; g: number; b: number; count: number }>();
			for (let i = 0; i < data.length; i += 4) {
				if (data[i + 3] < 128) continue;
				const r = data[i];
				const g = data[i + 1];
				const b = data[i + 2];
				const key = ((r >> 5) << 6) | ((g >> 5) << 3) | (b >> 5);
				const entry = buckets.get(key);
				if (entry) {
					entry.r += r;
					entry.g += g;
					entry.b += b;
					entry.count++;
				} else {
					buckets.set(key, { r, g, b, count: 1 });
				}
			}
			let max: { r: number; g: number; b: number; count: number } | null = null;
			for (const v of buckets.values()) {
				if (!max || v.count > max.count) max = v;
			}
			if (!max) return null;
			const r = max.r / max.count;
			const g = max.g / max.count;
			const b = max.b / max.count;
			const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
			return luminance > 0.5 ? 'black' : 'white';
		} catch {
			return null;
		}
	}

	$effect(() => {
		const el = imgEl;
		if (!el) return;
		const handle = () => {
			hoverTint = computeHoverTint(el);
		};
		if (el.complete && el.naturalWidth > 0) {
			handle();
		} else {
			el.addEventListener('load', handle, { once: true });
			return () => el.removeEventListener('load', handle);
		}
	});
</script>

<a href={url} target="_blank" rel="noopener noreferrer" class="group relative isolate block">
	<div
		class="card-media transition duration-200 {hoverTint === 'black'
			? 'group-hover:brightness-90'
			: hoverTint === 'white'
				? 'group-hover:brightness-110'
				: ''}"
	>
		<Image {src} alt="" bind:el={imgEl} />
	</div>
	<div class="absolute right-12 bottom-3 left-3 flex items-center gap-2">
		<img src={faviconSrc} alt="" aria-hidden="true" loading="lazy" class="h-4 w-4 shrink-0" />
		<span class="truncate text-sm font-medium {tone === 'dark' ? 'text-black' : 'text-white'}">
			{title}
		</span>
	</div>
	<ArrowUpRight
		class="absolute right-3 bottom-3 h-4 w-4 {tone === 'dark' ? 'text-black' : 'text-white'} {tone
			? ''
			: 'mix-blend-difference'}"
	/>
</a>

<style>
	/* The card image already carries a 2px border, so the focus ring lands right on
	top of it: drop the box-shadow ring and recolor that border to the accent, so the
	2px ring overlaps the border exactly with no gap. */
	a:focus-visible {
		outline: none;
		box-shadow: none;
	}

	a:focus-visible .card-media :global(img) {
		border-color: var(--color-accent);
	}
</style>
