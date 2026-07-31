<script lang="ts">
	let {
		title,
		subtitle,
		created,
		path
	}: {
		title: string;
		subtitle: string;
		created: string;
		path: string;
	} = $props();

	// Pin UTC so the rendered day matches the authored frontmatter date regardless
	// of where the page is prerendered.
	const date = $derived(
		new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			timeZone: 'UTC'
		}).format(new Date(created))
	);
</script>

<a
	href="/{path}"
	class="group -mx-2 flex items-center gap-3 rounded-[0.875rem] p-2 hover:bg-paper-hover"
>
	<!-- A4-ish sheet. Five bars carry the hand-tuned first-frame widths/gaps; after
	hydration article-list measures the corpus and animates them to a content-derived
	shape (normalized list-wide, see article-list.svelte). -->
	<div
		data-article-icon
		aria-hidden="true"
		class="flex h-[3.4375rem] w-[2.9375rem] shrink-0 flex-col items-start justify-center rounded-[0.4375rem] border border-border bg-paper px-1.5"
	>
		<span data-icon-bar class="rounded-full bg-border-strong" style="width: 1rem; height: 0.1875rem"
		></span>
		<span
			data-icon-bar
			class="rounded-full bg-border-strong"
			style="width: 2rem; height: 0.1875rem; margin-top: 0.5rem"
		></span>
		<span
			data-icon-bar
			class="rounded-full bg-border-strong"
			style="width: 1.5rem; height: 0.1875rem; margin-top: 0.25rem"
		></span>
		<span
			data-icon-bar
			class="rounded-full bg-border-strong"
			style="width: 1.25rem; height: 0.1875rem; margin-top: 0.5rem"
		></span>
		<span
			data-icon-bar
			class="rounded-full bg-border-strong"
			style="width: 0.75rem; height: 0.1875rem; margin-top: 0.25rem"
		></span>
	</div>

	<div class="min-w-0 flex-1">
		<!-- Title shares its line with the dotted leader and date, so the leader
		starts at the title's end rather than the (often longer) subtitle below. -->
		<div class="flex items-center gap-3">
			<h3 class="min-w-0 truncate font-medium text-text-strong">{title}</h3>
			<div class="h-0 flex-1 border-t border-dashed border-border-strong"></div>
			<time datetime={created} class="shrink-0 text-[0.9375rem] text-text-soft">{date}</time>
		</div>
		<p class="truncate text-text-soft">{subtitle}</p>
	</div>
</a>

<style>
	/* Keyboard focus lands the ring on the sheet icon, not the full row: the icon
	is each entry's emblem and the ring hugs its corner. Drop the site-wide row
	ring (unlayered here beats the @layer base default) and redraw the same token
	ring on the icon; the transparent outline keeps a forced-colors fallback. */
	.group:focus-visible {
		outline: none;
		box-shadow: none;
	}

	.group:focus-visible [data-article-icon] {
		/* No page-color moat here: the ring hugs the icon edge (offset 0), so the
		accent reads as flush against the sheet rather than floating off it. */
		--focus-ring-offset: 0px;
		outline: var(--focus-ring-width) solid transparent;
		outline-offset: var(--focus-ring-offset);
		box-shadow:
			0 0 0 var(--focus-ring-offset) var(--focus-ring-gap),
			0 0 0 calc(var(--focus-ring-offset) + var(--focus-ring-width)) var(--focus-ring-color);
	}
</style>
