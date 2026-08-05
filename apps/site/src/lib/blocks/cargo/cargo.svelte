<script lang="ts">
	import './palette.css';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import { hierarchy, treemap, treemapBinary } from 'd3-hierarchy';
	import { remFromMeasuredPixels } from '$lib/client/units';
	import {
		KIND_COLORS,
		crateColors,
		dependencyItems,
		formatBytes,
		kindColor,
		type DependencyItem,
	} from './cargo';
	import type { CargoView, CrateDep, CrateRecord } from '$lib/content/types';

	let { crate, view = 'treemap' }: { crate: CrateRecord; view?: CargoView } = $props();
	let chart = $state<HTMLDivElement>();
	let tip = $state<{ x: number; y: number; dep: CrateDep }>();

	const items = $derived(dependencyItems(crate.deps));
	const colors = $derived(crateColors(crate.deps));
	const direct = $derived(crate.deps.filter((dep) => dep.depth === 0).length);
	const features = $derived(Object.keys(crate.features).length);
	const sorted = $derived(items.toSorted((a, b) => (b.dep.size ?? 0) - (a.dep.size ?? 0)));

	const WIDTH = 700;
	const HEIGHT = 420;
	const clipPrefix = $derived(`cargo-${crate.name.replace(/[^a-z0-9_-]/gi, '-')}`);
	type Tile = DependencyItem & { x: number; y: number; width: number; height: number };
	const tiles: Tile[] = $derived.by(() => {
		const sized = items.filter(({ dep }) => (dep.size ?? 0) > 0);
		if (sized.length === 0) return [];
		type Node = { children?: DependencyItem[] };
		const root = hierarchy<Node>({ children: sized })
			.sum((node) => (node as unknown as DependencyItem).dep?.size ?? 0)
			// oxlint-disable-next-line unicorn/no-array-sort -- d3 hierarchy requires in-place ordering
			.sort((a, b) => (b.value ?? 0) - (a.value ?? 0));
		const laid = treemap<Node>()
			.tile(treemapBinary)
			.size([WIDTH, HEIGHT])
			.padding(2)
			.round(true)(root);
		return laid.leaves().map((leaf) => {
			const item = leaf.data as unknown as DependencyItem;
			return {
				dep: item.dep,
				key: item.key,
				x: leaf.x0,
				y: leaf.y0,
				width: leaf.x1 - leaf.x0,
				height: leaf.y1 - leaf.y0,
			};
		});
	});

	function pointerTip(event: PointerEvent, dep: CrateDep) {
		if (!chart) return;
		const bounds = chart.getBoundingClientRect();
		tip = { x: event.clientX - bounds.left, y: event.clientY - bounds.top, dep };
	}

	function focusTip(event: FocusEvent, dep: CrateDep) {
		if (!chart || !(event.currentTarget instanceof SVGElement)) return;
		const bounds = chart.getBoundingClientRect();
		const tile = event.currentTarget.getBoundingClientRect();
		tip = {
			x: tile.left + tile.width / 2 - bounds.left,
			y: tile.top + tile.height / 2 - bounds.top,
			dep,
		};
	}
</script>

<div class="cargo-widget">
	<div class="chart-area">
		{#if view === 'table'}
			<div class="table-wrap">
				<table>
					<thead>
						<tr>
							<th class="left">Crate</th>
							<th>Version</th>
							<th>Kind</th>
							<th>Depth</th>
							<th>Size</th>
						</tr>
					</thead>
					<tbody>
						{#each sorted as item (item.key)}
							<tr>
								<td class="name-cell">
									<span
										class="crate-dot"
										style="background: {colors.get(item.dep.name) ?? '#888'}"
										aria-hidden="true"
									></span>
									{item.dep.name}
									{#if item.dep.optional}<span class="optional">opt</span>{/if}
								</td>
								<td>{item.dep.version}</td>
								<td>
									<span
										class="kind-dot"
										style="background: {kindColor(item.dep)}"
										aria-hidden="true"
									></span>
									{item.dep.kind}
								</td>
								<td>{item.dep.depth === 0 ? 'direct' : item.dep.depth}</td>
								<td>{item.dep.size == null ? '-' : formatBytes(item.dep.size)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else if tiles.length === 0}
			<p class="empty">No dependency size data available.</p>
		{:else}
			<div
				bind:this={chart}
				class="chart"
				role="presentation"
				onpointerleave={() => (tip = undefined)}
			>
				<svg
					viewBox="0 0 {WIDTH} {HEIGHT}"
					role="img"
					aria-label="{crate.name} {crate.version}: {crate.deps.length} dependencies, {formatBytes(
						crate.total_dep_size,
					)} in total"
				>
					<defs>
						{#each tiles as tile, index (tile.key)}
							<clipPath id="{clipPrefix}-{index}">
								<rect
									x={tile.x + 5}
									y={tile.y}
									width={Math.max(0, tile.width - 10)}
									height={tile.height}
								/>
							</clipPath>
						{/each}
					</defs>
					{#each tiles as tile, index (tile.key)}
						<a
							href="https://crates.io/crates/{tile.dep.name}/{tile.dep.version}"
							target="_blank"
							rel="noopener noreferrer"
							aria-label="{tile.dep.name} {tile.dep.version}, {tile.dep.depth === 0
								? 'direct'
								: `transitive depth ${tile.dep.depth}`}, {formatBytes(tile.dep.size ?? 0)}"
							onpointerenter={(event) => pointerTip(event, tile.dep)}
							onpointermove={(event) => pointerTip(event, tile.dep)}
							onfocus={(event) => focusTip(event, tile.dep)}
							onblur={() => (tip = undefined)}
						>
							<rect
								x={tile.x}
								y={tile.y}
								width={tile.width}
								height={tile.height}
								fill={colors.get(tile.dep.name) ?? '#888'}
								opacity={tile.dep.depth === 0 ? 0.88 : 0.65}
								rx={Math.min(4, Math.min(tile.width, tile.height) * 0.3)}
							/>
							{#if tile.dep.optional && tile.width > 6 && tile.height > 6}
								<rect
									x={tile.x + 0.5}
									y={tile.y + 0.5}
									width={tile.width - 1}
									height={tile.height - 1}
									fill="none"
									stroke="rgba(255,255,255,0.4)"
									stroke-dasharray="3 2"
									rx={Math.min(4, Math.min(tile.width, tile.height) * 0.3)}
								/>
							{/if}
							{#if tile.width > 58 && tile.height > 28}
								<text
									x={tile.x + 5}
									y={tile.y + 15}
									class="tile-name"
									clip-path="url(#{clipPrefix}-{index})">{tile.dep.name}</text
								>
								{#if tile.height > 38}
									<text
										x={tile.x + 5}
										y={tile.y + 28}
										class="tile-size"
										clip-path="url(#{clipPrefix}-{index})"
										>{formatBytes(tile.dep.size ?? 0)}</text
									>
								{/if}
							{/if}
						</a>
					{/each}
				</svg>

				{#if tip}
					<div
						class="tooltip shadow-sm"
						style="left: calc({remFromMeasuredPixels(tip.x)} + 1rem); top: calc({remFromMeasuredPixels(
							tip.y,
						)} + 1rem)"
					>
						<div class="tooltip-head">
							<span
								class="tooltip-dot"
								style="background: {kindColor(tip.dep)}"
								aria-hidden="true"
							></span>
							<span class="tooltip-title">{tip.dep.name}</span>
							<span class="tooltip-count">{tip.dep.version}</span>
						</div>
						<div class="tooltip-grid">
							<span class="muted">Kind</span>
							<span>{tip.dep.kind}{tip.dep.optional ? ' (optional)' : ''}</span>
							<span class="muted">Size</span>
							<span>{tip.dep.size == null ? 'unknown' : formatBytes(tip.dep.size)}</span>
							<span class="muted">Depth</span>
							<span>{tip.dep.depth === 0 ? 'direct' : `transitive (${tip.dep.depth})`}</span>
							{#if tip.dep.target}
								<span class="muted">Target</span>
								<span>{tip.dep.target}</span>
							{/if}
							{#if tip.dep.features.length > 0}
								<span class="muted">Features</span>
								<span>{tip.dep.features.length <= 3
										? tip.dep.features.join(', ')
										: `${tip.dep.features.slice(0, 3).join(', ')} +${tip.dep.features.length - 3}`}</span
								>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<div class="footer">
		<div class="legend" aria-label="Dependency kinds">
			{#each Object.entries(KIND_COLORS) as [kind, color] (kind)}
				<span class="legend-item">
					<span class="legend-dot" style="background: {color}" aria-hidden="true"></span>
					{kind.charAt(0).toUpperCase() + kind.slice(1)}
				</span>
			{/each}
		</div>
		<div class="footer-right">
			{#if features > 0}<span><span class="muted">Features</span> <b>{features}</b></span>{/if}
			<span><span class="muted">Deps</span> <b>{direct}+{crate.deps.length - direct}</b></span>
			<span><span class="muted">Size</span> <b>{formatBytes(crate.total_dep_size)}</b></span>
			<span class="links">
				{#each [
					[`https://crates.io/crates/${crate.name}`, 'crates.io'],
					[`https://lib.rs/crates/${crate.name}`, 'lib.rs'],
					[`https://docs.rs/${crate.name}`, 'docs.rs'],
				] as [href, label] (label)}
					<a class="focus-link" {href} target="_blank" rel="noopener noreferrer">
						{label}<ArrowUpRight class="size-2.5" strokeWidth={2} aria-hidden="true" />
					</a>
				{/each}
			</span>
		</div>
	</div>
</div>

<style>
	.cargo-widget { margin-block: 1.8em; }
	.chart-area { min-height: 3.75rem; }
	.chart { position: relative; }
	.chart svg { display: block; width: 100%; height: auto; }
	.chart a { outline: none; }
	.chart a:focus-visible rect:first-child { stroke: white; stroke-width: 2; }
	.tile-name { fill: white; font-size: 0.6875rem; font-weight: 500; pointer-events: none; }
	.tile-size { fill: rgb(255 255 255 / 70%); font-size: 0.5625rem; pointer-events: none; }
	.empty { margin: 0; color: var(--color-text-soft); font-size: 0.8125rem; }
	.footer { display: flex; margin-top: 0.5rem; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 0.375rem 0.75rem; }
	.legend { display: flex; gap: 0.75rem; color: var(--color-text-soft); font-size: 0.75rem; }
	.legend-item { display: flex; align-items: center; gap: 0.25rem; }
	.legend-dot { display: inline-block; width: 0.625rem; height: 0.625rem; border-radius: 0.125rem; }
	.footer-right { display: flex; align-items: center; gap: 0.65rem; font-size: 0.75rem; white-space: nowrap; }
	.footer-right b { color: var(--color-text-strong); font-weight: 500; }
	.muted { color: var(--color-text-soft); }
	.links { display: flex; gap: 0.5rem; font-size: 0.6875rem; }
	.links a { display: inline-flex; align-items: center; gap: 0.0625rem; color: var(--color-text-soft); text-decoration: none; transition: color 140ms ease; }
	.links a:hover { color: var(--color-text-strong); }
	.tooltip { position: absolute; z-index: 10; min-width: 11.25rem; max-width: 16.25rem; border: 0.0625rem solid var(--color-border); border-radius: 0.375rem; background: var(--color-paper); padding: 0.4rem 0.55rem; color: var(--color-text); font-size: 0.75rem; line-height: 1.4; pointer-events: none; }
	.tooltip-head { display: flex; margin-bottom: 0.3rem; align-items: center; gap: 0.3rem; }
	.tooltip-dot { width: 0.5rem; height: 0.5rem; flex-shrink: 0; border-radius: 0.125rem; }
	.tooltip-title { font-weight: 560; }
	.tooltip-count { margin-left: auto; color: var(--color-text-soft); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.6875rem; }
	.tooltip-grid { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 0 0.5rem; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.6875rem; }
	.tooltip-grid > :nth-child(even) { overflow-wrap: anywhere; text-align: right; }
	.table-wrap { overflow-x: auto; }
	table { width: 100%; border-collapse: collapse; color: var(--color-text-strong); font-size: 0.8125rem; }
	th { border-bottom: 0.0625rem solid var(--color-border); padding: 0.375rem; color: var(--color-text-soft); font-size: 0.75rem; font-weight: 400; text-align: right; }
	td { border-bottom: 0.03125rem solid var(--color-border); padding: 0.375rem; text-align: right; }
	.left, .name-cell { text-align: left; }
	.name-cell { font-weight: 500; white-space: nowrap; }
	.crate-dot { display: inline-block; width: 0.5rem; height: 0.5rem; margin-right: 0.3125rem; border-radius: 0.125rem; vertical-align: middle; }
	.kind-dot { display: inline-block; width: 0.375rem; height: 0.375rem; margin-right: 0.1875rem; border-radius: 0.125rem; vertical-align: middle; }
	.optional { margin-left: 0.25rem; color: var(--color-text-soft); font-size: 0.625rem; }
	@media (max-width: 40rem) { .footer-right { display: none; } }
	@media (prefers-reduced-motion: reduce) { .links a { transition: none; } }
</style>
