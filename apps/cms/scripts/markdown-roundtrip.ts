import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Clock, Container, Ctx } from '@milkdown/ctx';
import {
	config,
	Editor,
	editorViewCtx,
	init,
	parser,
	parserCtx,
	remarkStringifyOptionsCtx,
	schema,
	serializer,
	serializerCtx,
} from '@milkdown/core';
import { $nodeSchema, $remark } from '@milkdown/utils';
import remarkFrontmatter from 'remark-frontmatter';
import {
	remarkAddOrderInListPlugin,
	remarkHtmlTransformer,
	remarkInlineLinkPlugin,
	remarkLineBreak,
	codeBlockSchema,
	remarkMarker,
	remarkPreserveEmptyLinePlugin,
	schema as commonmarkSchema,
} from '@milkdown/preset-commonmark';

// Without this, `---\ntitle: ...\n---` parses as a thematic break, a paragraph and a setext
// heading underline, and serializing that back writes the closing fence as a long run of dashes.
// The block is kept as an opaque node rather than being understood, which is what preserves the
// author's own YAML spelling.
const frontmatterPlugin = $remark('frontmatter', () => remarkFrontmatter, ['yaml']);

// Parsing frontmatter is only half of it: mdast then carries a `yaml` node that the ProseMirror
// schema has nowhere to put, and the transformer refuses a node it cannot match. So the block
// gets a node of its own, holding the author's YAML verbatim as an atom. Nothing here understands
// the YAML -- not reordering or requoting it is the entire point, since `cms i18n` hashes
// frontmatter values into segment ids.
//
// This is the same shape every custom block will need: an mdast field, a schema node or attribute
// to hold it, and a serializer that writes it back. See spec/tasks.md.
const frontmatterNode = $nodeSchema('frontmatter', () => ({
	atom: true,
	group: 'block',
	attrs: { value: { default: '' } },
	parseDOM: [{ tag: 'div[data-frontmatter]' }],
	toDOM: (node: { attrs: { value: string } }) => ['div', { 'data-frontmatter': node.attrs.value }],
	parseMarkdown: {
		match: ({ type }: { type: string }) => type === 'yaml',
		runner: (state: any, node: any, type: any) => {
			state.addNode(type, { value: node.value ?? '' });
		},
	},
	toMarkdown: {
		match: (node: any) => node.type.name === 'frontmatter',
		runner: (state: any, node: any) => {
			state.addNode('yaml', undefined, node.attrs.value);
		},
	},
}));

// The stock code_block schema keeps only `language`, so everything after the language word --
// ```tokei title="Seam language statistics" -- is read, dropped on the way into ProseMirror, and
// never written back. That string is not decoration: it is where this repository is about to put
// the parameters of its own block types. Overriding the node adds a `meta` attribute beside the
// language and writes both back.
const codeBlockWithMeta = codeBlockSchema.extendSchema((prev) => (ctx) => {
	const base = prev(ctx);
	return {
		...base,
		attrs: { ...base.attrs, meta: { default: '' } },
		parseMarkdown: {
			match: ({ type }: { type: string }) => type === 'code',
			runner: (state: any, node: any, type: any) => {
				state.openNode(type, { language: node.lang ?? '', meta: node.meta ?? '' });
				if (node.value) state.addText(node.value);
				state.closeNode();
			},
		},
		toMarkdown: {
			match: (node: any) => node.type.name === 'code_block',
			runner: (state: any, node: any) => {
				// `lang` and `meta` are separate mdast fields and have to stay that way. Packing
				// both into `lang` makes remark escape the spaces -- a space terminates the
				// language word -- and the fence comes back as `tokei&#x20;title=...`.
				state.addNode('code', undefined, node.content.firstChild?.text ?? '', {
					lang: node.attrs.language || null,
					meta: node.attrs.meta || null,
				});
			},
		},
	};
});

const articlePaths = [
	'contents/architecture/compile-time-rendering.md',
	'contents/development/rust-cargo-cranelift-tuning.md',
	'contents/milestone/less-is-more.md',
	'contents/mirror/less-than-an-hour.md',
] as const;

const repositoryRoot = fileURLToPath(new URL('../../../', import.meta.url));

const preserveStringifyOptions = {
	bullet: '-',
	bulletOrdered: '.',
	bulletOther: '*',
	closeAtx: false,
	emphasis: '*',
	fence: '`',
	fences: true,
	incrementListMarker: true,
	listItemIndent: 'one',
	quote: '"',
	resourceLink: true,
	rule: '-',
	ruleRepetition: 3,
	ruleSpaces: false,
	setext: false,
	strong: '*',
	tightDefinitions: false,
} as const;

// Milkdown timers use browser event globals even when only its parser and serializer run.
const eventTarget = new EventTarget();
Object.assign(globalThis, {
	addEventListener: eventTarget.addEventListener.bind(eventTarget),
	removeEventListener: eventTarget.removeEventListener.bind(eventTarget),
	dispatchEvent: eventTarget.dispatchEvent.bind(eventTarget),
});

function arraysEqual(left: readonly string[], right: readonly string[]) {
	return left.length === right.length && left.every((value, index) => value === right[index]);
}

function extractFrontmatter(markdown: string) {
	return markdown.match(/^---\n[\s\S]*?\n---(?:\n|$)/)?.[0] ?? '';
}

type FencedBlock = {
	body: string;
	info: string;
};

function extractFencedBlocks(markdown: string): FencedBlock[] {
	const lines = markdown.split('\n');
	const blocks: FencedBlock[] = [];

	for (let index = 0; index < lines.length; index += 1) {
		const opening = lines[index]?.match(/^(`{3,}|~{3,})(.*)$/);
		if (!opening) continue;

		const marker = opening[1];
		const info = opening[2];
		if (!marker || info === undefined) continue;

		const body: string[] = [];
		let closing = index + 1;
		for (; closing < lines.length; closing += 1) {
			const line = lines[closing];
			if (line?.match(new RegExp(`^${marker[0]}{${marker.length},}\\s*$`))) break;
			if (line !== undefined) body.push(line);
		}

		blocks.push({ body: body.join('\n'), info });
		index = closing;
	}

	return blocks;
}

function withoutFencedBlocks(markdown: string) {
	return markdown.replace(/^(`{3,}|~{3,}).*\n[\s\S]*?^\1\s*$/gm, '');
}

function extractRawHtml(markdown: string) {
	return withoutFencedBlocks(markdown).match(/<(?:\/?[A-Za-z][^>\n]*|!--[\s\S]*?--)>/g) ?? [];
}

function extractLineSyntax(markdown: string, pattern: RegExp) {
	return withoutFencedBlocks(markdown)
		.split('\n')
		.flatMap((line) => line.match(pattern) ?? []);
}

function extractEmphasisMarkers(markdown: string) {
	return (
		withoutFencedBlocks(markdown).match(
			/(?<!\\)(?:\*{1,3}|_{1,3}|~~)(?=\S)|(?<=\S)(?:\*{1,3}|_{1,3}|~~)/g,
		) ?? []
	);
}

function extractReferences(markdown: string) {
	return (
		withoutFencedBlocks(markdown).match(
			/!?\[[^\]\n]*\](?:\([^\n)]*\)|\[[^\]\n]*\])?|^\[[^\]\n]+\]:.*$/gm,
		) ?? []
	);
}

function extractEscapes(markdown: string) {
	return withoutFencedBlocks(markdown).match(/\\[!"#$%&'()*+,\-./:;<=>?@[\\\]^_`{|}~]/g) ?? [];
}

function countBlankLines(markdown: string) {
	return markdown.split('\n').filter((line) => line.trim() === '').length;
}

function describeChanges(source: string, output: string) {
	const categories: string[] = [];
	const sourceFences = extractFencedBlocks(source);
	const outputFences = extractFencedBlocks(output);
	const sourceSvg = sourceFences.filter(
		({ info }) => info.trim().split(/\s/, 1)[0] === 'svg-canvas',
	);
	const outputSvg = outputFences.filter(
		({ info }) => info.trim().split(/\s/, 1)[0] === 'svg-canvas',
	);

	if (extractFrontmatter(source) !== extractFrontmatter(output)) {
		categories.push('frontmatter: delimiter, layout, or scalar spelling changed');
	}

	if (
		!arraysEqual(
			sourceSvg.map(({ body }) => body),
			outputSvg.map(({ body }) => body),
		)
	) {
		categories.push('raw HTML or SVG: fenced SVG content changed or was dropped');
	}

	if (!arraysEqual(extractRawHtml(source), extractRawHtml(output))) {
		categories.push('raw HTML or SVG: raw HTML changed or was dropped');
	}

	const fenceInfoChanges = sourceFences.flatMap(({ info }, index) => {
		const next = outputFences[index]?.info;
		return next !== undefined && info !== next
			? [`${JSON.stringify(info.trim())} -> ${JSON.stringify(next.trim())}`]
			: [];
	});
	if (sourceFences.length !== outputFences.length || fenceInfoChanges.length > 0) {
		categories.push(
			`code fences: info/meta changed${fenceInfoChanges.length > 0 ? ` (${fenceInfoChanges.join(', ')})` : ''}`,
		);
	}

	const syntaxChecks = [
		['emphasis', extractEmphasisMarkers(source), extractEmphasisMarkers(output)],
		[
			'bullet',
			extractLineSyntax(source, /^\s*(?:[-+*]|\d+[.)])(?=\s)/),
			extractLineSyntax(output, /^\s*(?:[-+*]|\d+[.)])(?=\s)/),
		],
		[
			'heading',
			extractLineSyntax(source, /^(?:#{1,6})(?=\s|$)/),
			extractLineSyntax(output, /^(?:#{1,6})(?=\s|$)/),
		],
	] as const;
	for (const [name, before, after] of syntaxChecks) {
		if (!arraysEqual(before, after))
			categories.push(`emphasis, bullet, or heading syntax: ${name} markers changed`);
	}

	if (!arraysEqual(extractEscapes(source), extractEscapes(output))) {
		categories.push('escaping: backslash escapes were added or removed');
	}

	const sourceBlankLines = countBlankLines(source);
	const outputBlankLines = countBlankLines(output);
	const sourceTrailing = source.split('\n').filter((line) => /[\t ]+$/.test(line));
	const outputTrailing = output.split('\n').filter((line) => /[\t ]+$/.test(line));
	if (sourceBlankLines !== outputBlankLines || !arraysEqual(sourceTrailing, outputTrailing)) {
		categories.push(
			`blank lines or trailing whitespace: blank-line count ${sourceBlankLines} -> ${outputBlankLines}`,
		);
	}

	if (!arraysEqual(extractReferences(source), extractReferences(output))) {
		categories.push('link or image reference style: spelling changed');
	}

	return {
		categories,
		svgCount: sourceSvg.length,
		svgIdentical: arraysEqual(
			sourceSvg.map(({ body }) => body),
			outputSvg.map(({ body }) => body),
		),
	};
}

async function createRoundTripper() {
	const ctx = new Ctx(new Container(), new Clock());
	ctx.inject(editorViewCtx, {} as never);

	const plugins = [
		schema,
		parser,
		serializer,
		init(Editor.make()),
		config((configCtx) => {
			configCtx.update(remarkStringifyOptionsCtx, (options) => ({
				...options,
				...preserveStringifyOptions,
			}));
		}),
		...commonmarkSchema,
		...remarkAddOrderInListPlugin,
		...remarkHtmlTransformer,
		...remarkInlineLinkPlugin,
		...remarkLineBreak,
		...remarkMarker,
		...remarkPreserveEmptyLinePlugin,
		...frontmatterPlugin,
		...frontmatterNode,
		...codeBlockWithMeta,
	];

	const handlers = plugins.map((plugin) => plugin(ctx.produce()));
	await Promise.all(handlers.map((handler) => handler()));

	return (markdown: string) => {
		const doc = ctx.get(parserCtx)(markdown);
		// Paragraph serialization only needs this state lookup; no DOM editor view is created.
		ctx.set(editorViewCtx, { state: { doc } } as never);
		return ctx.get(serializerCtx)(doc);
	};
}

const roundTrip = await createRoundTripper();
const temporaryDirectory = await mkdtemp(join(tmpdir(), 'cms-markdown-roundtrip-'));
let results: { identical: boolean; report: string }[] = [];

try {
	results = await Promise.all(
		articlePaths.map(async (relativePath) => {
			const sourcePath = join(repositoryRoot, relativePath);
			const source = await readFile(sourcePath, 'utf8');
			const output = roundTrip(source);

			if (source === output) {
				return { identical: true, report: `IDENTICAL ${relativePath}` };
			}

			const { categories, svgCount, svgIdentical } = describeChanges(source, output);
			const report = [
				`DIFFERENT ${relativePath}`,
				...categories.map((category) => `  - ${category}`),
			];
			report.push(
				`  - raw HTML or SVG check: ${svgIdentical ? `all ${svgCount} fenced SVG blocks are byte-identical` : 'failed'}`,
			);

			const outputPath = join(temporaryDirectory, basename(relativePath));
			await writeFile(outputPath, output);
			const difference = spawnSync('diff', ['-u', sourcePath, outputPath], { encoding: 'utf8' });
			if (difference.stdout) report.push(difference.stdout.trimEnd());

			return { identical: false, report: report.join('\n') };
		}),
	);
} finally {
	await rm(temporaryDirectory, { recursive: true });
}

for (const { report } of results) console.log(report);
const failures = results.filter(({ identical }) => !identical).length;
console.log(`\n${articlePaths.length - failures} identical; ${failures} different`);
process.exitCode = failures === 0 ? 0 : 1;
