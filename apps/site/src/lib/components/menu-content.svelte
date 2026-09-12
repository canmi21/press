<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		id,
		align = 'start',
		children,
	}: {
		id?: string;
		/** Which of the trigger's edges the panel lines up with. */
		align?: 'start' | 'end';
		children: Snippet;
	} = $props();

	/**
	 * How close this surface may come to the window's edge, in pixels.
	 *
	 * The page's own gutter, so a menu pushed back by a collision stops where the article's text
	 * stops rather than a hair from the glass. The library's default is 8px, which is invisible on
	 * a laptop -- nothing there is near an edge -- and on a phone puts the whole panel against the
	 * side of the screen while the column beside it holds a 1.5rem margin. See spec/styling.md.
	 *
	 * A number rather than the token, because the library measures in pixels and cannot read a
	 * custom property. It agrees with the article column's `px-6` by hand.
	 */
	const EDGE_PADDING = 24;
</script>

<DropdownMenu.Portal>
	<DropdownMenu.Content
		{id}
		{align}
		sideOffset={8}
		collisionPadding={EDGE_PADDING}
		loop
		class="menu-content z-30 min-w-36 overflow-hidden rounded-md border border-border bg-paper shadow-sm"
	>
		{@render children()}
	</DropdownMenu.Content>
</DropdownMenu.Portal>

<style>
	:global(.menu-content) {
		transition:
			opacity 150ms cubic-bezier(0.22, 1, 0.36, 1),
			transform 150ms cubic-bezier(0.22, 1, 0.36, 1);
		transform-origin: var(--bits-dropdown-menu-content-transform-origin);
	}

	:global(.menu-content[data-starting-style]),
	:global(.menu-content[data-ending-style]) {
		opacity: 0;
		transform: scale(0.98);
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.menu-content) {
			transition: none;
		}
	}
</style>
