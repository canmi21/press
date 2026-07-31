import { dev } from '$app/environment';
import { URLS } from '@canmi/urls';
import * as Sentry from '@sentry/sveltekit';

Sentry.init({
	dsn: URLS.external.sentry.site,
	enabled: !dev,
	environment: dev ? 'development' : 'production',
});

export const handleError = Sentry.handleErrorWithSentry();
