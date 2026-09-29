import { version } from '$app/environment';
import { env } from '$env/dynamic/private';
import { parseEnvironment } from '$lib/environment';
import { DATA_COLLECTION } from '$lib/sentry';
import { handleErrorWithSentry, init, sentryHandle } from '@sentry/sveltekit';

// Same rule as the Rust services: no DSN means error reporting is off, so
// local runs and CI need no account and no secret. See
// docs/decisions/0017-observability.md.
init({
  dsn: env.SENTRY_DSN || undefined,
  release: `web@${version}`,
  environment: parseEnvironment(env.CRONCAVE_ENV),
  // We decide what a report contains, not the SDK.
  dataCollection: DATA_COLLECTION
  // Performance tracing is not part of R1. Leaving tracesSampleRate unset is
  // what disables it; 0 would still turn tracing on and sample none of it.
});

export const handleError = handleErrorWithSentry();
export const handle = sentryHandle();
