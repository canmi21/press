/**
 * A CJK letter, as opposed to CJK punctuation.
 *
 * Script properties rather than a block range, which is what keeps `，`, `。` and `、` out: a
 * full-width comma already carries its own trailing space in the glyph, and putting another one
 * beside it opens a hole in the line.
 */
const CJK_LETTER = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]$/u;
const CJK_LETTER_START =
	/^[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/u;
const LATIN = /^[A-Za-z0-9]/;
const LATIN_END = /[A-Za-z0-9]$/;

/**
 * Put a space where a Latin run meets a CJK letter, in text assembled at runtime.
 *
 * Authored copy carries these spaces already -- typing them is what everybody writing Chinese
 * or Japanese on the web does, and the articles here do it too. Text this site *builds* has
 * nobody to type them: `Intl.ListFormat` joins `crates.io` and `npm` with a bare `和`, and the
 * result reads as one unbroken run.
 *
 * `text-autospace: normal` was the first attempt and is not enough. It works -- measured, it
 * does apply, and it applies across element boundaries -- but Chrome implements the property's
 * eighth of an em, which came out at 2px against the 4.4px of a real space, and no other engine
 * ships it at all. A rule that lands on one browser and is invisible when it does is not the
 * fix; the space that everybody else types is.
 *
 * Only letters count, so `npm，` keeps its comma tight against it.
 */
export function spaceScriptBoundaries(parts: readonly string[]): string[] {
	return parts.map((part, index) => {
		if (index === 0) return part;
		const previous = parts[index - 1] ?? '';
		const meets =
			(CJK_LETTER.test(previous) && LATIN.test(part)) ||
			(LATIN_END.test(previous) && CJK_LETTER_START.test(part));
		return meets ? ` ${part}` : part;
	});
}
