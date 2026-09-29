import { api } from '$lib/server/api';
import { error, fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export type Workspace = {
  id: string;
  name: string;
  state: string;
  created_at: string;
};

export const load: PageServerLoad = async (event) => {
  const response = await api(event, `/workspaces/${event.params.id}`);

  if (response.status === 404) {
    error(404, 'No such workspace');
  }

  if (!response.ok) {
    error(502, "We couldn't reach the workspace just now.");
  }

  return { workspace: (await response.json()) as Workspace };
};

/** Start and stop go through the control plane, which owns the provider. */
async function run(event: Parameters<Actions[string]>[0], what: 'start' | 'stop') {
  const response = await api(event, `/workspaces/${event.params.id}/${what}`, { method: 'POST' });

  if (response.status === 404) {
    error(404, 'No such workspace');
  }

  // 502 means the provider is the problem, not the request — worth saying so
  // rather than showing a generic failure.
  if (response.status === 502) {
    return fail(502, {
      problem:
        what === 'start'
          ? "We couldn't reach the machines that run workspaces. Try again in a moment."
          : "We couldn't reach the machines that run workspaces, so it may still be awake."
    });
  }

  if (!response.ok) {
    return fail(500, { problem: 'Something went wrong.' });
  }

  return { ok: true };
}

export const actions: Actions = {
  start: (event) => run(event, 'start'),
  stop: (event) => run(event, 'stop')
};
