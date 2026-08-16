<script lang="ts">
	import * as m from '$lib/paraglide/messages';
	import type { LocaleCode } from '$lib/locale';
	import type { PackageRow } from './directory';

	let { rows, locale, license }: { rows: PackageRow[]; locale: LocaleCode; license?: string } =
		$props();
</script>

<div>
	{#each rows as entry (entry.purl)}
		<a
			href={entry.href}
			class="focus-ring-within -mx-2 grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 rounded-[0.5rem] px-2 py-1 hover:bg-paper-hover focus-visible:outline-none"
		>
			<span class="flex min-w-0 items-baseline gap-2">
				<span class="focus-link-inner min-w-0 truncate text-text-strong">{entry.name}</span>
				<span class="shrink-0 font-mono text-[0.8125rem] text-text-soft">{entry.version}</span>
			</span>
			<span class="flex min-w-0 items-center gap-2">
				{#if entry.asserted}
					<span
						class="shrink-0 rounded-[0.25rem] border border-border px-1 text-[0.75rem] text-text-soft"
						>{m['licenses.asserted']({}, { locale })}</span
					>
				{/if}
				{#if entry.spdx !== license}
					<span class="max-w-56 truncate text-right text-[0.8125rem] text-text-soft"
						>{entry.spdx}</span
					>
				{/if}
			</span>
		</a>
	{/each}
</div>
