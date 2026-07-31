<script lang="ts">
	type Props = {
		kind: string;
		meta?: Record<string, string | undefined>;
	};
	let { kind, meta }: Props = $props();
</script>

<div
	class="rounded-xl border-2 border-dashed border-border bg-paper p-4 font-mono text-sm text-text-soft"
>
	<div class="text-text">::{kind}</div>
	{#if meta}
		{#each Object.entries(meta) as [k, v]}
			{#if v !== undefined}
				<div class="mt-1 text-xs">{k} = "{v}"</div>
			{/if}
		{/each}
	{/if}
</div>
