<script lang="ts">
	import '@canmi/svg-canvas/style.css';

	let { svg }: { svg: string } = $props();

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
	// when clicked.
	//
	// Scoped to start tags rather than the whole string: a diagram is free to print `onclick=`
	// as ordinary label text, and that is prose, not a handler.
	const START_TAG = /<[a-z][^>]*>/gi;
	const EVENT_HANDLER = /\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi;

	const escapeAngles = (m: string) => m.replace(/</g, '&lt;').replace(/>/g, '&gt;');

	function translate(part: string): string {
		return part.replace(COMMENT, escapeAngles).replace(BREAKOUT_TAG, escapeAngles);
	}

	const disarm = (raw: string) =>
		raw.replace(START_TAG, (tag) => tag.replace(EVENT_HANDLER, ''));

	function contain(raw: string): string {
		// Before the split, so a handler is stripped inside <foreignObject> too -- that subtree is
		// left intact for the parser's sake, which is a reason to keep its markup, not its code.
		const source = disarm(raw);
		let out = '';
		let last = 0;
		for (const m of source.matchAll(FOREIGN_OBJECT)) {
			out += translate(source.slice(last, m.index)) + m[0];
			last = m.index + m[0].length;
		}
		return out + translate(source.slice(last));
	}

	const safe = $derived(contain(svg));
</script>

<!-- Authored SVG from the tracked corpus, wrapped by contain() above; not reader input.
     Stated rather than suppressed; see spec/lint-format.md. -->
<div class="svg-canvas">{@html safe}</div>
