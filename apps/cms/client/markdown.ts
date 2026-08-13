import { fontFamilies } from '@canmi/fonts';
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
	codeBlockSchema,
	remarkAddOrderInListPlugin,
	remarkHtmlTransformer,
	remarkInlineLinkPlugin,
	remarkPreserveEmptyLinePlugin,
	schema as commonmarkSchema,
} from '@milkdown/preset-commonmark';
import { $nodeSchema, $remark } from '@milkdown/utils';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';
import { isScalar, parseDocument, Scalar, visit } from 'yaml';

type DirectiveAttributes = Record<string, string | null>;

export type FenceParameters = {
	name: string;
	values: string[];
};

type DirectiveNode = {
	type: string;
	name?: string;
	attributes?: DirectiveAttributes;
	children?: any[];
};

type MarkdownDocument = {
	descendants: (visitor: (node: any) => void) => void;
};

export const canonicalStringifyOptions = {
	bullet: '-',
	bulletOrdered: '.',
	bulletOther: '*',
	closeAtx: false,
	emphasis: '*',
	fence: '`',
	fences: true,
	incrementListMarker: false,
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

const fontFamilyIds = new Set(fontFamilies.map(({ id }) => id));

function assertFontFamily(value: unknown, source: string): asserts value is string {
	if (typeof value !== 'string' || !fontFamilyIds.has(value)) {
		throw new Error(`${source} names an unavailable font family: ${String(value)}`);
	}
}

function attributesOf(node: DirectiveNode): DirectiveAttributes {
	return { ...node.attributes };
}

function validateDirective(name: string, attributes: DirectiveAttributes) {
	if (name === 'font') assertFontFamily(attributes.family, 'font directive');
}

function directiveAttrs(node: DirectiveNode) {
	const name = node.name ?? '';
	const attributes = attributesOf(node);
	validateDirective(name, attributes);
	return { name, attributes };
}

function parseFenceParameters(language: string, meta: string): FenceParameters | null {
	// Remark owns the raw split; deriving a second field preserves its exact serialization contract.
	// See spec/tasks.md.
	const name = language.match(/^\{([A-Za-z][\w-]*)$/)?.[1];
	if (!name || !meta.endsWith('}')) return null;

	const value = meta.slice(0, -1).trim();
	return { name, values: value ? value.split(/\s+/) : [] };
}

function validateFence(parameters: FenceParameters | null) {
	if (parameters?.name !== 'font') return;
	if (parameters.values.length !== 1) {
		throw new Error('font fence requires exactly one font family');
	}
	assertFontFamily(parameters.values[0], 'font fence');
}

function domAttributes(kind: string, node: any) {
	return {
		[`data-${kind}-directive`]: node.attrs.name,
		'data-attributes': JSON.stringify(node.attrs.attributes),
	};
}

function parseDomAttributes(dom: HTMLElement) {
	const encoded = dom.getAttribute('data-attributes');
	return {
		name:
			dom.getAttribute('data-container-directive') ??
			dom.getAttribute('data-leaf-directive') ??
			dom.getAttribute('data-text-directive') ??
			'',
		attributes: encoded ? (JSON.parse(encoded) as DirectiveAttributes) : {},
	};
}

// The opaque schema node keeps canonical YAML out of the prose editor. See spec/tasks.md.
const frontmatterPlugin = $remark('frontmatter', () => remarkFrontmatter, ['yaml']);

const normaliseFrontmatterPlugin = $remark('normalise-frontmatter', () => () => (tree: any) => {
	const stack = [tree];
	while (stack.length > 0) {
		const node = stack.pop();
		if (node?.type === 'yaml') {
			const document = parseDocument(String(node.value ?? ''));
			const error = document.errors[0];
			if (error) throw error;
			visit(document, (_key, value) => {
				if (
					isScalar(value) &&
					typeof value.value === 'string' &&
					(value.type === Scalar.QUOTE_DOUBLE || value.type === Scalar.QUOTE_SINGLE)
				) {
					value.type = undefined;
				}
			});
			node.value = document.toString().trimEnd();
		}
		if (Array.isArray(node?.children)) stack.push(...node.children);
	}
});

const normaliseSoftBreaksPlugin = $remark('normalise-soft-breaks', () => () => (tree: any) => {
	const stack = [tree];
	while (stack.length > 0) {
		const node = stack.pop();
		if (node?.type === 'text' && typeof node.value === 'string') {
			node.value = node.value.replace(/[\t ]*(?:\r?\n|\r)[\t ]*/g, ' ');
		}
		if (Array.isArray(node?.children)) stack.push(...node.children);
	}
});

const frontmatterNode = $nodeSchema('frontmatter', () => ({
	atom: true,
	group: 'block',
	attrs: { value: { default: '' } },
	parseDOM: [{ tag: 'div[data-frontmatter]' }],
	toDOM: (node: any) => ['div', { 'data-frontmatter': String(node.attrs.value ?? '') }],
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

const directivePlugin = $remark('directive', () => remarkDirective);

// Milkdown has no generic home for remark's directive nodes, so each form needs a schema node.
// See spec/tasks.md.
const containerDirectiveNode = $nodeSchema('container_directive', () => ({
	content: 'block*',
	group: 'block',
	attrs: { name: { default: '' }, attributes: { default: {} } },
	parseDOM: [
		{
			tag: 'div[data-container-directive]',
			getAttrs: (dom: HTMLElement) => parseDomAttributes(dom),
		},
	],
	toDOM: (node: any) => ['div', domAttributes('container', node), 0],
	parseMarkdown: {
		match: ({ type }: DirectiveNode) => type === 'containerDirective',
		runner: (state: any, node: DirectiveNode, type: any) => {
			state.openNode(type, directiveAttrs(node));
			if (node.children) state.next(node.children);
			state.closeNode();
		},
	},
	toMarkdown: {
		match: (node: any) => node.type.name === 'container_directive',
		runner: (state: any, node: any) => {
			validateDirective(node.attrs.name, node.attrs.attributes);
			state.openNode('containerDirective', undefined, {
				name: node.attrs.name,
				attributes: node.attrs.attributes,
			});
			state.next(node.content);
			state.closeNode();
		},
	},
}));

const leafDirectiveNode = $nodeSchema('leaf_directive', () => ({
	atom: true,
	group: 'block',
	attrs: { name: { default: '' }, attributes: { default: {} } },
	parseDOM: [
		{
			tag: 'div[data-leaf-directive]',
			getAttrs: (dom: HTMLElement) => parseDomAttributes(dom),
		},
	],
	toDOM: (node: any) => ['div', domAttributes('leaf', node)],
	parseMarkdown: {
		match: ({ type }: DirectiveNode) => type === 'leafDirective',
		runner: (state: any, node: DirectiveNode, type: any) => {
			state.addNode(type, directiveAttrs(node));
		},
	},
	toMarkdown: {
		match: (node: any) => node.type.name === 'leaf_directive',
		runner: (state: any, node: any) => {
			validateDirective(node.attrs.name, node.attrs.attributes);
			state.addNode('leafDirective', undefined, undefined, {
				name: node.attrs.name,
				attributes: node.attrs.attributes,
			});
		},
	},
}));

const textDirectiveNode = $nodeSchema('text_directive', () => ({
	content: 'inline*',
	group: 'inline',
	inline: true,
	attrs: { name: { default: '' }, attributes: { default: {} } },
	parseDOM: [
		{
			tag: 'span[data-text-directive]',
			getAttrs: (dom: HTMLElement) => parseDomAttributes(dom),
		},
	],
	toDOM: (node: any) => ['span', domAttributes('text', node), 0],
	parseMarkdown: {
		match: ({ type }: DirectiveNode) => type === 'textDirective',
		runner: (state: any, node: DirectiveNode, type: any) => {
			state.openNode(type, directiveAttrs(node));
			if (node.children) state.next(node.children);
			state.closeNode();
		},
	},
	toMarkdown: {
		match: (node: any) => node.type.name === 'text_directive',
		runner: (state: any, node: any) => {
			validateDirective(node.attrs.name, node.attrs.attributes);
			state.openNode('textDirective', undefined, {
				name: node.attrs.name,
				attributes: node.attrs.attributes,
			});
			state.next(node.content);
			state.closeNode();
		},
	},
}));

const codeBlockWithParameters = codeBlockSchema.extendSchema((prev) => (ctx) => {
	const base = prev(ctx);
	return {
		...base,
		attrs: {
			...base.attrs,
			meta: { default: '' },
			parameters: { default: null },
		},
		parseMarkdown: {
			match: ({ type }: { type: string }) => type === 'code',
			runner: (state: any, node: any, type: any) => {
				const language = node.lang ?? '';
				const meta = node.meta ?? '';
				const parameters = parseFenceParameters(language, meta);
				validateFence(parameters);
				state.openNode(type, { language, meta, parameters });
				if (node.value) state.addText(node.value);
				state.closeNode();
			},
		},
		toMarkdown: {
			match: (node: any) => node.type.name === 'code_block',
			runner: (state: any, node: any) => {
				validateFence(node.attrs.parameters);
				state.addNode('code', undefined, node.content.firstChild?.text ?? '', {
					lang: node.attrs.language || null,
					meta: node.attrs.meta || null,
				});
			},
		},
	};
});

const normaliseMarkdownPlugin = config((ctx) => {
	ctx.update(remarkStringifyOptionsCtx, (options) => ({
		...options,
		...canonicalStringifyOptions,
	}));
});

export const markdownExtensions = [
	normaliseMarkdownPlugin,
	...frontmatterPlugin,
	...normaliseFrontmatterPlugin,
	...normaliseSoftBreaksPlugin,
	...frontmatterNode,
	...directivePlugin,
	...containerDirectiveNode,
	...leafDirectiveNode,
	...textDirectiveNode,
	...codeBlockWithParameters,
];

function installEventTarget() {
	if (typeof globalThis.addEventListener === 'function') return;
	const eventTarget = new EventTarget();
	Object.assign(globalThis, {
		addEventListener: eventTarget.addEventListener.bind(eventTarget),
		removeEventListener: eventTarget.removeEventListener.bind(eventTarget),
		dispatchEvent: eventTarget.dispatchEvent.bind(eventTarget),
	});
}

export async function createMarkdownNormalizer() {
	installEventTarget();
	const ctx = new Ctx(new Container(), new Clock());
	ctx.inject(editorViewCtx, {} as never);

	const plugins = [
		schema,
		parser,
		serializer,
		init(Editor.make()),
		...commonmarkSchema,
		...remarkAddOrderInListPlugin,
		...remarkHtmlTransformer,
		...remarkInlineLinkPlugin,
		...remarkPreserveEmptyLinePlugin,
		...markdownExtensions,
	];
	const handlers = plugins.map((plugin) => plugin(ctx.produce()));
	await Promise.all(handlers.map((handler) => handler()));

	return (markdown: string): { doc: MarkdownDocument; markdown: string } => {
		const doc = ctx.get(parserCtx)(markdown) as MarkdownDocument;
		ctx.set(editorViewCtx, { state: { doc } } as never);
		return { doc, markdown: ctx.get(serializerCtx)(doc as never) };
	};
}
