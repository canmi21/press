<script lang="ts">
	import { page } from '$app/state';
	import * as m from '$lib/paraglide/messages';
	import type { LocaleCode } from '$lib/locale';

	// The view being rendered, read off what the server stamped. An error page still answers in
	// the language the reader asked for. See spec/locale.md.
	const locale = $derived((page.data.locale?.code ?? 'mw') as LocaleCode);

	// Left in English on purpose. These are the protocol's own names for its statuses, printed
	// beside the number they belong to, and a reader who meets `404` meets `Not Found` with it
	// everywhere else on the web. The sentence below is the part written for a person.
	const STATUS_TEXT: Record<number, string> = {
		400: 'Bad Request',
		401: 'Unauthorized',
		403: 'Forbidden',
		404: 'Not Found',
		405: 'Method Not Allowed',
		410: 'Gone',
		429: 'Too Many Requests',
		500: 'Internal Server Error',
		502: 'Bad Gateway',
		503: 'Service Unavailable',
		504: 'Gateway Timeout',
	};

	const message = $derived(
		page.status === 404
			? m['error.not-found']({}, { locale })
			: (page.error?.message ?? m['error.unexpected']({}, { locale })),
	);
	const titleText = $derived(STATUS_TEXT[page.status] ?? 'Error');
</script>

<svelte:head>
	<title>{page.status} {titleText}</title>
</svelte:head>

<main class="flex min-h-screen items-center justify-center px-6">
	<div class="flex items-center">
		<h1 class="border-r border-border pr-6 text-2xl leading-[3.0625rem] font-medium text-text">
			{page.status}<span class="sr-only"> {titleText}</span>
		</h1>
		<p class="pl-6 text-sm leading-[3.0625rem] text-text">
			{message}
		</p>
	</div>
</main>
