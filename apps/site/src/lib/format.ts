import { PUBLIC_LANGUAGE, type LocaleCode } from './locale/index.ts';

/**
 * Formatting the site does the same way everywhere, gathered because it was not.
 *
 * Each of these was written out two to five times, and the copies had begun to disagree in the
 * one way a reader can see: the same magnitude was abbreviated `1.5k` on one widget and `1.5K` on
 * another. Nothing was wrong with either; what was wrong was that the page had two answers.
 */

/**
 * The tag `Intl` should be given for a view.
 *
 * The source view has no language of its own, so it borrows English -- it is the article's own
 * words, and the numbers around them still have to be grouped somehow.
 *
 * Derived from `PUBLIC_LANGUAGE` rather than restated. The licence pages carried their own
 * version of this, four times, spelling `tw` out as a special case and passing everything else
 * through bare; it produced identical output, which is exactly why nobody noticed there were two
 * rules for one question.
 */
export function intlLocale(locale: LocaleCode): string {
	return locale === 'mw' ? PUBLIC_LANGUAGE.en : PUBLIC_LANGUAGE[locale];
}

/**
 * A date as the site writes it: `Apr 13, 2026`.
 *
 * **UTC, always.** The day shown has to match the date in the article's frontmatter, and a page
 * prerendered west of Greenwich would otherwise render the day before. That reason lived beside
 * one of the three copies of this and not the other two.
 *
 * English, also always. These are timestamps on cards and article headers rather than prose, and
 * a translated month name beside an untranslated title reads as a mistake rather than a courtesy.
 */
export function shortDate(value: string | number | Date): string {
	return new Intl.DateTimeFormat('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric',
		timeZone: 'UTC',
	}).format(new Date(value));
}

/**
 * A count shortened to fit a stat row or a chart axis: `950`, `1.5k`, `16k`, `2.3M`.
 *
 * Lowercase `k` and uppercase `M`, which is what SI writes and therefore the only pair that is
 * not a house style somebody has to remember. The two copies this replaces had picked opposite
 * conventions.
 *
 * **One decimal below ten thousand and none above it.** Past that the tenth is noise -- nobody
 * reads the `.5` in `15.5k` -- and the shorter label is worth more on an axis, which is where
 * these numbers mostly appear.
 */
export function compactCount(value: number): string {
	if (value >= 1_000_000) return `${trim(value / 1_000_000)}M`;
	if (value >= 10_000) return `${Math.round(value / 1_000)}k`;
	if (value >= 1_000) return `${trim(value / 1_000)}k`;
	return value.toString();
}

/** One decimal, and none at all when it would be a zero. */
function trim(value: number): string {
	return value.toFixed(1).replace(/\.0$/, '');
}
