import { version } from '$app/environment';
import { env } from '$env/dynamic/private';
import { parseEnvironment } from '$lib/environment';
import { DATA_COLLECTION } from '$lib/sentry';
import { BOOT_SCRIPT } from '$lib/theme/theme';
import { handleErrorWithSentry, init, sentryHandle } from '@sentry/sveltekit';
import { sequence } from '@sveltejs/kit/hooks';
import type { Handle } from '@sveltejs/kit';

// Same rule as the Rust services: no DSN means error reporting is off, so
// local runs and CI need no account and no secret. See
// docs/decisions.md.
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

/**
 * Stamp the theme on the document before the browser paints, so nobody sees
 * a flash of light before dark loads. It has to be inline and first, which is
 * why it is a string rather than a module.
 */
const theme: Handle = ({ event, resolve }) =>
  resolve(event, {
    transformPageChunk: ({ html }) =>
      html.replace('%croncave.boot%', `<script>${BOOT_SCRIPT}</script>`)
  });

export const handle = sequence(sentryHandle(), theme);
