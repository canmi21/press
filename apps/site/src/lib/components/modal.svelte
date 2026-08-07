<script lang="ts">
	import X from '@lucide/svelte/icons/x';
	import { Dialog } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		open,
		title,
		closeLabel,
		icon,
		onOpenChange,
		children,
	}: {
		open: boolean;
		title: string;
		closeLabel: string;
		/** Optional mark for the surface, usually the icon of the action that opened it. */
		icon?: Snippet;
		onOpenChange: (open: boolean) => void;
		children: Snippet;
	} = $props();
</script>

<Dialog.Root {open} {onOpenChange}>
	<Dialog.Portal>
		<Dialog.Overlay class="modal-overlay fixed inset-0 z-50" />
		<Dialog.Content
			class="modal-content fixed top-1/2 left-1/2 z-50 w-[min(26rem,calc(100vw-3rem))] rounded-lg border border-border bg-paper p-5 text-text shadow-sm"
		>
			<!-- The mark and the close control are each centred on a 1.5rem box so they sit on the
			first line of the title rather than on the whole header, which a wrapped title moves. -->
			<div class="flex items-start gap-2.5">
				{#if icon}
					<span class="flex h-6 shrink-0 items-center text-text-soft">
						{@render icon()}
					</span>
				{/if}
				<Dialog.Title class="min-w-0 flex-1 font-medium text-text-strong">{title}</Dialog.Title>
				<Dialog.Close
					aria-label={closeLabel}
					class="focus-ring -mt-0.5 -mr-1 inline-flex size-7 shrink-0 items-center justify-center rounded-md text-text-soft transition-colors duration-150 hover:bg-paper-hover hover:text-text-strong focus-visible:bg-paper-hover focus-visible:text-text-strong"
				>
					<X class="size-4" aria-hidden="true" />
				</Dialog.Close>
			</div>
			<Dialog.Description class="mt-2 text-[0.9375rem] leading-relaxed text-pretty text-text-soft">
				{@render children()}
			</Dialog.Description>
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
