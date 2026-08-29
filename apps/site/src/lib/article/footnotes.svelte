<script lang="ts">
	import CornerDownLeft from '@lucide/svelte/icons/corner-down-left';
	import * as m from '$lib/paraglide/messages';
	import { jumpTo, movesThisPage, targetOf } from '$lib/client/jump';
	import type { ArticleNote } from '$lib/content/types';
	import type { LocaleCode } from '$lib/locale';

	/** `locale` is the view being rendered. Passed rather than read: see spec/locale.md. */
	let { notes, locale }: { notes: ArticleNote[]; locale: LocaleCode } = $props();

	/**
	 * The way back, scrolled rather than jumped -- the same move the markers make, owned here
	 * because this section renders outside the article body whose delegation covers them. A
	 * component's own links do not need delegating; compiled prose has no component, which is
	 * what the delegation in body.svelte is for.
	 */
	function jumpBack(event: MouseEvent & { currentTarget: HTMLAnchorElement }) {
		if (!movesThisPage(event)) return;
		const destination = targetOf(event.currentTarget);
		if (!destination) return;
		// The move is not an address: see spec/styling.md.
		event.preventDefault();
		jumpTo(destination);
	}
</script>

<!-- Apparatus about the article, not part of it: this renders after the article closes, under
     the dashed rule that is the article's ending boundary, so the rail and the table of contents
     never measure it. Quiet for the same reason -- the heading at the size of the metadata row,
     and the notes smaller than the prose they came from. See spec/styling.md. -->
<section aria-label={m['article.notes']({}, { locale })} class="notes">
	<!-- The heading is chrome and stays quiet; the notes under it are not. -->
	<h2 class="notes-heading">{m['article.notes']({}, { locale })}</h2>
	<ol class="notes-list">
		{#each notes as note (note.number)}
			<li id="note-{note.number}" class="jump-target note">
				<!-- The words first, so a note names what it is about instead of asking the reader
				     to hold the sentence they left in their head. Then the same superscript the
				     marker in the prose is, which makes the two one thing seen twice -- hidden
				     from a screen reader, which is already being told the ordinal by the list.

				     The explanation is the way back, not just the arrow at its end: an icon-sized
				     target at the end of a wrapped line asks for aim this size of text does not
				     deserve. The phrase and its number stay outside the link -- they are the
				     note's address, not its content, and the address is what the walk returns to.
				     The link's accessible name stays the explanation itself -- an aria-label would
				     replace it -- and the purpose rides after it as words only a screen reader
				     gets. -->
				<span class="note-phrase">{note.phrase}</span><sup class="note-marker" aria-hidden="true"
					>{note.number}</sup
				><a href="#marker-{note.number}" class="note-link focus-link" onclick={jumpBack}
					>{note.text}<span class="note-back" aria-hidden="true">
						<CornerDownLeft class="size-[1.1em]" />
					</span><span class="sr-only">
						({m['article.notes.back']({ number: note.number }, { locale })})</span
					></a
				>
			</li>
		{/each}
	</ol>
</section>

<style>
	/* This rule is the article's ending boundary, worn by the notes when they exist and by the
	   newsletter otherwise -- see article.svelte. Dashed because what follows an article is
	   offered rather than fenced off; the plain rule below the notes is then only a separator
	   between two offerings. The spacing matches the newsletter's own (mt-16 plus its padding),
	   so the boundary sits where it always has whether or not an article carried notes. */
	.notes {
		margin-top: 4rem;
		border-top: 0.0625rem dashed var(--color-border);
		padding-top: 1.5rem;
	}

	.notes-heading {
		margin: 0 0 0.75rem;
		color: var(--color-text-soft);
		font-size: 0.875rem;
		font-weight: 500;
	}

	/* An ordered list still, for what a screen reader is told, with nothing of a list drawn: the
	   numbers a reader sees are the superscripts, which is how they were met in the prose. */
	.notes-list {
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
	.note {
		margin: 0;
		font-size: 0.6875rem;
		line-height: 1.6;
		color: var(--color-text-soft);
	}

	/* The quoted words are the article's, said again -- weight alone marks them. Colour is spent
	   on the number and the arrow instead: the two ends of the walk, which note this is and the
	   way back from it. */
	.note-phrase {
		font-weight: 500;
	}

	.note > :global(.note-marker) {
		color: var(--color-text-strong);
	}

	/* Here the marker sits between the phrase and the note, so it needs air on the side facing
	   the note. In the prose it follows the word it belongs to and must not be spaced off it. */
	.note > :global(.note-marker) {
		margin-inline-end: 0.3rem;
	}

	/* The link is the explanation: it inherits the note's quiet colour and brightens whole
	   under the pointer, so hovering anywhere on those words says they are the control. The
	   phrase and number ahead of it sit outside and keep their resting look. The underline
	   stays off -- eight dotted lines of apparatus would out-shout the article above them. */
	.note-link {
		color: inherit;
		text-decoration: none;
		transition: color 200ms ease-out;
	}

	.note-link:hover,
	.note-link:focus-visible {
		color: var(--color-text-strong);
	}

	/* Trailing the last word, not parked at the right edge. The way back belongs to the sentence
	   that was just read, and a column of arrows down the right reads as a table's furniture.
	   Bright at rest, like the number at the note's head: the two ends of the walk are the two
	   points of colour, and everything between them is the reading. */
	.note-back {
		display: inline-flex;
		margin-inline-start: 0.35rem;
		/* Sized against the note, not against the page, for the same reason the marker is. */
		font-size: 0.9em;
		color: var(--color-text-strong);
		/* Zero, so an arrow at the end of a wrapped note cannot open up the line it lands on. */
		line-height: 0;
		vertical-align: -0.1em;
	}

	@media (prefers-reduced-motion: reduce) {
		.note-link {
			transition: none;
		}
	}
</style>
