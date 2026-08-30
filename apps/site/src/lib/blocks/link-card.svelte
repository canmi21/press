<script lang="ts">
	import { dev } from '$app/environment';
	import { pickUrls } from '@canmi/urls';
	import Image from './image.svelte';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import { m } from '$lib/paraglide/messages';
	import type { LocaleCode } from '$lib/locale';

	type Props = {
		/** The view being rendered. Passed rather than read: see spec/locale.md. */
		locale: LocaleCode;
		src: string;
		url: string;
		title: string;
		tone?: 'light' | 'dark';
		/**
		 * Where `cms favicon` should take this site's icon from.
		 *
		 * Accepted and ignored here on purpose. It is an instruction to the collector, which
		 * resolves it into the domain's own slot under `data/public/favicon`, so by the time a
		 * page renders the answer is already at `/favicon/{domain}` and reading the attribute
		 * again would send the browser to somebody else's origin for a copy we hold.
		 */
		favicon?: string;
		width?: number;
		height?: number;
		preview?: string;
		srcset?: string;
		/** The cover's crop, resolved at compile time. See spec/architecture/media.md. */
		crop?: string;
		/** `object-position` within that crop. Absent means centred. */
		align?: string;
		/** What the cover shows, from the manifest. See the markup for where it goes. */
		description?: string;
	};
	let {
		locale,
		src,
		url,
		title,
		tone,
		width,
		height,
		preview,
		srcset,
		crop,
		align,
		description,
	}: Props = $props();

	const describedBy = $props.id();

	const cdnUrl = pickUrls(dev).cdn;
	const domain = $derived(new URL(url).hostname);
	const faviconSrc = $derived(`${cdnUrl}/favicon/${domain}${tone ? `?tone=${tone}` : ''}`);

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
				const r = data[i];
				const g = data[i + 1];
				const b = data[i + 2];
				const a = data[i + 3];
				if (r === undefined || g === undefined || b === undefined || a === undefined) continue;
				if (a < 128) continue;
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

<!--
	The cover keeps `alt=""` deliberately, and this is the one decision here worth arguing.

	Everything inside an anchor becomes part of the link's accessible name. Putting an
	800-character description there would make the link announce as the whole screenshot before
	saying where it goes, and a reader tabbing through links would have to sit through it every
	time. A link's name should identify its destination and stop.

	So the description is offered as a *description* instead: `aria-describedby` points at the
	hidden text below, which a screen reader announces after the name and lets the reader skip.
	The content is available without being in the way.

	The name itself gains the domain and the new-tab warning. "Hexo: A fast, simple & powerful
	blog framework" never said it went to hexo.io -- the favicon carries that visually and is
	`aria-hidden`, so without this the destination was sighted-only.
-->
<a
	href={url}
	target="_blank"
	rel="noopener noreferrer"
	aria-describedby={description ? describedBy : undefined}
	class="group relative isolate block"
>
	<div
		class="card-media transition duration-200 {hoverTint === 'black'
			? 'group-hover:brightness-90'
			: hoverTint === 'white'
				? 'group-hover:brightness-110'
				: ''}"
	>
		<Image {src} alt="" {width} {height} {preview} {srcset} {crop} {align} bind:el={imgEl} />
	</div>
	<div class="absolute right-12 bottom-3 left-3 flex items-center gap-2">
		<img src={faviconSrc} alt="" aria-hidden="true" loading="lazy" class="h-4 w-4 shrink-0" />
		<span class="truncate text-sm font-medium {tone === 'dark' ? 'text-black' : 'text-white'}">
			{title}
		</span>
		<span class="sr-only">, {domain}, {m['support.new-tab']({}, { locale })}</span>
	</div>
	<ArrowUpRight
		aria-hidden="true"
		class="absolute right-3 bottom-3 h-4 w-4 {tone === 'dark' ? 'text-black' : 'text-white'} {tone
			? ''
			: 'mix-blend-difference'}"
	/>
</a>
{#if description}
	<!-- Outside the anchor on purpose: inside, it would join the name it is meant to follow. -->
	<span id={describedBy} class="sr-only">{description}</span>
{/if}

<style>
	/* The card image already carries a 0.125rem border, so the focus ring lands right on
	top of it: drop the box-shadow ring and recolor that border to the accent, so the
	0.125rem ring overlaps the border exactly with no gap. */
	a:focus-visible {
		outline: 0.125rem solid transparent;
	}

	a:focus-visible .card-media :global(img) {
		border-color: var(--color-accent);
	}
</style>
