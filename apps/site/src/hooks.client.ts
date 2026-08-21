import { dev } from '$app/environment';
import { URLS } from '@canmi/urls';
import * as Sentry from '@sentry/sveltekit';
import type { ClientInit } from '@sveltejs/kit';
import { registerAnalytics } from '$lib/analytics';
import { prepareBrowserRuntime } from '$lib/client/compatibility';
import { withoutLanguageParameter } from '$lib/locale';
import { registerClientStrategy } from '$lib/locale/paraglide';

registerClientStrategy();
registerAnalytics();

Sentry.init({
	dsn: URLS.external.sentry.site,
	enabled: !dev,
	environment: dev ? 'development' : 'production',
});

export const init: ClientInit = prepareBrowserRuntime;
export const handleError = Sentry.handleErrorWithSentry();

function cleanLanguageParameter(): void {
	const replacement = withoutLanguageParameter(new URL(window.location.href));
	if (replacement) history.replaceState(history.state, '', replacement);
}

// The Worker has already selected and persisted the view. Address-bar cleanup is deliberately
// deferred until load, so it can neither block rendering nor race the request that used `lang`.
if (document.readyState === 'complete') cleanLanguageParameter();
else window.addEventListener('load', cleanLanguageParameter, { once: true });
