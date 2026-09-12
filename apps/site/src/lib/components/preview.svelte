<script lang="ts">
	import X from '@lucide/svelte/icons/x';
	import { Dialog } from 'bits-ui';
	import type { Snippet } from 'svelte';

	let {
		label,
		title,
		closeLabel,
		width,
		height,
		radius,
		inline,
		enlarged,
	}: {
		/** The trigger's accessible name: what pressing the picture does. */
		label: string;
		/** The enlarged view's accessible name. */
		title: string;
		closeLabel: string;
		/**
		 * The picture's own dimensions -- pixels for a photograph, drawn units for a diagram.
		 * Only their ratio is read here. Absent means the window is the only thing that sizes it.
		 */
		width?: number;
		height?: number;
		/** The trigger's corner, so the focus ring follows the shape of what it is around. */
		radius?: string;
		inline: Snippet;
		enlarged: Snippet;
	} = $props();

	let open = $state(false);

	/**
	 * How far a pointer may travel between press and release and still have been a tap.
	 *
	 * Any press on the ground dismisses, so without this a drag that ended over the picture --
	 * a swipe, a selection, a pointer that was put down and thought about -- would dismiss on
	 * release. Primary pointer only: a pinch puts a second one down elsewhere and lifts it a few
	 * pixels from where it landed, which is a tap by every measure except intent.
	 */
	const TAP_SLOP = 8;
	let pressed: { x: number; y: number } | undefined;

	function press(event: PointerEvent) {
		const taps = event.isPrimary && event.button === 0;
		pressed = taps ? { x: event.clientX, y: event.clientY } : undefined;
	}

	function release(event: PointerEvent) {
		const from = pressed;
		pressed = undefined;
		if (!from || !event.isPrimary) return;
		if (Math.hypot(event.clientX - from.x, event.clientY - from.y) > TAP_SLOP) return;
		open = false;
	}
</script>

<button
	type="button"
	class="preview-trigger focus-ring"
	style:border-radius={radius}
	aria-label={label}
	onclick={() => (open = true)}
>
	{@render inline()}
</button>

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay class="preview-ground fixed inset-0 z-50" />
		<Dialog.Content
			class="preview-stage fixed inset-0 z-50 flex items-center justify-center overflow-hidden"
			onpointerdown={press}
			onpointerup={release}
		>
			<Dialog.Title class="sr-only">{title}</Dialog.Title>
			<div
				class="preview-figure"
				class:preview-measured={width && height}
				style:--picture-width={width}
				style:--picture-height={height}
			>
				{@render enlarged()}
			</div>
			<Dialog.Close class="preview-close focus-ring" aria-label={closeLabel}>
				<X class="size-4" aria-hidden="true" />
			</Dialog.Close>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	.preview-trigger {
		display: block;
		width: 100%;
		margin: 0;
		border: 0;
		background: none;
		padding: 0;
		/* One cursor over the whole of it, because every part of it does the one thing. */
		cursor: zoom-in;
	}

	/* Pure black, in both themes, behind every picture. The page's two grounds are a warm
	   near-white and a warm near-black, and a picture read against either of them is being read
	   against the site rather than on its own. This is the one surface here that does not follow
	   the theme, which is why its colour is written as a literal and not taken from the palette.
	   See spec/styling.md. */
	:global(.preview-ground) {
		background: #000;
		transition: opacity 200ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	:global(.preview-stage) {
		transition: opacity 200ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	.preview-figure {
		/* What a transparent picture keeps: the ground it was drawn against, which is the page's
		   own and therefore the theme's. A diagram is ink on light or light on dark and has to
		   stay whichever it is; the black behind it does not move either way. An opaque
		   photograph covers this and never knows it is there. */
		background: var(--color-page);
		max-width: 100vw;
		max-height: 100dvh;
		transition: transform 200ms cubic-bezier(0.22, 1, 0.36, 1);
	}

	/* Two terms, and the smaller of them wins: the picture reaches the window's left and right
	   edges when it is the wider of the two, and its top and bottom when it is the taller. There
	   is no third term. Nothing here is allowed to hold the picture off an edge -- not a gutter,
	   not a corner radius, not a frame, and not a ceiling on how large it may be drawn. Whatever
	   the window has, the picture takes. See spec/styling.md. */
	.preview-figure.preview-measured {
		aspect-ratio: var(--picture-width) / var(--picture-height);
		width: min(100vw, calc(100dvh * var(--picture-width) / var(--picture-height)));
	}

	.preview-figure :global(svg),
	.preview-figure :global(img),
	.preview-figure :global(picture) {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	/* The one piece of chrome on a layer that is always dark, so it is written for that rather
	   than taken from a palette that answers to the page's theme. It has to stay legible over a
	   photograph as well as over the ground, which is what the wash and the hairline are for. */
	:global(.preview-close) {
		position: fixed;
		top: max(1.5rem, env(safe-area-inset-top));
		right: max(1.5rem, env(safe-area-inset-right));
		display: inline-flex;
		width: 2.25rem;
		height: 2.25rem;
		align-items: center;
		justify-content: center;
		border: 1px solid rgb(255 255 255 / 0.18);
		border-radius: 9999px;
		background: rgb(0 0 0 / 0.55);
		color: rgb(255 255 255 / 0.75);
		cursor: pointer;
		transition:
			background-color 150ms,
			color 150ms;
	}

	:global(.preview-close:hover),
	:global(.preview-close:focus-visible) {
		background: rgb(0 0 0 / 0.8);
		color: rgb(255 255 255);
	}

	:global(.preview-ground[data-starting-style]),
	:global(.preview-ground[data-ending-style]),
	:global(.preview-stage[data-starting-style]),
	:global(.preview-stage[data-ending-style]) {
		opacity: 0;
	}

	/* The stage is the window, so it is the picture inside it that arrives, not the box. */
	:global(.preview-stage[data-starting-style]) .preview-figure,
	:global(.preview-stage[data-ending-style]) .preview-figure {
		transform: scale(0.97);
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.preview-ground),
		:global(.preview-stage),
		.preview-figure {
			transition: none;
		}
	}
</style>
