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
					<CornerDownLeft class="size-[1.1em]" aria-hidden="true" />
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
		/* Tight, because at this size the notes are a block to be scanned rather than paragraphs
		   to be read apart. Spaced as they were for the larger text, two notes read as two
		   unrelated things. */
		gap: 0.25rem;
		padding: 0;
		list-style: none;
	}

	/* Small and quiet, the way a note at the foot of a page is: it is there to be stepped over and
	   come back to, not read on the way past. Both were tried the other way -- set at the article's
	   size and colour, the section competed with the prose above it for the same attention.

	   The first attempt at quiet was grey and small with nothing to catch on, which read as
	   somebody else's apparatus. What makes it work now is the phrase: it holds the article's own
	   colour, so each note has one strong point to find it by and can be soft everywhere else. */
	.fn-note {
		margin: 0;
		font-size: 0.6875rem;
		line-height: 1.6;
		color: var(--color-text-soft);
	}

	/* The quoted words are the article's, said again -- weight alone marks them. Colour is spent
	   on the number instead: one bright thing per note, and it is the one that says which note
	   this is. */
	.fn-note-phrase {
		font-weight: 500;
	}

	.fn-note > :global(.fn-ref) {
		color: var(--color-text-strong);
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
		/* Sized against the note, not against the page, for the same reason the marker is. */
		font-size: 0.9em;
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
