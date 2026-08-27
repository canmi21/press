<script lang="ts">
	import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
	import * as m from '$lib/paraglide/messages';
	import type { ArticleNote } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';

	/** `locale` is the view being rendered. Passed rather than read: see spec/locale.md. */
	let { notes, locale }: { notes: ArticleNote[]; locale: LocaleCode } = $props();
</script>

<!-- The one place the article speaks in its own voice rather than the writer's, so it is quiet:
     a rule above it, the heading at the size of the metadata row, and the notes smaller than the
     prose they came from. -->
<section aria-label={m['article.notes']({}, { locale })} class="fn-notes">
	<h2 class="fn-notes-heading">{m['article.notes']({}, { locale })}</h2>
	<ol class="fn-notes-list">
		{#each notes as note (note.number)}
			<li id="fn-{note.number}" class="jump-target fn-note">
				<span class="fn-note-text">{note.text}</span>
				<a
					href="#fnref-{note.number}"
					class="fn-note-back focus-link"
					aria-label={m['article.notes.back']({ number: note.number }, { locale })}
				>
					<CornerDownLeft class="size-3.5" aria-hidden="true" />
				</a>
			</li>
		{/each}
	</ol>
</section>

<style>
	.fn-notes {
		margin-top: 4rem;
		border-top: 0.0625rem solid var(--color-border);
		padding-top: 1.5rem;
	}

	.fn-notes-heading {
		margin: 0 0 0.75rem;
		color: var(--color-text-soft);
		font-size: 0.875rem;
		font-weight: 500;
	}

	.fn-notes-list {
		display: flex;
		margin: 0;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0;
		/* The number is the reader's way back to the marker, so it is the list's own counter
		   rather than a bullet: `1.` beside a marker that read `1` is the same label twice. */
		list-style: none;
		counter-reset: fn-note;
	}

	.fn-note {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		counter-increment: fn-note;
		color: var(--color-text-soft);
		font-size: 0.8125rem;
		line-height: 1.6;
	}

	.fn-note::before {
		content: counter(fn-note);
		flex: none;
		min-width: 1rem;
		color: var(--color-text-soft);
		font-size: 0.6875rem;
		font-variant-numeric: tabular-nums;
		text-align: right;
	}

	.fn-note-text {
		flex: 1;
	}

	/* Beside the note rather than after its last word: a note that wraps would otherwise put the
	   way back at whatever ragged point the text happened to end on. */
	.fn-note-back {
		flex: none;
		align-self: center;
		color: var(--color-text-soft);
		line-height: 1;
		transition: color 200ms ease-out;
	}

	.fn-note-back:hover,
	.fn-note-back:focus-visible {
		color: var(--color-text-strong);
	}

	@media (prefers-reduced-motion: reduce) {
		.fn-note-back {
			transition: none;
		}
	}
</style>
