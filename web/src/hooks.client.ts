import { version } from '$app/environment';
import { env } from '$env/dynamic/public';
import { parseEnvironment } from '$lib/environment';
import { DATA_COLLECTION } from '$lib/sentry';
import { handleErrorWithSentry, init } from '@sentry/sveltekit';

// The browser only ever sees PUBLIC_-prefixed values, so these mirror
// SENTRY_DSN and CRONCAVE_ENV. Both are blank locally, which turns browser
// error reporting off.
init({
  dsn: env.PUBLIC_SENTRY_DSN || undefined,
  release: `web@${version}`,
  environment: parseEnvironment(env.PUBLIC_CRONCAVE_ENV),
  dataCollection: DATA_COLLECTION
  // Performance tracing is not part of R1. Leaving tracesSampleRate unset is
  // what disables it; 0 would still turn tracing on and sample none of it.
});

export const handleError = handleErrorWithSentry();
