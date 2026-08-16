<script lang="ts">
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import { hierarchy, treemap } from 'd3-hierarchy';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import { URLS } from '@canmi/urls';
	import { langColor, parseTokei, type LangStat } from './tokei';
	import type { TokeiView } from '$lib/content/types';

	let {
		source,
		title,
		view = 'treemap',
	}: { source: string; title: string; view?: TokeiView } = $props();
	let chart = $state<HTMLDivElement>();
	let tip = $state<{ x: number; y: number; stat: LangStat }>();

	const stats = $derived(parseTokei(source));
	const sorted = $derived(stats.toSorted((a, b) => b.lines - a.lines));
	const totals = $derived(
		stats.reduce(
			(sum, stat) => ({
				files: sum.files + stat.files,
				lines: sum.lines + stat.lines,
				code: sum.code + stat.code,
				comments: sum.comments + stat.comments,
				blanks: sum.blanks + stat.blanks,
			}),
			{ files: 0, lines: 0, code: 0, comments: 0, blanks: 0 },
		),
	);

	const FUNCTION_COLORS = {
		code: '#3178c6',
		comments: '#7c6ede',
		blanks: '#b0ada6',
	} as const;

	const TREE_WIDTH = 700;
	const TREE_HEIGHT = 420;
	const clipPrefix = 'tokei-language';
	type TreeTile = { stat: LangStat; x: number; y: number; width: number; height: number };
	const treeTiles: TreeTile[] = $derived.by(() => {
		if (sorted.length === 0) return [];
		type Node = { children?: LangStat[] };
		const root = hierarchy<Node>({ children: sorted })
			.sum((node) => (node as unknown as LangStat).lines ?? 0)
			// oxlint-disable-next-line unicorn/no-array-sort -- d3 hierarchy requires in-place ordering
			.sort((a, b) => (b.value ?? 0) - (a.value ?? 0));
		const laid = treemap<Node>().size([TREE_WIDTH, TREE_HEIGHT]).padding(2).round(true)(root);
		return laid.leaves().map((leaf) => ({
			stat: leaf.data as unknown as LangStat,
			x: leaf.x0,
			y: leaf.y0,
			width: leaf.x1 - leaf.x0,
			height: leaf.y1 - leaf.y0,
		}));
	});

	const BAR_WIDTH = 700;
	const BAR_HEIGHT = 400;
	const BAR_MARGIN = { top: 20, right: 20, bottom: 90, left: 55 };
	const barInnerWidth = BAR_WIDTH - BAR_MARGIN.left - BAR_MARGIN.right;
	const barInnerHeight = BAR_HEIGHT - BAR_MARGIN.top - BAR_MARGIN.bottom;
	const maxLines = $derived(Math.max(0, ...sorted.map((stat) => stat.lines)));
	const barStep = $derived(sorted.length === 0 ? 0 : barInnerWidth / sorted.length);
	const barWidth = $derived(barStep * 0.75);
	const y = (value: number) =>
		maxLines === 0 ? barInnerHeight : barInnerHeight - (value / maxLines) * barInnerHeight;
	const ticks = $derived(Array.from({ length: 6 }, (_, index) => (maxLines * index) / 5));

	function compact(value: number): string {
		if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`;
		if (value >= 10_000) return `${Math.round(value / 1000)}K`;
		if (value >= 1_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, '')}K`;
		return value.toString();
	}

	function percent(part: number, total: number): number {
		return total === 0 ? 0 : Math.round((part / total) * 100);
	}

	function pointerTip(event: PointerEvent, stat: LangStat) {
		if (!chart) return;
		const bounds = chart.getBoundingClientRect();
		tip = { x: event.clientX - bounds.left, y: event.clientY - bounds.top, stat };
	}

</script>

{#if stats.length > 0}
	<div class="code-stats">
		<div class="chart-area">
			{#if view === 'table'}
				<div class="table-wrap">
					<table>
						<thead>
							<tr>
								<th class="left">Language</th>
								<th>Files</th>
								<th>Lines</th>
								<th>Code</th>
								<th>Comments</th>
								<th>Blanks</th>
								<th class="left breakdown-heading">Breakdown</th>
							</tr>
						</thead>
						<tbody>
							{#each sorted as stat (stat.lang)}
								<tr>
									<td class="language-cell">
										<span
											class="language-dot"
											style="background: {langColor(stat.lang)}"
											aria-hidden="true"
										></span>
										{stat.lang}
									</td>
									<td>{stat.files.toLocaleString('en-US')}</td>
									<td title={stat.lines.toLocaleString('en-US')}>{compact(stat.lines)}</td>
									<td title={stat.code.toLocaleString('en-US')}>{compact(stat.code)}</td>
									<td title={stat.comments.toLocaleString('en-US')}>{compact(stat.comments)}</td>
									<td title={stat.blanks.toLocaleString('en-US')}>{compact(stat.blanks)}</td>
									<td>
										<div class="breakdown" aria-label="{percent(stat.code, stat.lines)}% code, {percent(
											stat.comments,
											stat.lines,
										)}% comments">
											<span style="width: {percent(stat.code, stat.lines)}%; background: {FUNCTION_COLORS.code}"></span>
											<span style="width: {percent(stat.comments, stat.lines)}%; background: {FUNCTION_COLORS.comments}"></span>
											<span style="width: {Math.max(0, 100 - percent(stat.code, stat.lines) - percent(stat.comments, stat.lines))}%; background: {FUNCTION_COLORS.blanks}"></span>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<div
					bind:this={chart}
					class="chart"
					role="presentation"
					onpointerleave={() => (tip = undefined)}
				>
					{#if view === 'bar'}
						<svg viewBox="0 0 {BAR_WIDTH} {BAR_HEIGHT}" role="img" aria-label="{title}: lines by language">
							<g transform="translate({BAR_MARGIN.left} {BAR_MARGIN.top})">
								{#each ticks as tick (tick)}
									{@const tickY = y(tick)}
									<line x1="0" x2={barInnerWidth} y1={tickY} y2={tickY} class="grid-line" />
									<text x="-8" y={tickY + 4} text-anchor="end" class="axis-label">{compact(tick)}</text>
								{/each}
								{#each sorted as stat, index (stat.lang)}
									{@const barX = index * barStep + (barStep - barWidth) / 2}
									{@const codeHeight = maxLines === 0 ? 0 : (stat.code / maxLines) * barInnerHeight}
									{@const commentHeight = maxLines === 0 ? 0 : (stat.comments / maxLines) * barInnerHeight}
									{@const blankHeight = maxLines === 0 ? 0 : (stat.blanks / maxLines) * barInnerHeight}
									<g
										role="img"
										aria-label="{stat.lang}: {stat.lines.toLocaleString('en-US')} lines"
										onpointerenter={(event) => pointerTip(event, stat)}
										onpointermove={(event) => pointerTip(event, stat)}
									>
										<rect x={barX} y={barInnerHeight - codeHeight} width={barWidth} height={codeHeight} fill={FUNCTION_COLORS.code} rx="1" />
										<rect x={barX} y={barInnerHeight - codeHeight - commentHeight} width={barWidth} height={commentHeight} fill={FUNCTION_COLORS.comments} rx="1" />
										<rect x={barX} y={barInnerHeight - codeHeight - commentHeight - blankHeight} width={barWidth} height={blankHeight} fill={FUNCTION_COLORS.blanks} rx="1" />
										{#if stat.lines >= 800}
											<text x={barX + barWidth / 2} y={y(stat.lines) - 5} text-anchor="middle" class="axis-label">{compact(stat.lines)}</text>
										{/if}
									</g>
									<text transform="translate({barX + barWidth / 2} {barInnerHeight + 10}) rotate(-45)" text-anchor="end" class="axis-label">{stat.lang}</text>
								{/each}
							</g>
						</svg>
					{:else}
						<svg viewBox="0 0 {TREE_WIDTH} {TREE_HEIGHT}" role="img" aria-label="{title}: {stats.length} languages, {totals.lines.toLocaleString('en-US')} lines">
							<defs>
								{#each treeTiles as tile, index (tile.stat.lang)}
									<clipPath id="{clipPrefix}-{index}">
										<rect
											x={tile.x + 6}
											y={tile.y}
											width={Math.max(0, tile.width - 18)}
											height={tile.height}
										/>
									</clipPath>
								{/each}
							</defs>
							{#each treeTiles as tile, index (tile.stat.lang)}
								<g
									role="img"
									aria-label="{tile.stat.lang}: {tile.stat.lines.toLocaleString('en-US')} lines"
									onpointerenter={(event) => pointerTip(event, tile.stat)}
									onpointermove={(event) => pointerTip(event, tile.stat)}
								>
									<rect x={tile.x} y={tile.y} width={tile.width} height={tile.height} fill={langColor(tile.stat.lang)} opacity="0.82" rx={Math.min(4, Math.min(tile.width, tile.height) * 0.3)} />
									{#if tile.width > 64 && tile.height > 30}
										<text
											x={tile.x + 6}
											y={tile.y + 16}
											class="tile-name"
											clip-path="url(#{clipPrefix}-{index})">{tile.stat.lang}</text
										>
										{#if tile.height > 40}<text
												x={tile.x + 6}
												y={tile.y + 28}
												class="tile-size"
												clip-path="url(#{clipPrefix}-{index})"
												>{compact(tile.stat.lines)} lines</text
											>{/if}
									{/if}
									{#if tile.width > 8 && tile.height > 30}
										<rect x={tile.x + tile.width - 8} y={tile.y + 4} width="4" height={tile.height - 8} fill="rgba(0,0,0,0.15)" rx="2" />
										<rect x={tile.x + tile.width - 8} y={tile.y + 4} width="4" height={Math.max(1, (tile.height - 8) * (tile.stat.code / tile.stat.lines))} fill="rgba(255,255,255,0.6)" rx="2" />
									{/if}
								</g>
							{/each}
						</svg>
					{/if}

					{#if tip}
						{@const codePercent = percent(tip.stat.code, tip.stat.lines)}
						{@const commentPercent = percent(tip.stat.comments, tip.stat.lines)}
						{@const blankPercent = Math.max(0, 100 - codePercent - commentPercent)}
						<div
							class="tooltip shadow-sm"
							style="left: calc({remFromMeasuredPixels(tip.x)} + 1rem); top: calc({remFromMeasuredPixels(
								tip.y,
							)} + 1rem)"
						>
							<div class="tooltip-head">
								<span class="tooltip-dot" style="background: {langColor(tip.stat.lang)}" aria-hidden="true"></span>
								<span class="tooltip-title">{tip.stat.lang}</span>
								<span class="tooltip-count">{tip.stat.lines.toLocaleString('en-US')} lines</span>
							</div>
							<div class="tooltip-bar">
								<span style="width: {codePercent}%; background: {FUNCTION_COLORS.code}"></span>
								<span style="width: {commentPercent}%; background: {FUNCTION_COLORS.comments}"></span>
								<span style="width: {blankPercent}%; background: {FUNCTION_COLORS.blanks}"></span>
							</div>
							<div class="tooltip-grid">
								<span class="muted">Files</span><span>{tip.stat.files.toLocaleString('en-US')}</span>
								<span style="color: {FUNCTION_COLORS.code}">Code</span><span>{tip.stat.code.toLocaleString('en-US')} <small>{codePercent}%</small></span>
								<span style="color: {FUNCTION_COLORS.comments}">Comments</span><span>{tip.stat.comments.toLocaleString('en-US')} <small>{commentPercent}%</small></span>
								<span style="color: {FUNCTION_COLORS.blanks}">Blanks</span><span>{tip.stat.blanks.toLocaleString('en-US')} <small>{blankPercent}%</small></span>
							</div>
							{#if tip.stat.nested.length > 0}
								<div class="nested">
									{#each tip.stat.nested as nested (nested.lang)}
										<span><i style="background: {langColor(nested.lang)}"></i>{nested.lang} {compact(nested.lines)}</span>
									{/each}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<div class="bottom-bar">
			<div class="legend" aria-label="Line kinds">
				{#each Object.entries(FUNCTION_COLORS) as [kind, color] (kind)}
					<span><i style="background: {color}" aria-hidden="true"></i>{kind.charAt(0).toUpperCase() + kind.slice(1)}</span>
				{/each}
			</div>
			<div class="summary">
				<span><span class="muted">Total files</span> <b>{compact(totals.files)}</b></span>
				<span><span class="muted">Total lines</span> <b>{compact(totals.lines)}</b></span>
				<span><span class="muted">Code lines</span> <b>{compact(totals.code)}</b></span>
				<span><span class="muted">Comment ratio</span> <b>{percent(totals.comments, totals.lines)}%</b></span>
				<a class="focus-link" href="{URLS.external.github.web}/XAMPPRocky/tokei" target="_blank" rel="noopener noreferrer">tokei<ArrowUpRight class="size-2.5" strokeWidth={2} aria-hidden="true" /></a>
			</div>
		</div>
	</div>
{/if}

<style>
	.code-stats { margin-block: 1.8em; }
	.chart-area { min-height: 6.25rem; }
	.chart { position: relative; }
	.chart svg { display: block; width: 100%; height: auto; }
	.tile-name { fill: white; font-size: 0.75rem; font-weight: 500; pointer-events: none; }
	.tile-size { fill: rgb(255 255 255 / 75%); font-size: 0.625rem; pointer-events: none; }
	.grid-line { stroke: var(--color-border); stroke-width: 1; }
	.axis-label { fill: var(--color-text-soft); font-size: 0.6875rem; }
	.bottom-bar { display: flex; margin-top: 0.625rem; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 0.375rem 0.75rem; }
	.legend { display: flex; gap: 0.75rem; color: var(--color-text-soft); font-size: 0.75rem; }
	.legend span { display: flex; align-items: center; gap: 0.25rem; }
	.legend i, .nested i { display: inline-block; width: 0.625rem; height: 0.625rem; border-radius: 0.125rem; }
	.summary { display: flex; flex-wrap: wrap; gap: 0.4rem 1rem; font-size: 0.75rem; }
	.summary b { color: var(--color-text-strong); font-weight: 500; }
	.summary a { display: inline-flex; align-items: center; gap: 0.0625rem; color: var(--color-text-soft); font-size: 0.6875rem; text-decoration: none; transition: color 140ms ease; }
	.summary a:hover { color: var(--color-text-strong); }
	.muted { color: var(--color-text-soft); }
	.tooltip { position: absolute; z-index: 10; min-width: 11.25rem; max-width: 16.25rem; border: 0.0625rem solid var(--color-border); border-radius: 0.375rem; background: var(--color-paper); padding: 0.4rem 0.55rem; color: var(--color-text); font-size: 0.75rem; line-height: 1.4; pointer-events: none; }
	.tooltip-head { display: flex; margin-bottom: 0.3rem; align-items: center; gap: 0.3rem; }
	.tooltip-dot { width: 0.5rem; height: 0.5rem; flex-shrink: 0; border-radius: 0.125rem; }
	.tooltip-title { font-weight: 560; }
	.tooltip-count { margin-left: auto; color: var(--color-text-soft); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.6875rem; }
	.tooltip-bar { display: flex; height: 0.1875rem; margin-bottom: 0.3rem; overflow: hidden; border-radius: 0.09375rem; }
	.tooltip-grid { display: grid; grid-template-columns: auto 1fr; gap: 0 0.5rem; font-size: 0.6875rem; }
	.tooltip-grid > :nth-child(even) { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; text-align: right; }
	.tooltip-grid small { color: var(--color-text-soft); font-size: 0.65625rem; }
	.nested { display: flex; margin-top: 0.25rem; padding-top: 0.25rem; flex-wrap: wrap; gap: 0.15rem 0.5rem; border-top: 0.0625rem solid var(--color-border); }
	.nested span { display: inline-flex; align-items: center; gap: 0.2rem; color: var(--color-text-soft); font-size: 0.65625rem; white-space: nowrap; }
	.nested i { width: 0.375rem; height: 0.375rem; }
	.table-wrap { overflow-x: auto; }
	table { width: 100%; border-collapse: collapse; color: var(--color-text-strong); font-size: 0.8125rem; }
	th { border-bottom: 0.0625rem solid var(--color-border); padding: 0.5rem 0.375rem; color: var(--color-text-soft); font-size: 0.75rem; font-weight: 400; text-align: right; }
	td { border-bottom: 0.03125rem solid var(--color-border); padding: 0.5rem 0.375rem; text-align: right; }
	.left, .language-cell { text-align: left; }
	.language-cell { font-weight: 500; white-space: nowrap; }
	.language-dot { display: inline-block; width: 0.625rem; height: 0.625rem; margin-right: 0.375rem; border-radius: 0.125rem; vertical-align: middle; }
	.breakdown-heading { min-width: 7.5rem; }
	.breakdown { display: flex; min-width: 6.25rem; height: 0.875rem; overflow: hidden; border-radius: 0.1875rem; }
	@media (max-width: 40rem) { .summary { display: none; } }
	@media (prefers-reduced-motion: reduce) { .summary a { transition: none; } }
</style>
