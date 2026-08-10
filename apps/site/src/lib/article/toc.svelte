<script lang="ts">
	import { measureNaturalWidth, prepareWithSegments } from '@chenglou/pretext';
	import { animate } from 'motion';
	import {
		DEFAULT_PIXELS_PER_REM,
		remFromDefaultPixels,
		remFromMeasuredPixels,
	} from '$lib/client/units';
	import type { TocEntry } from '$lib/content/types';

	let { toc }: { toc: TocEntry[] } = $props();

	const MAX_BAR_WIDTH = 64;
	const MIN_BAR_WIDTH = 8;
	const BAR_HEIGHT = 4;
	const INDICATOR_HEIGHT = 12;
	const REVEAL_DELAY = 180;
	const LEAVE_DELAY = 250;
	const SCROLL_OFFSET = 96;
	const TOP_DEAD_ZONE = 64;
	const BAR_SPRING = { type: 'spring' as const, stiffness: 300, damping: 28 };
	const TEXT_TWEEN = { duration: 0.15 };

	// Geometry is authored against the default root and written as rem. Calculations that mix it
	// with DOM measurements scale it to the live root first.
	const rootFontPixels = () =>
		Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
		DEFAULT_PIXELS_PER_REM;
	const toScaledPixels = (value: number, root: number) =>
		(value / DEFAULT_PIXELS_PER_REM) * root;

	type Phase = 'collapsed' | 'expanded' | 'revealed';
	type Entry = { el?: HTMLHeadingElement; slug: string; width: number; text: string };
	type HydratedEntries = { source: TocEntry[]; entries: Entry[] };
	type IndicatorGeometry = { y: number; height: number };
	type AnimationControl = { stop: () => void };

	let hydratedEntries = $state.raw<HydratedEntries>();
	const entries = $derived(
		hydratedEntries?.source === toc
			? hydratedEntries.entries
			: toc.map<Entry>(({ slug, text }) => ({ slug, width: 0, text })),
	);
	let asideEl = $state<HTMLElement | undefined>();
	let indicatorEl = $state<HTMLElement | undefined>();
	let phase = $state<Phase>('collapsed');
	let activeIndex = $state(-1);
	let firstWidthSet = false;
	let firstShowSet = false;
	let firstIndicatorSet = false;
	let prevIndicatorVisible = false;
	let prevIndicatorActive = -1;
	let prevGeometryVersion = 0;
	let isClickScrolling = false;
	let phaseTimer: ReturnType<typeof setTimeout> | undefined;
	let leaveTimer: ReturnType<typeof setTimeout> | undefined;
	let trackingToken = 0;
	let trackingRAF: number | undefined;
	let indicatorAnimation: AnimationControl | undefined;
	let geometryVersion = $state(0);

	function jumpToSection(el: HTMLHeadingElement | undefined, idx: number) {
		if (!el) return;
		isClickScrolling = true;
		activeIndex = idx;
		el.scrollIntoView({ behavior: 'smooth', block: 'start' });
		const onEnd = () => {
			isClickScrolling = false;
		};
		if ('onscrollend' in window) {
			window.addEventListener('scrollend', onEnd, { once: true });
		} else {
			setTimeout(onEnd, 600);
		}
	}

	function fontOf(el: HTMLElement): string {
		const cs = getComputedStyle(el);
		return `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
	}

	function linearBars(widths: number[]): number[] {
		if (widths.length === 0) return [];
		const max = Math.max(...widths);
		if (max < 1) return widths.map(() => MAX_BAR_WIDTH / 2);
		return widths.map((w) => Math.max(MIN_BAR_WIDTH, (w / max) * MAX_BAR_WIDTH));
	}

	function indicatorGeometry(button: HTMLElement): IndicatorGeometry {
		const label = button.querySelector<HTMLElement>('[data-toc-text]');
		const lineHeight = label ? parseFloat(getComputedStyle(label).lineHeight) : 0;
		const measuredLines = label && lineHeight > 0 ? Math.round(label.scrollHeight / lineHeight) : 1;
		const lines = Math.min(2, Math.max(1, measuredLines));
		const height =
			toScaledPixels(INDICATOR_HEIGHT, rootFontPixels()) + lineHeight * (lines - 1);
		const center = button.offsetTop + button.offsetHeight / 2;
		return { y: center - height / 2, height };
	}

	const barWidths = $derived(linearBars(entries.map((e) => e.width)));
	const showText = $derived(phase === 'revealed');

	function handleEnter() {
		if (leaveTimer) {
			clearTimeout(leaveTimer);
			leaveTimer = undefined;
		}
		if (phase === 'revealed') return;
		if (phaseTimer) clearTimeout(phaseTimer);
		phase = 'expanded';
		phaseTimer = setTimeout(() => (phase = 'revealed'), REVEAL_DELAY);
	}

	function handleLeave() {
		if (phaseTimer) clearTimeout(phaseTimer);
		if (leaveTimer) clearTimeout(leaveTimer);
		leaveTimer = setTimeout(() => (phase = 'collapsed'), LEAVE_DELAY);
	}

	$effect(() => {
		const source = toc;
		const headings = source
			.map(({ slug }) => document.getElementById(slug))
			.filter(
				(el): el is HTMLHeadingElement =>
					el instanceof HTMLHeadingElement && (el.textContent?.trim() ?? '') !== '',
			);

		hydratedEntries = {
			source,
			entries: headings.map((el) => ({
				el,
				slug: el.id,
				width: 0,
				text: el.textContent?.trim() ?? '',
			})),
		};

		const cleanups: Array<() => void> = [];

		for (const el of headings) {
			el.style.scrollMarginTop = remFromDefaultPixels(SCROLL_OFFSET);
			cleanups.push(() => {
				el.style.scrollMarginTop = '';
			});
		}

		// Only on a fresh navigation. A reload keeps the position the reader had scrolled to,
		// which is the browser's own behaviour and what somebody reloading halfway down a page
		// wants; taking over both alike is what throws that away. See spec/styling.md.
		const initialHash = window.location.hash.slice(1);
		const nav = performance.getEntriesByType('navigation')[0] as
			| PerformanceNavigationTiming
			| undefined;
		if (initialHash && nav?.type === 'navigate') {
			const target = document.getElementById(initialHash);
			if (target) {
				const targetIdx = headings.indexOf(target as HTMLHeadingElement);
				if (targetIdx >= 0) activeIndex = targetIdx;
				isClickScrolling = true;
				window.scrollTo({ top: 0, behavior: 'instant' });
				requestAnimationFrame(() => {
					const top = target.getBoundingClientRect().top + window.scrollY - SCROLL_OFFSET / 2;
					window.scrollTo({ top, behavior: 'smooth' });
					const onEnd = () => {
						isClickScrolling = false;
					};
					if ('onscrollend' in window) {
						window.addEventListener('scrollend', onEnd, { once: true });
					} else {
						setTimeout(onEnd, 1500);
					}
				});
			}
		}

		const raf = requestAnimationFrame(() => {
			const measured: Entry[] = [];
			for (const el of headings) {
				const text = el.textContent?.trim() ?? '';
				const prepared = prepareWithSegments(text, fontOf(el));
				const w = measureNaturalWidth(prepared);
				measured.push({ el, slug: el.id, width: w, text });
			}
			hydratedEntries = { source, entries: measured };
		});

		const onScroll = () => {
			if (isClickScrolling) return;
			if (window.scrollY <= TOP_DEAD_ZONE) {
				activeIndex = -1;
				if (window.location.hash) {
					history.replaceState(null, '', window.location.pathname + window.location.search);
				}
			}
		};
		window.addEventListener('scroll', onScroll, { passive: true });
		cleanups.push(() => window.removeEventListener('scroll', onScroll));

		const observer = new IntersectionObserver(
			(ixs) => {
				if (isClickScrolling) return;
				if (window.scrollY <= TOP_DEAD_ZONE) return;
				for (const ix of ixs) {
					if (ix.isIntersecting) {
						const idx = headings.indexOf(ix.target as HTMLHeadingElement);
						if (idx >= 0) {
							activeIndex = idx;
							break;
						}
					}
				}
			},
			{ rootMargin: '0% 0% -70% 0%', threshold: 0 }
		);
		for (const h of headings) observer.observe(h);
		cleanups.push(() => observer.disconnect());

		if (window.scrollY > TOP_DEAD_ZONE) {
			const threshold = window.scrollY + window.innerHeight * 0.3;
			let initialIdx = -1;
			for (let i = 0; i < headings.length; i++) {
				const top = headings[i].getBoundingClientRect().top + window.scrollY;
				if (top <= threshold) initialIdx = i;
			}
			if (initialIdx >= 0) activeIndex = initialIdx;
		}

		return () => {
			cancelAnimationFrame(raf);
			for (const c of cleanups) c();
			if (phaseTimer) clearTimeout(phaseTimer);
			if (leaveTimer) clearTimeout(leaveTimer);
			indicatorAnimation?.stop();
		};
	});

	$effect(() => {
		if (!asideEl) return;
		const widths = barWidths;
		const active = activeIndex;
		const bars = asideEl.querySelectorAll<HTMLElement>('[data-toc-bar]');
		if (bars.length !== entries.length || bars.length === 0) return;

		if (!firstWidthSet) {
			firstWidthSet = true;
			return;
		}

		if (showText) return;

		for (let i = 0; i < entries.length; i++) {
			const bw = widths[i] ?? MAX_BAR_WIDTH / 2;
			const op = i === active ? 0.8 : 0.35;
			const bar = bars[i];
			animate(
				bar,
				{
					width: remFromDefaultPixels(bw),
					height: remFromDefaultPixels(BAR_HEIGHT),
					opacity: op,
				},
				{
					...BAR_SPRING,
					onComplete: () => {
						bar.style.width = remFromDefaultPixels(bw);
						bar.style.height = remFromDefaultPixels(BAR_HEIGHT);
					}
				}
			);
		}
	});

	$effect(() => {
		if (!asideEl) return;
		const show = showText;
		const widths = barWidths;
		const active = activeIndex;
		const bars = asideEl.querySelectorAll<HTMLElement>('[data-toc-bar]');
		const texts = asideEl.querySelectorAll<HTMLElement>('[data-toc-text]');
		if (bars.length !== entries.length || bars.length === 0) return;

		if (!firstShowSet) {
			firstShowSet = true;
			return;
		}

		for (let i = 0; i < entries.length; i++) {
			const bw = widths[i] ?? MAX_BAR_WIDTH / 2;
			const restOp = i === active ? 0.8 : 0.35;
			const bar = bars[i];
			const targetW = show ? 0 : bw;
			const targetH = show ? 0 : BAR_HEIGHT;
			animate(
				bar,
				{
					width: remFromDefaultPixels(targetW),
					height: remFromDefaultPixels(targetH),
					opacity: show ? 0 : restOp
				},
				{
					...BAR_SPRING,
					onComplete: () => {
						bar.style.width = remFromDefaultPixels(targetW);
						bar.style.height = remFromDefaultPixels(targetH);
					}
				}
			);
			animate(
				texts[i],
				{
					height: show ? 'auto' : 0,
					opacity: show ? 1 : 0
				},
				TEXT_TWEEN
			);
		}
	});

	$effect(() => {
		if (!asideEl || !indicatorEl) return;
		const show = showText;
		const active = activeIndex;
		const geometry = geometryVersion;
		const buttons = asideEl.querySelectorAll<HTMLElement>('[data-toc-button]');
		if (buttons.length !== entries.length || buttons.length === 0) return;

		if (!firstIndicatorSet) {
			firstIndicatorSet = true;
			prevGeometryVersion = geometry;
			return;
		}
		const geometryChanged = geometry !== prevGeometryVersion;
		prevGeometryVersion = geometry;

		trackingToken++;
		if (trackingRAF !== undefined) {
			cancelAnimationFrame(trackingRAF);
			trackingRAF = undefined;
		}
		indicatorAnimation?.stop();
		indicatorAnimation = undefined;

		const visible = show && active >= 0 && active < buttons.length;
		if (!visible) {
			indicatorEl.style.opacity = '0';
			prevIndicatorVisible = false;
			prevIndicatorActive = active;
			return;
		}

		if (!prevIndicatorVisible || (geometryChanged && active === prevIndicatorActive)) {
			const setLivePos = () => {
				if (!asideEl || !indicatorEl) return;
				const btn = asideEl.querySelectorAll<HTMLElement>('[data-toc-button]')[active];
				if (!btn) return;
				const target = indicatorGeometry(btn);
				indicatorEl.style.height = remFromMeasuredPixels(target.height);
				indicatorEl.style.transform = `translateY(${remFromMeasuredPixels(target.y)})`;
			};
			setLivePos();
			indicatorEl.style.opacity = '0.8';

			if (!prevIndicatorVisible) {
				const myToken = trackingToken;
				const start = performance.now();
				const tick = () => {
					if (myToken !== trackingToken) return;
					setLivePos();
					if (performance.now() - start < 200) {
						trackingRAF = requestAnimationFrame(tick);
					} else {
						trackingRAF = undefined;
					}
				};
				trackingRAF = requestAnimationFrame(tick);
			}
		} else {
			const target = indicatorGeometry(buttons[active]);
			const indicator = indicatorEl;
			const navRect = asideEl.getBoundingClientRect();
			const indicatorRect = indicator.getBoundingClientRect();
			const startHeight = indicatorRect.height;
			const startCenter = indicatorRect.top - navRect.top + startHeight / 2;
			const targetCenter = target.y + target.height / 2;
			indicatorAnimation = animate(0, 1, {
				...BAR_SPRING,
				onUpdate: (progress) => {
					const height = startHeight + (target.height - startHeight) * progress;
					const center = startCenter + (targetCenter - startCenter) * progress;
					indicator.style.height = remFromMeasuredPixels(height);
					indicator.style.transform = `translateY(${remFromMeasuredPixels(center - height / 2)})`;
				},
				onComplete: () => {
					indicator.style.height = remFromMeasuredPixels(target.height);
					indicator.style.transform = `translateY(${remFromMeasuredPixels(target.y)})`;
					indicatorAnimation = undefined;
				}
			});
		}
		prevIndicatorVisible = visible;
		prevIndicatorActive = active;
	});

	$effect(() => {
		if (!asideEl || typeof ResizeObserver === 'undefined') return;
		const observer = new ResizeObserver(() => {
			geometryVersion += 1;
		});
		observer.observe(asideEl);
		return () => observer.disconnect();
	});
</script>

{#if entries.length > 0}
	<nav
		bind:this={asideEl}
		aria-label="Table of contents"
		onmouseenter={handleEnter}
		onmouseleave={handleLeave}
		class:revealed={showText}
		class="toc-nav fixed top-1/2 hidden -translate-y-1/2 flex-col items-start overflow-visible lg:flex"
	>
		<span
			bind:this={indicatorEl}
			class="pointer-events-none absolute w-0.5 rounded-full bg-text-soft"
			style="left: -0.5rem; top: 0; height: 0.75rem; opacity: 0"
		></span>
		{#each entries as entry, i (entry.slug)}
			<button
				data-toc-button
				type="button"
				aria-label={entry.text}
				aria-current={i === activeIndex ? 'location' : undefined}
				title={entry.text}
				onclick={() => jumpToSection(entry.el, i)}
				class="block max-w-full cursor-pointer py-[0.1875rem] text-left focus-visible:outline-none"
			>
				<!-- Bar and text each sit in a full-opacity ring host: the inner span carries
				the opacity animation, so drawing the focus ring on the wrapper keeps it crisp
				instead of inheriting the dimmed opacity. The collapsed/revealed state picks
				which wrapper shows the ring (see <style>). -->
				<span class:focus-ring-inner={!showText} class="toc-ring-bar block w-fit rounded-full">
					<span
						data-toc-bar
						class="block rounded-full bg-text-soft"
						style="width: 2rem; height: 0.25rem; opacity: 0.35"
					></span>
				</span>
				<span class:focus-ring-inner={showText} class="toc-ring-text block w-fit max-w-full">
					<span
						data-toc-text
						class="text-[0.8125rem] leading-snug"
						class:text-text-strong={i === activeIndex}
						class:text-text-soft={i !== activeIndex}
						style="height: 0; opacity: 0"
					>
						{entry.text}
					</span>
				</span>
			</button>
		{/each}
	</nav>
{/if}

<style>
	/* The article is at most 45rem wide. Keep the ToC 1.5rem outside that box,
	and preserve the same viewport gutter at the lg breakpoint. */
	.toc-nav {
		right: calc(50% + 24rem);
		width: min(12rem, calc(50vw - 25.5rem));
	}

	[data-toc-text] {
		display: -webkit-box;
		max-width: 100%;
		overflow: hidden;
		overflow-wrap: anywhere;
		white-space: normal;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}

</style>
