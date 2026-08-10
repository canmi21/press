<script lang="ts">
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	const DEFAULT_TOP_REM = 6.75;

	let { locale }: { locale: LocaleCode } = $props();

	function titleCenter() {
		const title = document.querySelector<HTMLElement>('article h1');
		if (!title) return;
		const rect = title.getBoundingClientRect();
		const lineHeight = Number.parseFloat(getComputedStyle(title).lineHeight) || rect.height;
		// Scrolling the article must not move fixed navigation away from its default alignment.
		return rect.top + window.scrollY + lineHeight / 2;
	}

	function followToc(node: HTMLElement) {
		let frame = 0;
		let toc: HTMLElement | undefined;
		let restingTop = 0;
		let pendingBoundary = window.innerHeight / 2;
		let renderedOffset = '';
		let rootPixels = DEFAULT_PIXELS_PER_REM;
		let resize: ResizeObserver | undefined;
		let insertion: MutationObserver | undefined;

		const position = (boundary: number) => {
			pendingBoundary = boundary;
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => {
				// Title alignment yields only when the ToC consumes that space. See spec/styling.md.
				const target = Math.min(restingTop, Math.max(0, pendingBoundary) / 2);
				const offset = remFromMeasuredPixels(
					target - DEFAULT_TOP_REM * rootPixels,
					rootPixels,
				);
				if (offset === renderedOffset) return;
				renderedOffset = offset;
				node.style.setProperty('--home-offset', offset);
			});
		};

		const calibrate = () => {
			rootPixels =
				Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
				DEFAULT_PIXELS_PER_REM;
			restingTop = titleCenter() ?? window.innerHeight / 4;
			position(toc?.getBoundingClientRect().top ?? window.innerHeight / 2);
		};

		const connect = () => {
			const next = document.querySelector<HTMLElement>('.toc-nav');
			if (!next) {
				calibrate();
				return;
			}

			toc = next;
			resize?.observe(toc);
			insertion?.disconnect();
			calibrate();
		};

		if (typeof ResizeObserver !== 'undefined') {
			resize = new ResizeObserver(([entry]) => {
				if (!entry) return;
				// Layout is already complete here; deriving the fixed midpoint from box size avoids
				// making the ToC animation pay for another geometry read. See spec/styling.md.
				const size = entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height;
				position((window.innerHeight - size) / 2);
			});
		}
		insertion = new MutationObserver(connect);
		insertion.observe(document.body, { childList: true, subtree: true });
		window.addEventListener('resize', calibrate);
		connect();

		return {
			destroy() {
				cancelAnimationFrame(frame);
				resize?.disconnect();
				insertion?.disconnect();
				window.removeEventListener('resize', calibrate);
			},
		};
	}
</script>

<div
	use:followToc
	class="home-slot pointer-events-none fixed hidden items-center lg:flex"
>
	<a
		href="/"
		class="home-link focus-link pointer-events-auto inline-flex -translate-x-5 items-center gap-1.5 whitespace-nowrap text-sm text-text-soft transition-colors duration-200 hover:text-text-strong focus-visible:text-text-strong"
	>
		<Undo2 class="size-3.5 shrink-0 -translate-y-[0.03125rem]" aria-hidden="true" />
		<span>{m['article.back']({}, { locale })}</span>
	</a>
</div>

<style>
	.home-slot {
		top: 6.75rem;
		right: calc(50% + 24rem);
		transform: translateY(calc(-50% + var(--home-offset, 0rem)));
		width: min(12rem, calc(50vw - 25.5rem));
	}
</style>
