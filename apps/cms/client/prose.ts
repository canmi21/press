/**
 * A segment's markdown, drawn the way the article draws it.
 *
 * A segment is a block of the article's source, so showing it as plain text shows the punctuation
 * of markdown rather than the writing: backticks around code, asterisks around emphasis, a run of
 * hashes where a heading was. Somebody comparing eight translations of one paragraph is reading
 * the prose, and the marks are noise in front of it.
 *
 * The same chain the site compiles articles with -- remark to parse, GFM for the tables and strike
 * the corpus uses, mdast to hast, hast to HTML. Picking a smaller renderer here would mean two
 * answers to what a paragraph looks like, and the one nobody checks would be this one.
 */

import { toHtml } from 'hast-util-to-html';
import { toHast } from 'mdast-util-to-hast';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import { unified } from 'unified';

const parser = unified().use(remarkParse).use(remarkGfm);

/**
 * Schemes a link may keep.
 *
 * Everything else loses its `href` and stays as text. Measured on this chain: raw HTML never
 * survives -- `<script>`, an `onerror` attribute and an `onclick` one are all dropped, because
 * `mdast-util-to-hast` ignores HTML nodes unless asked not to -- but a `javascript:` URL written
 * as an ordinary markdown link comes through the anchor intact. That is the one hole the parser
 * leaves, and this closes it rather than trusting the corpus to contain no such link.
 */
const SAFE_SCHEME = /^(?:https?:|mailto:|#|\/|\.)/i;

type HastNode = {
	tagName?: string;
	properties?: Record<string, unknown>;
	children?: HastNode[];
};

function disarm(node: HastNode): void {
	for (const child of node.children ?? []) disarm(child);
	if (node.tagName !== 'a' || node.properties === undefined) return;
	const href = node.properties['href'];
	if (typeof href === 'string' && !SAFE_SCHEME.test(href.trim())) delete node.properties['href'];
}

/**
 * Markdown to HTML, for text that came out of the corpus.
 *
 * The result is inserted rather than escaped, which is safe for a narrower reason than "the
 * corpus is trusted": this chain does not emit raw HTML at all. What it can emit is a link, so
 * links are the only thing checked.
 */
export function renderMarkdown(source: string): string {
	// The parser's own root, which `toHast` types more narrowly than `runSync` returns. The cast
	// names what this chain already guarantees rather than widening anything.
	const tree = toHast(parser.runSync(parser.parse(source)) as Parameters<typeof toHast>[0]);
	disarm(tree as HastNode);
	return toHtml(tree);
}
