import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * Throws, so you can see an error reach the log and — with a DSN set —
 * Sentry. It exists only in development; a built app has no such route.
 */
export const load: PageServerLoad = () => {
  if (!dev) {
    error(404, 'Not found');
  }

  throw new Error('smoke test: a deliberate error from the web app');
};
