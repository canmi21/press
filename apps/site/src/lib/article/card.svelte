<script lang="ts">
	import { shortDate } from '$lib/format';
	import Thumbnail from './thumbnail.svelte';

	let {
		title,
		subtitle,
		created,
		path,
	}: {
		title: string;
		subtitle: string;
		created: string;
		path: string;
	} = $props();

	const date = $derived(shortDate(created));
</script>

<a href="/{path}" class="article-preview group hover:bg-paper-hover focus-visible:outline-none">
	<Thumbnail focusRing />

	<div class="article-preview-copy">
		<!-- Title shares its line with the dotted leader and date, so the leader
		starts at the title's end rather than the (often longer) subtitle below. -->
		<div class="article-preview-heading">
			<h3 class="article-preview-title">{title}</h3>
			<div class="article-preview-leader"></div>
			<time datetime={created} class="article-preview-date">{date}</time>
		</div>
		<p class="article-preview-subtitle">{subtitle}</p>
	</div>
</a>
