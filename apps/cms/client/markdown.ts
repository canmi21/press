import { fontFamilies } from '@canmi/fonts';
import { codeBlockSchema } from '@milkdown/preset-commonmark';
import { $nodeSchema, $remark } from '@milkdown/utils';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';

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

// The opaque schema node prevents parsed YAML from being rejected or rewritten. See spec/tasks.md.
const frontmatterPlugin = $remark('frontmatter', () => remarkFrontmatter, ['yaml']);

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

export const markdownExtensions = [
	...frontmatterPlugin,
	...frontmatterNode,
	...directivePlugin,
	...containerDirectiveNode,
	...leafDirectiveNode,
	...textDirectiveNode,
	...codeBlockWithParameters,
];
