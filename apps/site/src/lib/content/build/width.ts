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
/** A character outside the table, which for the nine locales here means none of them. */
const PX_UNKNOWN = 10;

/**
 * The advance of each Latin character at 16px, measured in the rendered page.
 *
 * A table rather than an average, because `i` advances 3.88px and `W` 16.14 and a single figure
 * chosen safely above both charges an ordinary sentence about a tenth more than it draws. Grouped
 * by width so the shape of the font is readable: the narrow uprights together, the round
 * lowercase together, the wide capitals together.
 */
const LATIN: [number, string][] = [
	[16.4, 'W'],
	[15.89, '%'],
	[15.72, '@'],
	[14.6, 'M'],
	[14.56, '…'],
	[14.21, 'm'],
	[13.26, 'w'],
	[12.3, 'Q'],
	[12.27, 'OÓ'],
	[12.1, 'NÑ'],
	[11.96, 'G'],
	[11.91, 'H'],
	[11.84, 'UÚÜ'],
	[11.74, 'CÇ'],
	[11.56, 'AVÀÁ'],
	[11.55, 'D'],
	[11.21, 'X'],
	[11.14, 'Y'],
	[11, 'K'],
	[10.68, '+<=>~'],
	[10.51, 'B'],
	[10.5, '4'],
	[10.45, '&T'],
	[10.37, 'R'],
	[10.34, '$S'],
	[10.32, '0'],
	[10.27, 'P'],
	[10.25, 'Z'],
	[10.22, '#'],
	[10.08, '69ß'],
	[10.07, '8'],
	[10.03, '3'],
	[9.91, 'g'],
	[9.89, 'bdpq'],
	[9.86, '2'],
	[9.67, 'oòóôõö'],
	[9.65, '5EÈÉÊ'],
	[9.63, 'huùúûü'],
	[9.62, 'nñ'],
	[9.43, '7F'],
	[9.4, 'eèéêë'],
	[9.23, 'cç'],
	[9.2, 'Jy'],
	[9.19, 'v'],
	[9.09, 'aàáâãä'],
	[9.05, 'L'],
	[8.95, 'k'],
	[8.94, 'z'],
	[8.92, 'x'],
	[8.62, 's'],
	[8.44, '?'],
	[8.32, '*'],
	[7.91, '"'],
	[7.62, '^'],
	[7.58, '“'],
	[7.53, '”'],
	[7.4, '-_'],
	[7.05, '{}'],
	[6.64, '1'],
	[6.51, 'ï'],
	[6.38, 'r'],
	[5.91, '/'],
	[5.9, '()[]'],
	[5.53, '|'],
	[5.46, 'f'],
	[5.39, '`t'],
	[5.17, '\\'],
	[5.05, ';'],
	[5, '\''],
	[4.87, '!'],
	[4.85, ',.:·'],
	[4.44, '‘’'],
	[4.36, 'IÍ'],
	[4.26, ' '],
	[4.03, 'ijlìíî'],
];

const ADVANCE = new Map<string, number>(
	LATIN.flatMap(([width, chars]) => [...chars].map((c) => [c, width] as [string, number])),
);

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
		else total += ADVANCE.get(character) ?? PX_UNKNOWN;
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
