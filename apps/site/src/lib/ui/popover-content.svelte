<script lang="ts">
	import { Popover } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		anchor,
		id,
		labelledby,
		describedby,
		onEscapeKeydown,
		onInteractOutside,
		onOpenAutoFocus,
		onCloseAutoFocus,
		children,
	}: {
		anchor: HTMLElement | null;
		id?: string;
		labelledby?: string;
		describedby?: string;
		onEscapeKeydown?: (event: KeyboardEvent) => void;
		onInteractOutside?: (event: PointerEvent) => void;
		onOpenAutoFocus?: (event: Event) => void;
		onCloseAutoFocus?: (event: Event) => void;
		children: Snippet;
	} = $props();
</script>

<Popover.Portal>
	<Popover.Content
		{id}
		customAnchor={anchor}
		side="bottom"
		align="center"
		sideOffset={10}
		collisionPadding={12}
		strategy="fixed"
		role="note"
		aria-labelledby={labelledby}
		aria-describedby={describedby}
		{onEscapeKeydown}
		{onInteractOutside}
		{onOpenAutoFocus}
		{onCloseAutoFocus}
		class="popover-content z-40 overflow-hidden rounded-md border border-border bg-paper text-sm leading-relaxed text-text shadow-sm"
	>
		{@render children()}
	</Popover.Content>
</Popover.Portal>

<style>
	:global(.popover-content) {
		width: min(26rem, calc(100vw - 1.5rem));
		transition:
			opacity 150ms cubic-bezier(0.22, 1, 0.36, 1),
			transform 150ms cubic-bezier(0.22, 1, 0.36, 1);
		transform-origin: var(--bits-popover-content-transform-origin);
	}

	:global(.popover-content[data-starting-style]),
	:global(.popover-content[data-ending-style]) {
		opacity: 0;
		transform: scale(0.98);
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.popover-content) {
			transition: none;
		}
	}
</style>
