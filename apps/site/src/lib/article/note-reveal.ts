/**
 * Opening the collected notes before a walk down lands in the part that is folded away.
 *
 * The two halves of a note live in different places: the marker is compiled HTML inside the
 * article body, delegated in body.svelte, while whether a note is currently folded is
 * footnotes.svelte's own state. A marker cannot ask the section a question, so the section
 * leaves an answer here for the length of its life.
 *
 * Deliberately one slot rather than a set of subscribers. A page renders one collected-notes
 * section or none, and a registry would invite a second caller whose meaning -- which section
 * did the reader mean? -- has no answer.
 */

/** Opens the folded notes if `target` is one of them; answers whether anything moved. */
type Revealer = (target: Element) => boolean;

let revealer: Revealer | undefined;

/** Offer the page's notes section. Returns the undo, to be called when that section goes. */
export function offerNoteReveal(reveal: Revealer): () => void {
	revealer = reveal;
	return () => {
		if (revealer === reveal) revealer = undefined;
	};
}

/**
 * Unfold the notes so `target` is really there, before anybody measures where it is.
 *
 * The caller must do this *before* asking the page to scroll. `scrollIntoView` resolves its
 * destination at the moment it is called, so content unfolding underneath afterwards moves the
 * note away from the position the scroll is already travelling to. Unfolding first is also what
 * keeps it unseen: the section is still below the fold, so the jump in height happens where
 * nobody is looking, and the reader arrives at a section that was simply already open.
 */
export function revealNoteBeforeJump(target: Element): boolean {
	return revealer?.(target) ?? false;
}
