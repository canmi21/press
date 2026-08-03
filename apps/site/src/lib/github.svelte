<script lang="ts">
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import Clock from '@lucide/svelte/icons/clock';
	import GitCommitHorizontal from '@lucide/svelte/icons/git-commit-horizontal';
	import GitFork from '@lucide/svelte/icons/git-fork';
	import Scale from '@lucide/svelte/icons/scale';
	import Star from '@lucide/svelte/icons/star';
	import { langColor } from './tokei';
	import type { CardAlign, RepoRecord } from './content/types';

	let {
		repo,
		gitRef,
		title,
		align = 'center',
	}: {
		repo: RepoRecord;
		gitRef?: string;
		title?: string;
		align?: CardAlign;
	} = $props();

	const href = $derived(
		gitRef
			? `https://github.com/${repo.full_name}/tree/${gitRef}`
			: `https://github.com/${repo.full_name}`,
	);
	const displayName = $derived(title || repo.full_name.split('/').at(-1) || repo.full_name);
	const pushed = $derived(
		repo.pushed_at
			? new Intl.DateTimeFormat('en-US', {
					month: 'short',
					day: 'numeric',
					year: 'numeric',
					timeZone: 'UTC',
				}).format(new Date(repo.pushed_at))
			: undefined,
	);

	function count(value: number): string {
		if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, '')}m`;
		if (value >= 1_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, '')}k`;
		return value.toString();
	}

	function repositoryName(value: string): string {
		const name = value.split('/').at(-1) ?? value;
		return name
			.toLowerCase()
			.replace(/[-_]+/g, ' ')
			.replace(/\b\w/g, (character) => character.toUpperCase());
	}
</script>

<a
	{href}
	target="_blank"
	rel="noopener noreferrer"
	class:card-center={align === 'center'}
	class:card-right={align === 'right'}
	class="repo-card group"
>
	<div class="header">
		<span class="name">{title || repositoryName(displayName)}</span>
		<span class="fullname">{repo.full_name}</span>
	</div>

	{#if gitRef}
		<span class="ref">
			<GitCommitHorizontal class="size-3" strokeWidth={2} aria-hidden="true" />
			{gitRef.slice(0, 7)}
		</span>
	{/if}

	{#if repo.description}
		<p class="description">{repo.description}</p>
	{/if}

	<div class="meta">
		{#if repo.language}
			<span class="meta-item">
				<span
					class="language-dot"
					style="background-color: {langColor(repo.language)}"
					aria-hidden="true"
				></span>
				{repo.language}
			</span>
		{/if}
		<span class="meta-item">
			<Star class="size-3.5" strokeWidth={2} aria-hidden="true" />
			<span class="tabular-nums">{count(repo.stars)}</span>
			<span class="sr-only">stars</span>
		</span>
		<span class="meta-item">
			<GitFork class="size-3.5" strokeWidth={2} aria-hidden="true" />
			<span class="tabular-nums">{count(repo.forks)}</span>
			<span class="sr-only">forks</span>
		</span>
		{#if repo.license && repo.license !== 'NOASSERTION'}
			<span class="meta-item">
				<Scale class="size-3.5" strokeWidth={2} aria-hidden="true" />
				{repo.license}
			</span>
		{/if}
		<span class="meta-item">
			<CircleDot class="size-3.5" strokeWidth={2} aria-hidden="true" />
			<span class="tabular-nums">{count(repo.open_issues)}</span>
			<span class="sr-only">open issues</span>
		</span>
		{#if pushed}
			<span class="meta-item">
				<Clock class="size-3.5" strokeWidth={2} aria-hidden="true" />
				{pushed}
			</span>
		{/if}
	</div>

	<span class="corner" aria-hidden="true">
		<ArrowUpRight class="size-4" strokeWidth={2} />
	</span>
</a>

<style>
	.repo-card {
		position: relative;
		display: flex;
		width: 100%;
		max-width: 28rem;
		height: 6.5rem;
		margin-block: 1.8em;
		flex-direction: column;
		gap: 0.35rem;
		overflow: hidden;
		border: 1px solid var(--color-border);
		border-radius: 0.75rem;
		background: var(--color-paper);
		padding: 0.6rem 0.75rem;
		color: inherit;
		text-decoration: none;
	}

	.card-center {
		margin-inline: auto;
	}

	.card-right {
		margin-inline-start: auto;
	}

	.header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.name {
		color: var(--color-text-strong);
		font-size: 0.875rem;
		font-weight: 560;
	}

	.fullname {
		margin-inline-start: 0.4rem;
		color: var(--color-text-soft);
		font-size: 0.6875rem;
	}

	.ref {
		position: absolute;
		top: 0.5rem;
		right: 0.6rem;
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		color: var(--color-text-soft);
		font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
		font-size: 0.6875rem;
	}

	.description {
		display: -webkit-box;
		flex: 1;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		margin: 0;
		color: var(--color-text-soft);
		font-size: 0.78125rem;
		line-height: 1.45;
	}

	.meta {
		display: flex;
		margin-top: auto;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.6rem;
		color: var(--color-text-soft);
		font-size: 0.71875rem;
	}

	.meta-item {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		white-space: nowrap;
	}

	.language-dot {
		display: inline-block;
		width: 0.6rem;
		height: 0.6rem;
		flex-shrink: 0;
		border-radius: 9999px;
	}

	.corner {
		position: absolute;
		right: 0.75rem;
		bottom: 0.75rem;
		color: var(--color-text-soft);
		opacity: 0;
		transition: opacity 200ms ease-out;
	}

	.repo-card:hover .corner,
	.repo-card:focus-visible .corner {
		opacity: 1;
	}

	@media (prefers-reduced-motion: reduce) {
		.repo-card,
		.corner {
			transition: none;
		}
	}
</style>
