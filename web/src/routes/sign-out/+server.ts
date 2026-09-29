/**
 * Signing out. A POST, not a link, so nothing can end a session by being
 * fetched or prefetched.
 */

import { api, relayCookies } from '$lib/server/api';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async (event) => {
  const response = await api(event, '/auth/sign-out', { method: 'POST' });

  const headers = new Headers();
  // The control plane clears the cookie; relay that to the browser.
  relayCookies(response, headers);
  headers.set('location', '/sign-in');

  return new Response(null, { status: 303, headers });
};
