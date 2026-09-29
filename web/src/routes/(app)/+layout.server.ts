/**
 * Everything behind sign-in.
 *
 * Checking here rather than in each page means a new page under this group is
 * protected by existing, not by remembering.
 */

import { api } from '$lib/server/api';
import { currentUser } from '$lib/server/session';
import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

export type Workspace = {
  id: string;
  name: string;
  state: string;
  created_at: string;
};

export const load: LayoutServerLoad = async (event) => {
  const me = await currentUser(event);

  if (!me) {
    redirect(303, '/sign-in');
  }

  const response = await api(event, '/workspaces');
  const workspaces: Workspace[] = response.ok ? await response.json() : [];

  return { me, workspaces };
};
