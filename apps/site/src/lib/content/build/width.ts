/**
 * What a string draws to, in pixels, at the 16px type the card list and the article title use.
 *
 * A mirror of `width::pixels` in the CMS, and the only place this repository carries the same
 * rule twice. The CMS needs it to refuse a translation it just paid for; the site build needs it
 * to choose, per article and per view, whether a phone can be shown the full title. Neither can
 * call the other: the CMS is Rust and does not read the sidecars at build time, and putting the
 * choice in the build artifact would make that artifact depend on translation state with nothing
 * to detect it going stale.
 *
 * What holds the two together is the corpus below rather than good intentions. Both sides are
 * tested against the same ten strings measured in the rendered page, and a constant edited on one
 * side turns the other side's tests red on the next run. See spec/i18n.md.
 */

/** A Han character or a kana, which draw a full square. */
const PX_WIDE = 16;
/** A Hangul syllable, which is a wide character that does not fill its square. */
const PX_HANGUL = 14;
/** Everything else, charged above the widest Latin string observed rather than at its average. */
const PX_NARROW = 8.6;

function isHangul(code: number): boolean {
	return (
		(code >= 0xac00 && code <= 0xd7a3) ||
		(code >= 0x1100 && code <= 0x11ff) ||
		(code >= 0x3130 && code <= 0x318f) ||
		(code >= 0xa960 && code <= 0xa97f) ||
		(code >= 0xd7b0 && code <= 0xd7ff)
	);
}

/**
 * Whether a code point is East Asian Wide or Fullwidth.
 *
 * The ranges that actually occur in this corpus and in any language it is translated into, rather
 * than the whole Unicode table: CJK ideographs and their extensions, the kana, Hangul, the
 * fullwidth forms, and the CJK punctuation that ends a sentence.
 */
function isWide(code: number): boolean {
	return (
		(code >= 0x1100 && code <= 0x115f) ||
		(code >= 0x2e80 && code <= 0x303e) ||
		(code >= 0x3041 && code <= 0x33ff) ||
		(code >= 0x3400 && code <= 0x4dbf) ||
		(code >= 0x4e00 && code <= 0x9fff) ||
		(code >= 0xa000 && code <= 0xa4cf) ||
		(code >= 0xac00 && code <= 0xd7a3) ||
		(code >= 0xf900 && code <= 0xfaff) ||
		(code >= 0xfe30 && code <= 0xfe6f) ||
		(code >= 0xff00 && code <= 0xff60) ||
		(code >= 0xffe0 && code <= 0xffe6) ||
		(code >= 0x20000 && code <= 0x3fffd)
	);
}

export function pixels(text: string): number {
	let total = 0;
	for (const character of text.trim()) {
		const code = character.codePointAt(0) ?? 0;
		if (isHangul(code)) total += PX_HANGUL;
		else if (isWide(code)) total += PX_WIDE;
		else total += PX_NARROW;
	}
	return total;
}

/**
 * The width an article title is drawn into on a phone.
 *
 * The article column is 354px inside the page padding on an iPhone 17 Pro, of which the title
 * takes 85%. Wider than the 186px a card title has, because a card shares its row with a dotted
 * leader and a date while this has the column to itself -- so a short title, written to the
 * card, always clears this. See `width::budget::ARTICLE_TITLE`.
 */
export const ARTICLE_TITLE_BUDGET = 300;

export function fits(text: string, budget: number): boolean {
	return pixels(text) <= budget;
}
