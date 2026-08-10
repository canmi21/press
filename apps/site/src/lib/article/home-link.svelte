<script lang="ts">
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let { locale }: { locale: LocaleCode } = $props();

	function followToc(node: HTMLElement) {
		let frame = 0;
		let toc: HTMLElement | undefined;
		let resize: ResizeObserver | undefined;
		let insertion: MutationObserver | undefined;

		const position = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => {
				const boundary = toc?.getBoundingClientRect().top ?? window.innerHeight / 2;
				node.style.top = remFromMeasuredPixels(Math.max(0, boundary) / 2);
			});
		};

		const connect = () => {
			const next = document.querySelector<HTMLElement>('.toc-nav');
			if (!next) {
				position();
				return;
			}

			toc = next;
			resize?.observe(toc);
			insertion?.disconnect();
			position();
		};

		if (typeof ResizeObserver !== 'undefined') resize = new ResizeObserver(position);
		insertion = new MutationObserver(connect);
		insertion.observe(document.body, { childList: true, subtree: true });
		window.addEventListener('resize', position);
		connect();

		return {
			destroy() {
				cancelAnimationFrame(frame);
				resize?.disconnect();
				insertion?.disconnect();
				window.removeEventListener('resize', position);
			},
		};
	}
</script>

<a
	use:followToc
	href="/"
	class="home-link focus-link fixed hidden -translate-y-1/2 items-center gap-1.5 text-[0.9375rem] text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong lg:inline-flex"
>
	<Undo2 class="size-4" aria-hidden="true" />
	<span>{m['nav.home']({}, { locale })}</span>
</a>

<style>
	.home-link {
		top: 25vh;
		right: calc(50% + 24rem);
		width: min(12rem, calc(50vw - 25.5rem));
	}
</style>
