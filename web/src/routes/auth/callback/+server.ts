/**
 * Where a sign-in link lands.
 *
 * The link in the email points at this app, not at the control plane, so the
 * session cookie is set on the origin the person actually browses. This hands
 * the token over, relays the cookie, and sends them on.
 */

import { api, relayCookies } from '$lib/server/api';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async (event) => {
  const token = event.url.searchParams.get('token') ?? '';
  const response = await api(event, `/auth/callback?token=${encodeURIComponent(token)}`);

  const headers = new Headers();
  relayCookies(response, headers);

  // The control plane answers 303 to its own app URL; we decide where to go,
  // so a bad link returns to sign-in with something to read rather than a
  // bare error.
  const ok = response.status >= 200 && response.status < 400;
  headers.set('location', ok ? '/' : '/sign-in?link=expired');

  return new Response(null, { status: 303, headers });
};
