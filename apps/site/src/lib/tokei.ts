/**
 * Reading `tokei`'s own table back into numbers.
 *
 * The block in an article is exactly what the tool printed, pasted unchanged, which is the
 * point: the source stays something a person can read in the markdown and check against their
 * terminal. Parsing it back is cheaper than asking anyone to keep a second machine-readable
 * copy in step with it.
 */

export type NestedLang = {
	lang: string;
	lines: number;
	code: number;
	comments: number;
	blanks: number;
};

export type LangStat = {
	lang: string;
	files: number;
	lines: number;
	code: number;
	comments: number;
	blanks: number;
	nested: NestedLang[];
};

/** GitHub's language colours, so a chart here reads the same as a repository page. */
const LANG_COLORS: Record<string, string> = {
	TypeScript: '#3178c6',
	JavaScript: '#f1e05a',
	Rust: '#dea584',
	Go: '#00add8',
	Python: '#3572a5',
	Java: '#b07219',
	'C++': '#f34b7d',
	C: '#555555',
	'C#': '#178600',
	Ruby: '#701516',
	PHP: '#4f5d95',
	Swift: '#f05138',
	Kotlin: '#a97bff',
	Dart: '#00b4ab',
	Scala: '#c22d40',
	Haskell: '#5e5086',
	Lua: '#000080',
	Perl: '#0298c3',
	R: '#198ce7',
	Shell: '#89e051',
	Bash: '#89e051',
	BASH: '#89e051',
	PowerShell: '#012456',
	SQL: '#e38c00',
	HTML: '#e34c26',
	CSS: '#563d7c',
	SCSS: '#c6538c',
	Vue: '#41b883',
	Svelte: '#ff3e00',
	TSX: '#3178c6',
	JSX: '#f1e05a',
	JSON: '#a0a0a0',
	YAML: '#cb171e',
	TOML: '#9c4221',
	XML: '#0060ac',
	Markdown: '#083fa1',
	Makefile: '#427819',
	Dockerfile: '#384d54',
	Nix: '#7e7eff',
	Zig: '#ec915c',
	Elixir: '#6e4a7e',
	OCaml: '#3be133',
	Julia: '#a270ba',
	SVG: '#5f5e5a',
	Just: '#534ab7',
};

const FALLBACK_POOL = [
	'#6366f1',
	'#ec4899',
	'#14b8a6',
	'#f97316',
	'#8b5cf6',
	'#06b6d4',
	'#84cc16',
	'#e11d48',
	'#0891b2',
	'#a855f7',
];

/**
 * A colour for a language the table above does not name.
 *
 * Derived from the name rather than handed out in arrival order. The original kept a counter
 * and a cache at module scope, which made a language's colour depend on what had been rendered
 * before it -- the server and the browser walk that in different orders, so the same chart came
 * out in different colours on each and the page changed under the reader on hydration.
 */
function fallbackColor(lang: string): string {
	let hash = 0;
	for (const char of lang) hash = (hash * 31 + char.codePointAt(0)!) % 0xffffffff;
	return FALLBACK_POOL[hash % FALLBACK_POOL.length]!;
}

export function langColor(lang: string): string {
	return LANG_COLORS[lang] ?? fallbackColor(lang);
}

/** Header rules, the column header, and the totals row: everything that is not a language. */
function isFurniture(line: string): boolean {
	const trimmed = line.trim();
	return (
		/^[━─]/.test(trimmed) ||
		/Language\s+Files/.test(line) ||
		/\(Total\)/.test(line) ||
		/^\s*Total\s/.test(line)
	);
}

const NESTED = /^\s*\|-\s+(\S+(?:\s+\S+)*?)\s{2,}(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)/;
const MAIN = /^\s+(\S+(?:\s+\S+)*?)\s{2,}(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)/;

/**
 * Languages in the order the tool printed them, with embedded ones attached to their host.
 *
 * A `|-` row belongs to the language above it -- tokei reports HTML inside a Vue file that way
 * -- so it is nested rather than counted again beside its host.
 */
export function parseTokei(raw: string): LangStat[] {
	const stats: LangStat[] = [];
	let current: LangStat | undefined;

	for (const line of raw.split('\n')) {
		if (isFurniture(line)) continue;

		const nested = NESTED.exec(line);
		if (nested && current) {
			current.nested.push({
				lang: nested[1]!,
				lines: Number(nested[3]),
				code: Number(nested[4]),
				comments: Number(nested[5]),
				blanks: Number(nested[6]),
			});
			continue;
		}

		const main = MAIN.exec(line);
		if (main) {
			current = {
				lang: main[1]!,
				files: Number(main[2]),
				lines: Number(main[3]),
				code: Number(main[4]),
				comments: Number(main[5]),
				blanks: Number(main[6]),
				nested: [],
			};
			stats.push(current);
		}
	}

	return stats.filter((stat) => stat.lines > 0);
}
