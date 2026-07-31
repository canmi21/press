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

	const escapeAngles = (m: string) => m.replace(/</g, '&lt;').replace(/>/g, '&gt;');

	function translate(part: string): string {
		return part.replace(COMMENT, escapeAngles).replace(BREAKOUT_TAG, escapeAngles);
	}

	function contain(raw: string): string {
		let out = '';
		let last = 0;
		for (const m of raw.matchAll(FOREIGN_OBJECT)) {
			out += translate(raw.slice(last, m.index)) + m[0];
			last = m.index + m[0].length;
		}
		return out + translate(raw.slice(last));
	}

	const safe = $derived(contain(svg));
</script>

<!-- eslint-disable-next-line svelte/no-at-html-tags -->
<div class="svg-canvas">{@html safe}</div>
