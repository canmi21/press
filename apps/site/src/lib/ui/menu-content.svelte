<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		id,
		children,
	}: {
		id?: string;
		children: Snippet;
	} = $props();
</script>

<DropdownMenu.Portal>
	<DropdownMenu.Content
		{id}
		align="start"
		sideOffset={8}
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
