<script lang="ts">
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let { locale }: { locale: LocaleCode } = $props();

	function titleCenter() {
		const title = document.querySelector<HTMLElement>('article h1');
		if (!title) return;
		const rect = title.getBoundingClientRect();
		// Scrolling the article must not move fixed navigation away from its default alignment.
		return rect.top + window.scrollY + rect.height / 2;
	}

	function followToc(node: HTMLElement) {
		let frame = 0;
		let toc: HTMLElement | undefined;
		let resize: ResizeObserver | undefined;
		let insertion: MutationObserver | undefined;

		const position = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => {
				const boundary = toc?.getBoundingClientRect().top ?? window.innerHeight / 2;
				const title = titleCenter() ?? window.innerHeight / 4;
				// Title alignment yields only when the ToC consumes that space. See spec/styling.md.
				node.style.top = remFromMeasuredPixels(Math.min(title, Math.max(0, boundary) / 2));
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

<div
	use:followToc
	class="home-slot pointer-events-none fixed hidden -translate-y-1/2 items-center lg:flex"
>
	<a
		href="/"
		class="home-link focus-link pointer-events-auto inline-flex -translate-x-[1.375rem] items-center gap-1.5 whitespace-nowrap text-sm text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
	>
		<Undo2 class="size-4 shrink-0" aria-hidden="true" />
		<span>{m['article.back']({}, { locale })}</span>
	</a>
</div>

<style>
	.home-slot {
		top: 25vh;
		right: calc(50% + 24rem);
		width: min(12rem, calc(50vw - 25.5rem));
	}
</style>
