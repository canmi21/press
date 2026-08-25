<script lang="ts">
	import type { QuadrantDirection, QuadrantItem, QuadrantPosition } from '$lib/content/types';

	let {
		title,
		description,
		axes,
		items,
	}: {
		title: string;
		description?: string;
		axes: Record<QuadrantDirection, string>;
		items: QuadrantItem[];
	} = $props();

	const positions: QuadrantPosition[] = ['top-left', 'top-right', 'bottom-left', 'bottom-right'];

	function sentence(value: string): string {
		return /[.!?]$/u.test(value) ? value : `${value}.`;
	}

	function describeItem(item: QuadrantItem): string {
		const [vertical, horizontal] = item.at.split('-') as ['top' | 'bottom', 'left' | 'right'];
		return `${axes[vertical]} and ${axes[horizontal]}: ${item.title}${item.note ? `, ${item.note}` : ''}`;
	}

	function visualAxis(value: string): string {
		return value.replaceAll('-', '\u2011');
	}

	let accessibleDescription = $derived(
		[title, description, ...items.map(describeItem)]
			.filter((part): part is string => Boolean(part))
			.map((part) => sentence(part))
			.join(' '),
	);
</script>

<figure
	class="quadrant-block overflow-hidden rounded-xl border border-border bg-paper"
	role="img"
	aria-label={accessibleDescription}
>
	<div class="quadrant-scroll overflow-x-auto">
		<div class="quadrant-stage" aria-hidden="true">
			<div class="quadrant-plot">
				<div class="vertical-axis">
					<span class="axis-label axis-top">{visualAxis(axes.top)}</span>
					<span class="vertical-rule"></span>
					<span class="axis-label axis-bottom">{visualAxis(axes.bottom)}</span>
				</div>
				<div class="horizontal-axis">
					<span class="axis-label axis-left">{visualAxis(axes.left)}</span>
					<span class="axis-label axis-right">{visualAxis(axes.right)}</span>
				</div>

				<div class="quadrant-grid">
					{#each positions as position}
						{@const quadrantItems = items.filter((item) => item.at === position)}
						<div class="quadrant-cell" data-position={position}>
							{#each quadrantItems as item}
								<div class="quadrant-box">
									<div class="quadrant-item">
										<span class="quadrant-title">{item.title}</span>
										{#if item.note}<span class="quadrant-note">{item.note}</span>{/if}
									</div>
								</div>
							{/each}
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</figure>

<style>
	.quadrant-stage {
		position: relative;
		inline-size: min(100%, 45rem);
		min-inline-size: 36rem;
		margin-inline: auto;
		aspect-ratio: 38 / 21;
		color: var(--color-text);
	}

	.quadrant-plot {
		position: absolute;
		top: 50%;
		left: 50%;
		display: grid;
		min-inline-size: 20rem;
		max-inline-size: calc(100% - 5rem);
		min-block-size: 12rem;
		max-block-size: calc(100% - 4rem);
		translate: -50% -50%;
	}

	.axis-label {
		font-family: var(--font-mono);
		font-size: 0.625rem;
		line-height: 1.25;
		color: var(--color-text-soft);
		white-space: nowrap;
	}

	.vertical-axis {
		z-index: 1;
		position: relative;
		grid-area: 1 / 1;
		justify-self: center;
		inline-size: 0;
		pointer-events: none;
	}

	.vertical-rule {
		position: absolute;
		inset-block: 0;
		left: 0;
		border-inline-start: 0.0625rem solid var(--color-border-strong);
	}

	.vertical-rule::before {
		position: absolute;
		top: -0.5rem;
		left: 50%;
		width: 0;
		height: 0;
		translate: -50% 0;
		border-inline: 0.3125rem solid transparent;
		border-block-end: 0.5625rem solid var(--color-border-strong);
		content: '';
	}

	.axis-top,
	.axis-bottom {
		position: absolute;
		left: 50%;
		translate: -50% 0;
	}

	.axis-top {
		bottom: calc(100% + 1rem);
	}

	.axis-bottom {
		top: calc(100% + 0.75rem);
	}

	.horizontal-axis {
		z-index: 1;
		position: relative;
		display: grid;
		grid-area: 1 / 1;
		align-self: center;
		border-block-start: 0.0625rem solid var(--color-border-strong);
		pointer-events: none;
	}

	.horizontal-axis::after {
		position: absolute;
		top: 50%;
		right: -0.5rem;
		width: 0;
		height: 0;
		translate: 0 -50%;
		border-block: 0.3125rem solid transparent;
		border-inline-start: 0.5625rem solid var(--color-border-strong);
		content: '';
	}

	.axis-left,
	.axis-right {
		position: absolute;
		top: 50%;
		translate: 0 -50%;
	}

	.axis-left {
		right: calc(100% + 0.75rem);
	}

	.axis-right {
		left: calc(100% + 1rem);
	}

	.quadrant-grid {
		display: grid;
		grid-area: 1 / 1;
		grid-template: repeat(2, minmax(0, 1fr)) / repeat(2, minmax(0, 1fr));
		inline-size: max-content;
	}

	.quadrant-cell {
		display: flex;
		min-inline-size: 0;
		max-inline-size: 16rem;
		flex-wrap: wrap;
		gap: 0.625rem;
		padding: 1.25rem;
	}

	.quadrant-cell[data-position^='top'] {
		flex-wrap: wrap-reverse;
		align-content: flex-start;
		align-items: flex-end;
	}

	.quadrant-cell[data-position^='bottom'] {
		align-content: flex-start;
		align-items: flex-start;
	}

	.quadrant-cell[data-position$='left'] {
		flex-direction: row-reverse;
		justify-content: flex-start;
	}

	.quadrant-cell[data-position$='right'] {
		justify-content: flex-start;
	}

	.quadrant-box {
		display: flex;
		inline-size: max-content;
		max-inline-size: 11rem;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.625rem;
		padding: 0.625rem 1rem;
		border: 0.0625rem solid var(--color-border);
		border-radius: 0.375rem;
		background: var(--color-paper-hover);
		text-align: center;
	}

	.quadrant-item {
		display: grid;
		gap: 0.35rem;
		max-inline-size: 13rem;
	}

	.quadrant-title {
		font-size: 0.8125rem;
		font-weight: 500;
		line-height: 1.2;
		color: var(--color-text-strong);
	}

	.quadrant-note {
		font-family: var(--font-mono);
		font-size: 0.625rem;
		line-height: 1.25;
		color: var(--color-text-soft);
	}
</style>
