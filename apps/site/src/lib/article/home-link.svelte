<script lang="ts">
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { animate } from 'motion';
	import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	const DEFAULT_TOP_REM = 6.75;
	const COLLAPSED_BAR_REM = 0.25;
	const HOME_SPRING = { type: 'spring' as const, stiffness: 600, damping: 32, mass: 0.85 };

	type AnimationControl = { stop: () => void };
	type TocState = 'collapsed' | 'expanded';

	let { locale }: { locale: LocaleCode } = $props();

	function titleCenter() {
		const title = document.querySelector<HTMLElement>('article h1');
		if (!title) return;
		const rect = title.getBoundingClientRect();
		const lineHeight = Number.parseFloat(getComputedStyle(title).lineHeight) || rect.height;
		// Scrolling the article must not move fixed navigation away from its default alignment.
		return rect.top + window.scrollY + lineHeight / 2;
	}

	function tocHeight(toc: HTMLElement, state: TocState, rootPixels: number) {
		const buttons = toc.querySelectorAll<HTMLElement>('[data-toc-button]');
		let height = 0;
		for (const button of buttons) {
			const buttonStyle = getComputedStyle(button);
			height +=
				(Number.parseFloat(buttonStyle.paddingTop) || 0) +
				(Number.parseFloat(buttonStyle.paddingBottom) || 0);
			if (state === 'collapsed') {
				height += COLLAPSED_BAR_REM * rootPixels;
				continue;
			}

			const label = button.querySelector<HTMLElement>('[data-toc-text]');
			if (!label) continue;
			const lineHeight = Number.parseFloat(getComputedStyle(label).lineHeight);
			if (!lineHeight) continue;
			const lines = Math.min(2, Math.max(1, Math.round(label.scrollHeight / lineHeight)));
			height += lineHeight * lines;
		}
		return height;
	}

	function followToc(node: HTMLElement, _locale: LocaleCode) {
		let frame = 0;
		let toc: HTMLElement | undefined;
		let currentOffset = 0;
		let rootPixels = DEFAULT_PIXELS_PER_REM;
		let endpoints: Record<TocState, number> = { collapsed: 0, expanded: 0 };
		let motion: AnimationControl | undefined;
		let state: TocState = 'collapsed';
		let stateChanges: MutationObserver | undefined;
		let insertion: MutationObserver | undefined;
		let destroyed = false;

		const render = (offset: number) => {
			currentOffset = offset;
			node.style.setProperty('--home-offset', remFromMeasuredPixels(offset, rootPixels));
		};

		const move = (target: number, immediate = false) => {
			motion?.stop();
			motion = undefined;
			if (
				immediate ||
				window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
				Math.abs(currentOffset - target) < 0.01
			) {
				render(target);
				return;
			}
			motion = animate(currentOffset, target, {
				...HOME_SPRING,
				onUpdate: render,
				onComplete: () => {
					render(target);
					motion = undefined;
				},
			});
		};

		const endpoint = (restingTop: number, height: number) => {
			const boundary = (window.innerHeight - height) / 2;
			// Title alignment yields only when the ToC consumes that space. See spec/styling.md.
			const target = Math.min(restingTop, Math.max(0, boundary) / 2);
			return target - DEFAULT_TOP_REM * rootPixels;
		};

		const calibrate = (immediate = true) => {
			rootPixels =
				Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
				DEFAULT_PIXELS_PER_REM;
			const restingTop = titleCenter() ?? window.innerHeight / 4;
			if (!toc) {
				endpoints = { collapsed: 0, expanded: 0 };
			} else {
				endpoints = {
					collapsed: endpoint(restingTop, tocHeight(toc, 'collapsed', rootPixels)),
					expanded: endpoint(restingTop, tocHeight(toc, 'expanded', rootPixels)),
				};
			}
			move(endpoints[state], immediate);
		};

		const connect = () => {
			const next = document.querySelector<HTMLElement>('.toc-nav');
			if (next === toc) return;
			stateChanges?.disconnect();
			toc = next ?? undefined;
			state = toc?.classList.contains('revealed') ? 'expanded' : 'collapsed';
			if (toc) {
				stateChanges = new MutationObserver(() => {
					if (!toc) return;
					const nextState = toc.classList.contains('revealed') ? 'expanded' : 'collapsed';
					if (nextState === state) return;
					state = nextState;
					move(endpoints[state]);
				});
				stateChanges.observe(toc, { attributes: true, attributeFilter: ['class'] });
			}
			calibrate();
		};

		const scheduleCalibration = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => calibrate());
		};
		insertion = new MutationObserver((records) => {
			const next = document.querySelector<HTMLElement>('.toc-nav');
			if (next !== toc) {
				connect();
				return;
			}
			if (toc && records.some(({ target }) => toc?.contains(target))) {
				scheduleCalibration();
			}
		});
		insertion.observe(document.body, { childList: true, characterData: true, subtree: true });
		window.addEventListener('resize', scheduleCalibration);
		connect();
		document.fonts.ready.then(() => {
			if (!destroyed) calibrate();
		});

		return {
			update(_nextLocale: LocaleCode) {
				calibrate();
			},
			destroy() {
				destroyed = true;
				cancelAnimationFrame(frame);
				motion?.stop();
				stateChanges?.disconnect();
				insertion?.disconnect();
				window.removeEventListener('resize', scheduleCalibration);
			},
		};
	}
</script>

<div
	use:followToc={locale}
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
