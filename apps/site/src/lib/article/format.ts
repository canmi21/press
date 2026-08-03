// Reusable number/unit formatting helpers.

const COMPACT_UNITS = ['', 'k', 'M', 'B', 'T'];

// Compact count: under 1000 stays as the raw integer; at or above, it scales to
// the largest unit that keeps the integer part below 1000, kept to at most two
// decimals with trailing zeros trimmed — 999 -> "999", 1500 -> "1.5k",
// 1420 -> "1.42k", 1000 -> "1k", 1_000_000 -> "1M".
export function formatCompact(n: number): string {
	const sign = n < 0 ? '-' : '';
	let value = Math.abs(n);
	if (value < 1000) return sign + value;

	let unit = 0;
	while (value >= 1000 && unit < COMPACT_UNITS.length - 1) {
		value /= 1000;
		unit++;
	}
	// Two-decimal rounding can lift a value to 1000 (e.g. 999_999 -> "1000k");
	// carry it into the next unit so the integer part stays below 1000.
	if (Number(value.toFixed(2)) >= 1000 && unit < COMPACT_UNITS.length - 1) {
		value /= 1000;
		unit++;
	}
	const text = value.toFixed(2).replace(/\.?0+$/, '');
	return sign + text + COMPACT_UNITS[unit];
}
