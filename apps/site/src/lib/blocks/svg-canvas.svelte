<script lang="ts">
	import '@canmi/svg-canvas/style.css';
	import Preview from '$lib/components/preview.svelte';
	import type { LocaleCode } from '$lib/locale';
	import * as m from '$lib/paraglide/messages';

	let { svg, locale }: { svg: string; locale: LocaleCode } = $props();

	// Safe boundary. When the browser HTML-parses a string, certain HTML start tags
	// inside SVG foreign content ("breakout" elements: span, div, p, b, comments…)
	// make the parser close the <svg> and resume HTML parsing, so a diagram that
	// embeds HTML-looking markup would truncate and spill into the page. Instead of
	// dropping that markup, we TRANSLATE it: escape its angle brackets to entities so
	// it renders as the literal text the author typed and can never break out. Real
	// SVG elements aren't in the breakout set, so structure is untouched; the markup
	// you write shows up verbatim. <foreignObject> is left intact (HTML is a valid,
	// self-contained integration point there). Breakout set per the HTML standard.
	const BREAKOUT_TAG =
		/<\/?(?:b|big|blockquote|body|br|center|code|dd|div|dl|dt|em|embed|h[1-6]|head|hr|i|img|li|listing|menu|meta|nobr|ol|p|pre|ruby|s|small|span|strong|strike|sub|sup|table|tt|u|ul|font)\b[^>]*>/gi;
	const COMMENT = /<!--[\s\S]*?-->/g;
	const FOREIGN_OBJECT = /<foreignObject\b[^>]*>[\s\S]*?<\/foreignObject>/gi;

	// An inline handler in a diagram can only call a global, and this app defines none: every
	// module scope is its own. So it is a ReferenceError parked in the corpus, waiting for the
	// first reader to click. `sendPrompt` was exactly that -- nine nodes in one article, throwing
	// for everyone who touched them, invisible until Sentry saw a phone do it.
	//
	// Dropped rather than escaped, which is the opposite of what BREAKOUT_TAG does, because the
	// two carry different things. A breakout tag is markup the author meant a reader to SEE, so
	// it is translated into the literal text they typed. A handler is markup the author meant to
	// RUN; there is no text in it to preserve, and leaving it visible would only publish the
	// broken call. Styling stays untouched, so a `.node` keeps its hover and simply does nothing
	// of its own when clicked.
	//
	// Scoped to start tags rather than the whole string: a diagram is free to print `onclick=`
	// as ordinary label text, and that is prose, not a handler.
	const START_TAG = /<[a-z][^>]*>/gi;
	const EVENT_HANDLER = /\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi;

	const escapeAngles = (part: string) => part.replace(/</g, '&lt;').replace(/>/g, '&gt;');

	function translate(part: string): string {
		return part.replace(COMMENT, escapeAngles).replace(BREAKOUT_TAG, escapeAngles);
	}

	const disarm = (raw: string) => raw.replace(START_TAG, (tag) => tag.replace(EVENT_HANDLER, ''));

	function contain(raw: string): string {
		// Before the split, so a handler is stripped inside <foreignObject> too -- that subtree is
		// left intact for the parser's sake, which is a reason to keep its markup, not its code.
		const source = disarm(raw);
		let out = '';
		let last = 0;
		for (const kept of source.matchAll(FOREIGN_OBJECT)) {
			out += translate(source.slice(last, kept.index)) + kept[0];
			last = kept.index + kept[0].length;
		}
		return out + translate(source.slice(last));
	}

	const safe = $derived(contain(svg));

	/**
	 * The size the author drew the diagram at, read off the root `viewBox`.
	 *
	 * Only its ratio is read, and only to decide which pair of window edges the enlarged diagram
	 * reaches. The size itself is the window's to choose.
	 *
	 * The root tag, not the first `viewBox` in the string -- every one of these diagrams also
	 * carries a `<marker>` with a `viewBox` of its own, and that one is 10 units square.
	 */
	const DRAWN =
		/<svg\b[^>]*?\bviewBox\s*=\s*["']\s*[-\d.]+[\s,]+[-\d.]+[\s,]+([\d.]+)[\s,]+([\d.]+)/i;

	const drawn = $derived.by(() => {
		const found = DRAWN.exec(svg);
		if (!found) return undefined;
		const width = Number(found[1]);
		const height = Number(found[2]);
		return width > 0 && height > 0 ? { width, height } : undefined;
	});
</script>

<!-- The whole drawn area is the control, because there is nothing else in it to press: the
     handlers a diagram may have carried are stripped above, so a node's hover is decoration and
     the press belongs to the diagram as a whole. See spec/styling.md. -->
<Preview
	label={m['diagram.enlarge']({}, { locale })}
	title={m['diagram.title']({}, { locale })}
	closeLabel={m['diagram.close']({}, { locale })}
	width={drawn?.width}
	height={drawn?.height}
>
	<!-- Authored SVG from the tracked corpus, wrapped by contain() above; not reader input.
	     Stated rather than suppressed; see spec/lint-format.md. -->
	{#snippet inline()}
		<div class="svg-canvas">{@html safe}</div>
	{/snippet}
	<!-- The same wrapped source, drawn a second time at size. Both copies carry the diagram's own
	     `<marker id="arrow">`, and a duplicate id resolves to the first in the document: every one
	     of these markers is the same arrowhead, which is what makes that harmless rather than
	     lucky. -->
	{#snippet enlarged()}
		<div class="svg-canvas">{@html safe}</div>
	{/snippet}
</Preview>
