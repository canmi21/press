<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';
	import X from '@lucide/svelte/icons/x';
	import { animate } from 'motion';
	import { onDestroy, untrack } from 'svelte';
	import { DEFAULT_PIXELS_PER_REM, remFromMeasuredPixels } from '$lib/client/units';

	type Props = {
		label?: string;
		title?: string;
		collapsible?: boolean;
		defaultExpanded?: boolean;
		copyLabel: string;
		copiedLabel: string;
		copyFailedLabel: string;
		code?: string;
		html?: string;
	};
	let {
		label,
		title,
		collapsible,
		defaultExpanded = true,
		copyLabel,
		copiedLabel,
		copyFailedLabel,
		code,
		html,
	}: Props = $props();

	type AnimationControl = { stop: () => void };
	type CollapsePhase = 'collapsed' | 'collapsing' | 'expanded' | 'expanding';
	type CopyState = 'copied' | 'copying' | 'failed' | 'idle';

	const COLLAPSE_SPRING = {
		type: 'spring' as const,
		stiffness: 420,
		damping: 38,
		mass: 0.9,
	};
	const COPY_REVEAL_SPRING = {
		type: 'spring' as const,
		stiffness: 500,
		damping: 36,
		mass: 0.8,
	};
	const COPY_REVEAL_REM = 1.25;
	const COPY_GLYPH_REM = 0.875;
	const COPY_SLIDE_REM = 0.375;
	const COPY_FEEDBACK_RESET_MS = 650;
	const instanceId = $props.id();
	const panelId = `${instanceId}-panel`;
	// This prop is an initial state, not a command that reopens a disclosure after interaction.
	const initiallyExpanded = untrack(() => !title || collapsible === false || defaultExpanded);
	let expanded = $state(initiallyExpanded);
	let phase = $state<CollapsePhase>(initiallyExpanded ? 'expanded' : 'collapsed');
	let collapseEl = $state<HTMLElement>();
	let collapseMotion: AnimationControl | undefined;
	let copyState = $state<CopyState>('idle');
	let copyMaskEl = $state<HTMLElement>();
	let copyGlyphEl = $state<HTMLElement>();
	let copyRevealMotion: AnimationControl | undefined;
	let copyRevealProgress = 0;
	let copyPointerInside = false;
	let copyKeyboardFocused = false;
	let copyFeedbackTimer: ReturnType<typeof setTimeout> | undefined;
	let destroyed = false;
	const canCollapse = $derived(Boolean(title) && (collapsible ?? true));
	const hasLanguageLabel = $derived(Boolean(label));
	const panelHidden = $derived(canCollapse && !expanded);
	const dividerVisible = $derived(!canCollapse || phase !== 'collapsed');
	const copyActionLabel = $derived(
		copyState === 'copied' ? copiedLabel : copyState === 'failed' ? copyFailedLabel : copyLabel,
	);
	const copyFeedback = $derived(
		copyState === 'copied' ? copiedLabel : copyState === 'failed' ? copyFailedLabel : '',
	);

	function settle(nextExpanded: boolean) {
		if (collapseEl) collapseEl.style.height = nextExpanded ? 'auto' : '0rem';
		phase = nextExpanded ? 'expanded' : 'collapsed';
		collapseMotion = undefined;
	}

	function setExpanded(nextExpanded: boolean) {
		if (!canCollapse || nextExpanded === expanded || !collapseEl) return;

		const currentHeight = collapseEl.getBoundingClientRect().height;
		collapseMotion?.stop();
		collapseMotion = undefined;
		collapseEl.style.height = remFromMeasuredPixels(currentHeight);
		expanded = nextExpanded;
		phase = nextExpanded ? 'expanding' : 'collapsing';

		const targetHeight = nextExpanded ? collapseEl.scrollHeight : 0;
		if (
			window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
			Math.abs(currentHeight - targetHeight) < 0.5
		) {
			settle(nextExpanded);
			return;
		}

		const rootPixels =
			Number.parseFloat(getComputedStyle(document.documentElement).fontSize) ||
			DEFAULT_PIXELS_PER_REM;
		let control: AnimationControl;
		control = animate(currentHeight, targetHeight, {
			...COLLAPSE_SPRING,
			onUpdate: (height) => {
				collapseEl?.style.setProperty(
					'height',
					remFromMeasuredPixels(Math.max(0, height), rootPixels),
				);
			},
			onComplete: () => {
				if (collapseMotion !== control) return;
				settle(nextExpanded);
			},
		});
		collapseMotion = control;
	}

	function renderCopyReveal(progress: number) {
		copyRevealProgress = progress;
		if (copyMaskEl) {
			copyMaskEl.style.width = `${hasLanguageLabel ? COPY_REVEAL_REM * progress : COPY_GLYPH_REM}rem`;
		}
		if (copyGlyphEl) {
			copyGlyphEl.style.opacity = String(progress);
			copyGlyphEl.style.transform = hasLanguageLabel
				? `translateX(${COPY_SLIDE_REM * (1 - progress)}rem)`
				: `scale(${0.82 + 0.18 * progress})`;
		}
	}

	function setCopyReveal(revealed: boolean) {
		const target = revealed ? 1 : 0;
		copyRevealMotion?.stop();
		copyRevealMotion = undefined;
		if (
			window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
			Math.abs(copyRevealProgress - target) < 0.001
		) {
			renderCopyReveal(target);
			return;
		}

		let control: AnimationControl;
		control = animate(copyRevealProgress, target, {
			...COPY_REVEAL_SPRING,
			onUpdate: renderCopyReveal,
			onComplete: () => {
				if (copyRevealMotion !== control) return;
				renderCopyReveal(target);
				copyRevealMotion = undefined;
			},
		});
		copyRevealMotion = control;
	}

	function clearCopyFeedbackTimer() {
		if (!copyFeedbackTimer) return;
		clearTimeout(copyFeedbackTimer);
		copyFeedbackTimer = undefined;
	}

	function resetCopyFeedback() {
		if (copyPointerInside || copyKeyboardFocused || copyState === 'copying') return;
		copyState = 'idle';
		setCopyReveal(false);
	}

	function scheduleCopyReset() {
		clearCopyFeedbackTimer();
		if (copyState === 'idle') {
			setCopyReveal(false);
			return;
		}
		if (copyState === 'copying') return;

		copyFeedbackTimer = setTimeout(() => {
			copyFeedbackTimer = undefined;
			resetCopyFeedback();
		}, COPY_FEEDBACK_RESET_MS);
	}

	function enterCopy() {
		copyPointerInside = true;
		clearCopyFeedbackTimer();
		setCopyReveal(true);
	}

	function leaveCopy() {
		copyPointerInside = false;
		if (!copyKeyboardFocused) scheduleCopyReset();
	}

	function focusCopy(event: FocusEvent) {
		copyKeyboardFocused = (event.currentTarget as HTMLElement).matches(':focus-visible');
		if (!copyKeyboardFocused) return;
		clearCopyFeedbackTimer();
		setCopyReveal(true);
	}

	function keyCopy() {
		copyKeyboardFocused = true;
		clearCopyFeedbackTimer();
		setCopyReveal(true);
	}

	function blurCopy() {
		copyKeyboardFocused = false;
		if (!copyPointerInside) scheduleCopyReset();
	}

	async function copySource() {
		if (code === undefined) return;
		clearCopyFeedbackTimer();
		copyState = 'copying';
		setCopyReveal(true);
		try {
			await navigator.clipboard.writeText(code);
			if (destroyed) return;
			copyState = 'copied';
		} catch {
			if (destroyed) return;
			copyState = 'failed';
		}
		if (!copyPointerInside && !copyKeyboardFocused) scheduleCopyReset();
	}

	onDestroy(() => {
		destroyed = true;
		collapseMotion?.stop();
		copyRevealMotion?.stop();
		clearCopyFeedbackTimer();
	});
</script>

{#snippet codeAction()}
	{#if code !== undefined}
		<button
			type="button"
			class="code-copy focus-ring"
			class:code-copy-unlabelled={!hasLanguageLabel}
			data-copy-state={copyState}
			aria-label={copyActionLabel}
			onmouseenter={enterCopy}
			onmouseleave={leaveCopy}
			onfocus={focusCopy}
			onblur={blurCopy}
			onkeydown={keyCopy}
			onclick={copySource}
		>
			{#if label}<span class="code-copy-label" aria-hidden="true">{label}</span>{/if}
			<span bind:this={copyMaskEl} class="code-copy-mask" aria-hidden="true">
				<span bind:this={copyGlyphEl} class="code-copy-glyph">
					<span class="code-copy-icon code-copy-request">
						<Copy class="size-3.5" />
					</span>
					<span class="code-copy-icon code-copy-success">
						<Check class="size-3.5" />
					</span>
					<span class="code-copy-icon code-copy-failure">
						<X class="size-3.5" />
					</span>
				</span>
			</span>
		</button>
		<span class="sr-only" aria-live="polite">{copyFeedback}</span>
	{/if}
{/snippet}

{#snippet source()}
	{#if html}
		{@html html}
	{:else if code}
		<pre><code>{code}</code></pre>
	{/if}
{/snippet}

<div class="codeblock relative">
	{#if title}
		<div
			class="code-frame focus-ring-within overflow-hidden rounded-xl border border-border bg-paper"
		>
			{#if canCollapse}
				<button
					type="button"
					class="code-title flex w-full cursor-pointer items-center justify-between gap-3 border-border bg-paper-hover px-4 py-2.5 text-left text-sm font-medium text-text transition-colors duration-150 hover:text-text-strong"
					class:border-b={dividerVisible}
					aria-expanded={expanded}
					aria-controls={panelId}
					onclick={() => setExpanded(!expanded)}
				>
					<span>{title}</span>
					<span
						class="code-chevron shrink-0 text-text-soft transition-transform duration-200"
						class:rotate-180={expanded}
					>
						<ChevronDown class="size-3.5" aria-hidden="true" />
					</span>
				</button>
			{:else}
				<div
					class="border-b border-border bg-paper-hover px-4 py-2.5 text-sm font-medium text-text"
				>
					{title}
				</div>
			{/if}

			<div
				bind:this={collapseEl}
				class="code-collapse"
				data-phase={canCollapse ? phase : 'expanded'}
			>
				<div id={panelId} class="code-panel relative" aria-hidden={panelHidden} inert={panelHidden}>
					{@render codeAction()}
					<div class="code-scroll overflow-x-auto bg-paper p-4 pr-16 text-sm leading-snug">
						{@render source()}
					</div>
				</div>
			</div>
		</div>
	{:else}
		<div class="code-panel relative">
			{@render codeAction()}
			<!-- Shiki tags its <pre> with tabindex=0 so keyboard users can focus and scroll the
			code, so focus lands on the inner code area, not this box. The ring is redirected
			out to this bordered box via :has (see <style>) so it wraps the whole code block. -->
			<div
				class="code-scroll focus-ring-within overflow-x-auto rounded-xl border border-border bg-paper p-4 pr-16 text-sm leading-snug"
			>
				{@render source()}
			</div>
		</div>
	{/if}
</div>

<style>
	.codeblock :global(pre) {
		background: transparent;
		margin: 0;
		padding: 0;
	}

	/* Shiki's <pre> takes focus, while the shared within utility draws on this box. */
	.codeblock :global(pre:focus-visible),
	.code-title:focus-visible {
		outline: none;
	}

	.code-copy {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		z-index: 10;
		display: inline-flex;
		height: 1.5rem;
		min-width: 1.5rem;
		cursor: pointer;
		align-items: center;
		justify-content: flex-end;
		border-radius: 0.25rem;
		padding-inline: 0.25rem;
		font-size: 0.75rem;
		line-height: 1;
		letter-spacing: 0.05em;
		color: var(--color-text-soft);
		transition: color 150ms;
	}

	.code-copy:hover,
	.code-copy:focus-visible {
		color: var(--color-text-strong);
	}

	.code-copy-unlabelled {
		justify-content: center;
	}

	.code-copy-mask {
		display: inline-flex;
		width: 0;
		flex-shrink: 0;
		overflow: hidden;
	}

	.code-copy-glyph {
		display: grid;
		width: 0.875rem;
		flex: 0 0 0.875rem;
		margin-left: 0.375rem;
		opacity: 0;
		transform: translateX(0.375rem);
	}

	.code-copy-unlabelled .code-copy-glyph {
		margin-left: 0;
		transform: scale(0.82);
	}

	.code-copy-unlabelled .code-copy-mask {
		width: 0.875rem;
	}

	.code-copy-icon {
		grid-area: 1 / 1;
		display: grid;
		place-items: center;
		opacity: 0;
		transform: scale(0.82);
		transition:
			opacity 120ms ease-out,
			transform 120ms ease-out;
	}

	.code-copy[data-copy-state='idle'] .code-copy-request,
	.code-copy[data-copy-state='copying'] .code-copy-request,
	.code-copy[data-copy-state='copied'] .code-copy-success,
	.code-copy[data-copy-state='failed'] .code-copy-failure {
		opacity: 1;
		transform: scale(1);
	}

	.code-collapse {
		overflow: hidden;
	}

	.code-collapse[data-phase='collapsed'] {
		height: 0;
	}

	.code-collapse[data-phase='collapsing'],
	.code-collapse[data-phase='expanding'] {
		will-change: height;
	}

	.codeblock :global(.shiki),
	.codeblock :global(.shiki span) {
		color: var(--shiki-light);
	}

	:global(.dark) .codeblock :global(.shiki),
	:global(.dark) .codeblock :global(.shiki span),
	:global([data-theme='dark']) .codeblock :global(.shiki),
	:global([data-theme='dark']) .codeblock :global(.shiki span) {
		color: var(--shiki-dark);
	}

	@media (prefers-reduced-motion: reduce) {
		.code-chevron,
		.code-copy-icon {
			transition: none;
		}
	}
</style>
