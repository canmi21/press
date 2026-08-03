import { describe, expect, it } from 'vitest';
import { langColor, parseTokei } from './tokei';

// A trimmed capture of real `tokei` output, keeping the shapes that matter: the rules, the
// column header, an embedded language, and the totals row.
const OUTPUT = `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Language              Files        Lines         Code     Comments       Blanks
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 CSS                       5          152          113           16           23
 TypeScript              390        36643        30350         1627         4666
 Vue                       3          210          160           20           30
 |- HTML                   3           90           80            5            5
 Plain Text                2          100            0          100            0
─────────────────────────────────────────────────────────────────────────────────
 Total                   403        37205        30703         1768         4724
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`;

describe('tokei output', () => {
	it('keeps the languages and drops the furniture', () => {
		const stats = parseTokei(OUTPUT);
		expect(stats.map((s) => s.lang)).toEqual(['CSS', 'TypeScript', 'Vue', 'Plain Text']);
		// The totals row would otherwise read as a language called Total and dominate every chart.
		expect(stats.some((s) => s.lang === 'Total')).toBe(false);
	});

	it('attaches an embedded language to its host rather than counting it beside it', () => {
		const vue = parseTokei(OUTPUT).find((s) => s.lang === 'Vue');
		expect(vue?.nested.map((n) => n.lang)).toEqual(['HTML']);
		// The host keeps its own totals; the nested row is detail, not a sibling.
		expect(vue?.lines).toBe(210);
	});

	it('reads a language whose name contains a space', () => {
		const plain = parseTokei(OUTPUT).find((s) => s.lang === 'Plain Text');
		expect(plain).toMatchObject({ files: 2, lines: 100, code: 0, comments: 100 });
	});

	it('colours a language the same way wherever it is rendered', () => {
		// The original handed colours out in arrival order from a module-level counter, so the
		// server and the browser produced different charts and the page changed on hydration.
		expect(langColor('Rust')).toBe('#dea584');
		const first = langColor('Brainfuck');
		expect(langColor('Brainfuck')).toBe(first);
		// Order of other calls must not move it.
		langColor('Whitespace');
		langColor('Malbolge');
		expect(langColor('Brainfuck')).toBe(first);
	});
});
