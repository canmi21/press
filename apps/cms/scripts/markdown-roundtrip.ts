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
import {
	remarkAddOrderInListPlugin,
	remarkHtmlTransformer,
	remarkInlineLinkPlugin,
	remarkLineBreak,
	remarkMarker,
	remarkPreserveEmptyLinePlugin,
	schema as commonmarkSchema,
} from '@milkdown/preset-commonmark';

// Node's native TypeScript loader requires the source suffix, while the repository's bundler
// module resolution rejects it in static imports. The URL keeps the runtime path explicit and
// the type-only import keeps the shared extension surface checked.
const { markdownExtensions } = (await import(
	new URL('../client/markdown.ts', import.meta.url).href
)) as typeof import('../client/markdown');

const articlePaths = [
	'contents/architecture/compile-time-rendering.md',
	'contents/development/rust-cargo-cranelift-tuning.md',
	'contents/milestone/less-is-more.md',
	'contents/mirror/less-than-an-hour.md',
] as const;

const fixturePath = 'apps/cms/scripts/fixtures/custom-blocks.md';

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
		...markdownExtensions,
	];

	const handlers = plugins.map((plugin) => plugin(ctx.produce()));
	await Promise.all(handlers.map((handler) => handler()));

	return (markdown: string) => {
		const doc = ctx.get(parserCtx)(markdown);
		// Paragraph serialization only needs this state lookup; no DOM editor view is created.
		ctx.set(editorViewCtx, { state: { doc } } as never);
		return { doc, markdown: ctx.get(serializerCtx)(doc) };
	};
}

function assertCustomNodes(doc: any) {
	const directiveTypes = new Set<string>();
	let fontFence: any;

	doc.descendants((node: any) => {
		if (
			node.type.name === 'container_directive' ||
			node.type.name === 'leaf_directive' ||
			node.type.name === 'text_directive'
		) {
			directiveTypes.add(node.type.name);
			if (node.attrs.name !== 'font' || node.attrs.attributes.family !== 'georgia') {
				throw new Error(`${node.type.name} did not retain its font name and attributes`);
			}
		}

		if (node.type.name === 'code_block' && node.attrs.parameters?.name === 'font') {
			fontFence = node;
		}
	});

	for (const type of ['container_directive', 'leaf_directive', 'text_directive']) {
		if (!directiveTypes.has(type)) throw new Error(`fixture did not produce ${type}`);
	}

	if (
		fontFence?.attrs.language !== '{font' ||
		fontFence?.attrs.meta !== 'georgia}' ||
		fontFence?.attrs.parameters.values.length !== 1 ||
		fontFence?.attrs.parameters.values[0] !== 'georgia'
	) {
		throw new Error('fixture did not produce structured font fence parameters');
	}
}

function assertUnavailableFontIsRejected(
	roundTrip: (markdown: string) => unknown,
	markdown: string,
	source: string,
) {
	try {
		roundTrip(markdown);
	} catch (error) {
		if (error instanceof Error && error.message.includes('unavailable font family')) return;
		throw error;
	}

	throw new Error(`${source} accepted an unavailable font family`);
}

const roundTrip = await createRoundTripper();
const temporaryDirectory = await mkdtemp(join(tmpdir(), 'cms-markdown-roundtrip-'));
let results: { identical: boolean; report: string }[] = [];
let fixtureIdentical = false;

try {
	results = await Promise.all(
		articlePaths.map(async (relativePath) => {
			const sourcePath = join(repositoryRoot, relativePath);
			const source = await readFile(sourcePath, 'utf8');
			const { markdown: output } = roundTrip(source);

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

	const fixture = await readFile(join(repositoryRoot, fixturePath), 'utf8');
	const fixtureResult = roundTrip(fixture);
	fixtureIdentical = fixture === fixtureResult.markdown;
	assertCustomNodes(fixtureResult.doc);
	assertUnavailableFontIsRejected(
		roundTrip,
		':font[Unavailable]{family="comic-sans"}\n',
		'font directive',
	);
	assertUnavailableFontIsRejected(
		roundTrip,
		'```{font comic-sans}\nUnavailable\n```\n',
		'font fence',
	);
} finally {
	await rm(temporaryDirectory, { recursive: true });
}

for (const { report } of results) console.log(report);
const failures = results.filter(({ identical }) => !identical).length;
console.log(`\n${articlePaths.length - failures} identical; ${failures} different`);
console.log(`${fixtureIdentical ? 'IDENTICAL' : 'DIFFERENT'} ${fixturePath}`);
process.exitCode = failures === 0 && fixtureIdentical ? 0 : 1;
