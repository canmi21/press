import { dev } from '$app/environment';
import * as Sentry from '@sentry/sveltekit';

Sentry.init({
	dsn: 'https://a7f2f790ed2fa4f8e0c4310d26d9c39f@o4511131162116096.ingest.us.sentry.io/4511380121976832',
	enabled: !dev,
	environment: dev ? 'development' : 'production',
});

export const handleError = Sentry.handleErrorWithSentry();
