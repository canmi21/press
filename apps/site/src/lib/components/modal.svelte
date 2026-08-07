<script lang="ts">
	import X from '@lucide/svelte/icons/x';
	import { Dialog } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		open,
		title,
		closeLabel,
		onOpenChange,
		children,
	}: {
		open: boolean;
		title: string;
		closeLabel: string;
		onOpenChange: (open: boolean) => void;
		children: Snippet;
	} = $props();
</script>

<Dialog.Root {open} {onOpenChange}>
	<Dialog.Portal>
		<Dialog.Overlay class="modal-overlay fixed inset-0 z-50" />
		<Dialog.Content
			class="modal-content fixed top-1/2 left-1/2 z-50 w-[min(30rem,calc(100vw-3rem))] rounded-lg border border-border bg-paper p-6 pr-14 text-text shadow-sm"
		>
			<Dialog.Title class="sr-only">{title}</Dialog.Title>
			<Dialog.Description class="leading-relaxed text-pretty">
				{@render children()}
			</Dialog.Description>
			<Dialog.Close
				aria-label={closeLabel}
				class="focus-ring absolute top-3 right-3 inline-flex size-8 items-center justify-center rounded-md text-text-soft transition-colors duration-150 hover:bg-paper-hover hover:text-text-strong focus-visible:bg-paper-hover focus-visible:text-text-strong"
			>
				<X class="size-4" aria-hidden="true" />
			</Dialog.Close>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	:global(.modal-overlay) {
		background: color-mix(in oklch, var(--color-page) 38%, transparent);
		-webkit-backdrop-filter: blur(0.5rem);
		backdrop-filter: blur(0.5rem);
		transition: opacity 150ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	:global(.modal-content) {
		transform: translate(-50%, -50%);
		transition:
			opacity 150ms cubic-bezier(0.22, 1, 0.36, 1),
			transform 150ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	:global(.modal-overlay[data-starting-style]),
	:global(.modal-overlay[data-ending-style]),
	:global(.modal-content[data-starting-style]),
	:global(.modal-content[data-ending-style]) {
		opacity: 0;
	}

	:global(.modal-content[data-starting-style]),
	:global(.modal-content[data-ending-style]) {
		transform: translate(-50%, -50%) scale(0.98);
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.modal-overlay),
		:global(.modal-content) {
			transition: none;
		}
	}
</style>
