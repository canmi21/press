<script lang="ts">
	let { value }: { value: number } = $props();

	// One cell per digit, with a wider gap where a thousands separator would otherwise go. The
	// grouping is drawn rather than formatted, so no locale supplies a separator character.
	const cells = $derived(
		[...String(Math.max(0, Math.trunc(value)))].map((digit, index, all) => ({
			digit,
			grouped: index > 0 && (all.length - index) % 3 === 0,
		})),
	);
</script>

<!-- The box and its optical alignment are in styles/app.css. -->
<span class="value">
	{#each cells as cell, i (i)}
		<span class="value-cell ml-0.25 w-4.5" class:grouped={cell.grouped}>{cell.digit}</span>
	{/each}
</span>

<style>
	/* Where the comma was. */
	.grouped {
		margin-left: 0.3125rem;
	}
</style>
