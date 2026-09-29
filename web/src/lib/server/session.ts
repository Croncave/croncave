/**
 * Who is signed in, from the control plane.
 *
 * The web app keeps no session state of its own: it asks, carrying the
 * browser's cookie. One source of truth, and signing out takes effect
 * everywhere at once because there is no second copy to go stale.
 */

import { api } from '$lib/server/api';
import type { RequestEvent } from '@sveltejs/kit';

export type Me = {
  user: { id: string; email: string; name: string | null };
  team: { id: string; name: string };
};

/** The signed-in person, or null. */
export async function currentUser(event: RequestEvent): Promise<Me | null> {
  const response = await api(event, '/me');

  // 401 is a normal answer here, not a failure: it means nobody is signed in.
  if (response.status === 401) {
    return null;
  }

  if (!response.ok) {
    throw new Error(`the control plane answered ${response.status} for /me`);
  }

  return (await response.json()) as Me;
}
