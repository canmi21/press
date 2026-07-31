<script lang="ts">
	import { measureNaturalWidth, prepareWithSegments } from '@chenglou/pretext';
	import { animate } from 'motion';
	import ArticleCard from '$lib/article-card.svelte';

	let {
		articles
	}: {
		articles: {
			title: string;
			subtitle: string;
			created: string;
			path: string;
			paragraphs: string[];
		}[];
	} = $props();

	// Bar widths map straight into the range taken from the first frame's bars:
	// shortest (12) to longest (32), no quantization. The title stays within the
	// first half so it reads as a short heading.
	const BODY_MIN = 12;
	const BODY_MAX = 32;
	const TITLE_MIN = 11; // compressed title range so short titles aren't tiny stubs
	const TITLE_MAX = 16; // capped at half the body max
	const TITLE_GAP = 8; // title-to-body gap; reused as the single body separator
	const LINE_GAP = 4; // the body's other two gaps
	// The first frame is the ideal-looking shape; the body only leans toward real
	// proportions by BLEND, so icons stay pretty while differing a little.
	const IDEAL_BODY = [32, 24, 20, 12];
	const BLEND = 0.35;
	const SPRING = { type: 'spring' as const, stiffness: 320, damping: 30 };
	const GAP_EASE = 'cubic-bezier(0.22, 1, 0.36, 1)';
	const GAP_MS = 560;

	// Bar dimensions above are authored against the 16px root default. We animate in px
	// (scaled to the live root so springs stay smooth) but commit the resting size in
	// rem, so bars track the root font for responsive scaling.
	const REM_BASE = 16;
	const rootFontPx = () =>
		parseFloat(getComputedStyle(document.documentElement).fontSize) || REM_BASE;
	const toRem = (n: number) => `${n / REM_BASE}rem`;
	const toScaledPx = (n: number, root: number) => (n / REM_BASE) * root;

	let listEl = $state<HTMLElement>();

	function lerp(min: number, max: number, t: number): number {
		return min + Math.min(1, Math.max(0, t)) * (max - min);
	}

	// Deterministic per-article pick of which body gap (1, 2 or 3) is the separator,
	// so the split varies between articles but is stable for a given one.
	function separatorGap(seed: string): number {
		let h = 0;
		for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) >>> 0;
		return 1 + (h % 3);
	}

	function fontOf(el: Element): string {
		const cs = getComputedStyle(el);
		return `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
	}

	// Up to four clauses from the leading paragraphs, split on sentence punctuation.
	function clauses(paragraphs: string[]): string[] {
		const out: string[] = [];
		for (const p of paragraphs) {
			out.push(
				...p
					.split(/[。．.!?！？，,;；\n]+/)
					.map((s) => s.trim())
					.filter((s) => s.length >= 2)
			);
			if (out.length >= 4) break;
		}
		return out.slice(0, 4);
	}

	// Titles are normalized list-wide (so they vary against each other) into the
	// title half-range. Each article's body is normalized against its own shortest
	// and longest clause, spanning the full body range. Mirrors the ToC: the markup
	// ships a baked first frame and we spring each bar to the computed shape.
	$effect(() => {
		if (!listEl) return;
		const icons = listEl.querySelectorAll<HTMLElement>('[data-article-icon]');
		if (icons.length !== articles.length || icons.length === 0) return;

		const raf = requestAnimationFrame(() => {
			const root = rootFontPx();
			const font = fontOf(listEl?.querySelector('p') ?? document.body);
			const natural = (t: string) => measureNaturalWidth(prepareWithSegments(t, font));

			const titleW = articles.map((a) => natural(a.title));
			const tLo = Math.min(...titleW);
			const tHi = Math.max(...titleW);
			const bodyW = articles.map((a) => clauses(a.paragraphs).map(natural));

			icons.forEach((icon, ai) => {
				const bars = icon.querySelectorAll<HTMLElement>('[data-icon-bar]');
				if (bars.length !== 5) return;

				const tFill = tHi > tLo ? (titleW[ai] - tLo) / (tHi - tLo) : 0.5;
				const target = [{ width: Math.round(lerp(TITLE_MIN, TITLE_MAX, tFill)), gap: 0 }];

				const lines = bodyW[ai];
				const lo = lines.length ? Math.min(...lines) : 0;
				const hi = lines.length ? Math.max(...lines) : 1;
				const sep = separatorGap(articles[ai].path);
				for (let i = 0; i < 4; i++) {
					const w = lines[i];
					const content =
						w === undefined
							? IDEAL_BODY[i]
							: lerp(BODY_MIN, BODY_MAX, hi > lo ? (w - lo) / (hi - lo) : 0.7);
					// Lean the ideal shape toward the measured proportion by BLEND.
					const width = Math.round(IDEAL_BODY[i] * (1 - BLEND) + content * BLEND);
					// Title gap and the chosen body separator share TITLE_GAP; rest small.
					const gap = i === 0 || i === sep ? TITLE_GAP : LINE_GAP;
					target.push({ width, gap });
				}

				bars.forEach((bar, i) => {
					// Width springs via motion; marginTop is animated with the native
					// WAAPI because motion only snaps layout props (it tweens width but
					// jumps margin). Set the final margin as the base, then tween to it.
					// Both animate in scaled px but rest in rem (see toRem/toScaledPx).
					const widthPx = toScaledPx(target[i].width, root);
					animate(
						bar,
						{ width: widthPx },
						{ ...SPRING, onComplete: () => (bar.style.width = toRem(target[i].width)) }
					);
					const from = parseFloat(getComputedStyle(bar).marginTop) || 0;
					const toPx = toScaledPx(target[i].gap, root);
					bar.style.marginTop = toRem(target[i].gap);
					if (Math.abs(from - toPx) > 0.5) {
						bar.animate([{ marginTop: `${from}px` }, { marginTop: `${toPx}px` }], {
							duration: GAP_MS,
							easing: GAP_EASE
						});
					}
				});
			});
		});
		return () => cancelAnimationFrame(raf);
	});
</script>

<section bind:this={listEl} aria-label="Writing" class="mt-16">
	<h2 class="mb-3 font-medium text-text-strong">Writing</h2>
	<div>
		{#each articles as article (article.path)}
			<ArticleCard
				title={article.title}
				subtitle={article.subtitle}
				created={article.created}
				path={article.path}
			/>
		{/each}
	</div>
</section>
