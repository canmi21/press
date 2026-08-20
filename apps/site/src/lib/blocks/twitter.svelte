<script lang="ts">
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import { URLS } from '@canmi/urls';
	import type { TweetRecord } from '$lib/content/types';
	import { compactCount, shortDate } from '$lib/format';
	import SocialIcon from '$lib/home/icons.svelte';

	let { tweet }: { tweet: TweetRecord } = $props();

	const href = $derived(`${URLS.external.social.twitter}/${tweet.author}/status/${tweet.id}`);
	const date = $derived(shortDate(tweet.created));
</script>

<a {href} target="_blank" rel="noopener noreferrer" class="tweet-card group focus-ring">
	<header class="header">
		<SocialIcon name="twitter" class="size-4" />
		<span class="author">@{tweet.author}</span>
		<span aria-hidden="true" class="separator">·</span>
		<time datetime={tweet.created}>{date}</time>
	</header>

	<p class="tweet-text">{tweet.text}</p>

	<footer class="metrics">
		<span class="metric">
			<svg class="action-icon" viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M1.751 10c0-4.42 3.584-8 8.005-8h4.366c4.49 0 8.129 3.64 8.129 8.13 0 2.96-1.607 5.68-4.196 7.11l-8.054 4.46v-3.69h-.067c-4.49.1-8.183-3.51-8.183-8.01Zm8.005-6c-3.317 0-6.005 2.69-6.005 6 0 3.37 2.77 6.08 6.138 6.01l.351-.01h1.761v2.3l5.087-2.81c1.951-1.08 3.163-3.13 3.163-5.36 0-3.39-2.744-6.13-6.129-6.13H9.756Z"
				/>
			</svg>
			<span class="tabular-nums">{compactCount(tweet.replies)}</span>
			<span class="sr-only"> replies</span>
		</span>
		<span class="metric">
			<svg class="action-icon" viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M4.5 3.88 8.932 8.02 7.568 9.48 5.5 7.55V16c0 1.1.896 2 2 2H13v2H7.5c-2.209 0-4-1.79-4-4V7.55L1.432 9.48.068 8.02 4.5 3.88ZM16.5 6H11V4h5.5c2.209 0 4 1.79 4 4v8.45l2.068-1.93 1.364 1.46-4.432 4.14-4.432-4.14 1.364-1.46 2.068 1.93V8c0-1.1-.896-2-2-2Z"
				/>
			</svg>
			<span class="tabular-nums">{compactCount(tweet.reposts)}</span>
			<span class="sr-only"> reposts</span>
		</span>
		<span class="metric">
			<svg class="action-icon" viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M16.697 5.5c-1.222-.06-2.679.51-3.89 2.16l-.805 1.09-.806-1.09C9.984 6.01 8.526 5.44 7.304 5.5c-1.243.07-2.349.78-2.91 1.91-.552 1.12-.633 2.78.479 4.82 1.074 1.97 3.257 4.27 7.129 6.61 3.87-2.34 6.052-4.64 7.126-6.61 1.111-2.04 1.03-3.7.477-4.82-.561-1.13-1.666-1.84-2.908-1.91Zm4.187 7.69c-1.351 2.48-4.001 5.12-8.379 7.67l-.503.3-.504-.3c-4.379-2.55-7.029-5.19-8.382-7.67-1.36-2.5-1.41-4.86-.514-6.67.887-1.79 2.647-2.91 4.601-3.01 1.651-.09 3.368.56 4.798 2.01 1.429-1.45 3.146-2.1 4.796-2.01 1.954.1 3.714 1.22 4.601 3.01.896 1.81.846 4.17-.514 6.67Z"
				/>
			</svg>
			<span class="tabular-nums">{compactCount(tweet.likes)}</span>
			<span class="sr-only"> likes</span>
		</span>
	</footer>

	<span class="corner" aria-hidden="true">
		<ArrowUpRight class="size-4" strokeWidth={2} />
	</span>
</a>

<style>
	.tweet-card {
		position: relative;
		display: flex;
		width: 100%;
		max-width: 28rem;
		margin-block: 1.8em;
		flex-direction: column;
		gap: 0.7rem;
		border: 0.0625rem solid var(--color-border);
		border-radius: 0.75rem;
		background: var(--color-paper);
		padding: 0.75rem;
		color: inherit;
		text-decoration: none;
		transition:
			background-color 150ms ease-out,
			border-color 150ms ease-out;
	}

	.tweet-card:hover,
	.tweet-card:focus-visible {
		border-color: var(--color-border-strong);
		background: var(--color-paper-hover);
	}

	.header,
	.metrics,
	.metric {
		display: flex;
		align-items: center;
	}

	.header {
		gap: 0.35rem;
		color: var(--color-text-soft);
		font-size: 0.71875rem;
	}

	.author {
		color: var(--color-text-strong);
		font-size: 0.8125rem;
		font-weight: 560;
	}

	.separator {
		margin-inline: 0.05rem;
	}

	.tweet-text {
		margin: 0;
		white-space: pre-wrap;
		color: var(--color-text);
		font-size: 0.875rem;
		line-height: 1.55;
	}

	.metrics {
		gap: 0.9rem;
		color: var(--color-text-soft);
		font-size: 0.71875rem;
	}

	.metric {
		gap: 0.25rem;
	}

	.action-icon {
		width: 0.875rem;
		height: 0.875rem;
		flex: none;
		fill: currentColor;
	}

	.corner {
		position: absolute;
		right: 0.75rem;
		bottom: 0.75rem;
		color: var(--color-text-soft);
		opacity: 0;
		transition: opacity 200ms ease-out;
	}

	.tweet-card:hover .corner,
	.tweet-card:focus-visible .corner {
		opacity: 1;
	}

	@media (prefers-reduced-motion: reduce) {
		.tweet-card,
		.corner {
			transition: none;
		}
	}
</style>
