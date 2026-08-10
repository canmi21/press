<script lang="ts">
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import { animate } from 'motion';
	import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';
	import { homeCenter } from './rail';

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
		let article: HTMLElement | undefined;
		let articleTop = 0;
		let articleEnd = Number.POSITIVE_INFINITY;
		let restingCenter = 0;
		let renderedOffset = '';
		let heights: Record<TocState, number> = { collapsed: 0, expanded: 0 };
		let progress = 0;
		let rootPixels = DEFAULT_PIXELS_PER_REM;
		let endpoints: Record<TocState, number> = { collapsed: 0, expanded: 0 };
		let motion: AnimationControl | undefined;
		let state: TocState = 'collapsed';
		let articleResize: ResizeObserver | undefined;
		let stateChanges: MutationObserver | undefined;
		let insertion: MutationObserver | undefined;
		let destroyed = false;
		let calibrateNext = false;

		const render = (nextProgress: number) => {
			progress = nextProgress;
			const offset =
				endpoints.collapsed + (endpoints.expanded - endpoints.collapsed) * nextProgress;
			const rendered = remFromMeasuredPixels(offset, rootPixels);
			if (rendered === renderedOffset) return;
			renderedOffset = rendered;
			node.style.setProperty('--home-offset', rendered);
		};

		const move = (nextState: TocState) => {
			const target = nextState === 'expanded' ? 1 : 0;
			motion?.stop();
			motion = undefined;
			if (
				window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
				Math.abs(progress - target) < 0.001
			) {
				render(target);
				return;
			}
			motion = animate(progress, target, {
				...HOME_SPRING,
				onUpdate: render,
				onComplete: () => {
					render(target);
					motion = undefined;
				},
			});
		};

		const endpoint = (restingTop: number, height: number) => {
			const target = homeCenter(
				restingTop,
				window.innerHeight,
				height,
				articleEnd - window.scrollY,
			);
			return target - DEFAULT_TOP_REM * rootPixels;
		};

		const position = () => {
			if (!toc) {
				endpoints = { collapsed: 0, expanded: 0 };
			} else {
				endpoints = {
					collapsed: endpoint(restingCenter, heights.collapsed),
					expanded: endpoint(restingCenter, heights.expanded),
				};
			}
			render(progress);
		};

		const calibrate = () => {
			rootPixels =
				Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
				DEFAULT_PIXELS_PER_REM;
			restingCenter = titleCenter() ?? window.innerHeight / 4;
		if (toc) {
				heights = {
					collapsed: tocHeight(toc, 'collapsed', rootPixels),
					expanded: tocHeight(toc, 'expanded', rootPixels),
				};
			}
			if (article) {
				const rect = article.getBoundingClientRect();
				articleTop = rect.top + window.scrollY;
				articleEnd = rect.bottom + window.scrollY;
			}
			position();
		};

		const connect = () => {
			const nextToc = document.querySelector<HTMLElement>('.toc-nav') ?? undefined;
			const nextArticle = document.querySelector<HTMLElement>('article') ?? undefined;
			if (nextToc === toc && nextArticle === article) return;

			if (nextToc !== toc) {
				motion?.stop();
				motion = undefined;
				stateChanges?.disconnect();
				toc = nextToc;
				state = toc?.classList.contains('revealed') ? 'expanded' : 'collapsed';
				progress = state === 'expanded' ? 1 : 0;
				if (toc) {
					stateChanges = new MutationObserver(() => {
						if (!toc) return;
						const nextState = toc.classList.contains('revealed') ? 'expanded' : 'collapsed';
						if (nextState === state) return;
						state = nextState;
						move(state);
					});
					stateChanges.observe(toc, { attributes: true, attributeFilter: ['class'] });
				}
			}

			if (nextArticle !== article) {
				articleResize?.disconnect();
				article = nextArticle;
				articleTop = 0;
				articleEnd = Number.POSITIVE_INFINITY;
				if (article && typeof ResizeObserver !== 'undefined') {
					articleResize = new ResizeObserver(([entry]) => {
						if (!entry) return;
						const height = entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height;
						articleEnd = articleTop + height;
						position();
					});
					articleResize.observe(article);
				}
			}
			calibrate();
		};

		const schedule = (needsCalibration = false) => {
			calibrateNext ||= needsCalibration;
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => {
				if (calibrateNext) {
					calibrateNext = false;
					calibrate();
				} else {
					position();
				}
			});
		};
		insertion = new MutationObserver((records) => {
			const nextToc = document.querySelector<HTMLElement>('.toc-nav') ?? undefined;
			const nextArticle = document.querySelector<HTMLElement>('article') ?? undefined;
			if (nextToc !== toc || nextArticle !== article) {
				connect();
				return;
			}
			if (toc && records.some(({ target }) => toc?.contains(target))) {
				schedule(true);
			}
		});
		insertion.observe(document.body, { childList: true, characterData: true, subtree: true });
		const onResize = () => schedule(true);
		const onScroll = () => schedule();
		window.addEventListener('resize', onResize);
		window.addEventListener('scroll', onScroll, { passive: true });
		connect();
		document.fonts.ready.then(() => {
			if (!destroyed) calibrate();
		});

		return {
			update(_nextLocale: LocaleCode) {
				schedule(true);
			},
			destroy() {
				destroyed = true;
				cancelAnimationFrame(frame);
				motion?.stop();
				articleResize?.disconnect();
				stateChanges?.disconnect();
				insertion?.disconnect();
				window.removeEventListener('resize', onResize);
				window.removeEventListener('scroll', onScroll);
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
