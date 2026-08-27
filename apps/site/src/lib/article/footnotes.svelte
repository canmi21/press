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
	<!-- The heading is chrome and stays quiet; the notes under it are not. -->
	<h2 class="fn-notes-heading">{m['article.notes']({}, { locale })}</h2>
	<ol class="fn-notes-list">
		{#each notes as note (note.number)}
			<li id="fn-{note.number}" class="jump-target fn-note">
				<!-- The words first, so a note names what it is about instead of asking the reader
				     to hold the sentence they left in their head. Then the same superscript the
				     marker in the prose is, which makes the two one thing seen twice -- hidden
				     from a screen reader, which is already being told the ordinal by the list. -->
				<span class="fn-note-phrase">{note.phrase}</span><sup class="fn-ref" aria-hidden="true"
					>{note.number}</sup
				>{note.text}<a
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

	/* An ordered list still, for what a screen reader is told, with nothing of a list drawn: the
	   numbers a reader sees are the superscripts, which is how they were met in the prose. */
	.fn-notes-list {
		display: flex;
		margin: 0;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0;
		list-style: none;
	}

	/* Set like the article rather than like an apparatus below it. These are the writer's own
	   words, deferred rather than demoted, so they are read at the size the rest was read at. */
	.fn-note {
		margin: 0;
	}

	/* The quoted words are the article's, said again -- strong enough to find while scanning the
	   list, not so strong that the note reads as being about the word rather than by the writer. */
	.fn-note-phrase {
		color: var(--color-text-strong);
		font-weight: 500;
	}

	/* Here the marker sits between the phrase and the note, so it needs air on the side facing
	   the note. In the prose it follows the word it belongs to and must not be spaced off it. */
	.fn-note > :global(.fn-ref) {
		margin-inline-end: 0.3rem;
	}

	/* Trailing the last word, not parked at the right edge. The way back belongs to the sentence
	   that was just read, and a column of arrows down the right reads as a table's furniture. */
	.fn-note-back {
		display: inline-flex;
		margin-inline-start: 0.35rem;
		color: var(--color-text-soft);
		/* Zero, so an arrow at the end of a wrapped note cannot open up the line it lands on. */
		line-height: 0;
		vertical-align: -0.1em;
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
