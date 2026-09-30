/**
 * Talking to the control plane.
 *
 * The browser never calls the API directly: it only ever talks to this app,
 * which calls the control plane server to server and relays what comes back.
 * That means no CORS, no second origin to reason about, and one place where
 * the session cookie is handled.
 *
 * The control plane owns sessions and the database. `docs/architecture.md`
 * puts sign-in checking at the edge, which does not exist locally; see
 * `docs/decisions.md`.
 */

import { env } from '$env/dynamic/private';
import type { RequestEvent } from '@sveltejs/kit';

/** Where the control plane is. */
export function apiUrl(): string {
  return (env.CRONCAVE_API_URL || 'http://localhost:8080').replace(/\/$/, '');
}

/**
 * Call the control plane, carrying this browser's cookies with the request.
 *
 * Returns the raw response so callers decide what a status means; the API's
 * 401 is often a normal answer ("nobody is signed in"), not a failure.
 */
export async function api(
  event: RequestEvent,
  path: string,
  init: RequestInit = {}
): Promise<Response> {
  const cookie = event.request.headers.get('cookie');
  const headers = new Headers(init.headers);
  if (cookie) {
    headers.set('cookie', cookie);
  }

  return event.fetch(`${apiUrl()}${path}`, { ...init, headers, redirect: 'manual' });
}

/** Call the control plane with a JSON body. */
export function apiJson(
  event: RequestEvent,
  path: string,
  method: string,
  body: unknown
): Promise<Response> {
  return api(event, path, {
    method,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
}

/**
 * Copy any `Set-Cookie` the control plane issued onto our own response, so a
 * session opened there lands in the browser on this origin.
 */
export function relayCookies(from: Response, to: Headers): void {
  // getSetCookie keeps multiple cookies separate; joining them would corrupt
  // any cookie whose value contains a comma.
  for (const cookie of from.headers.getSetCookie()) {
    to.append('set-cookie', cookie);
  }
}
