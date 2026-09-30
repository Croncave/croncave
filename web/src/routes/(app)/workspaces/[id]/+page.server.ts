import { api, apiJson } from '$lib/server/api';
import { error, fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export type Workspace = {
  id: string;
  name: string;
  state: string;
  created_at: string;
  /** Whether its agent has an open connection to us right now. */
  connected: boolean;
};

export type Ran = {
  output: string;
  outcome: string;
  succeeded: boolean;
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
  stop: (event) => run(event, 'stop'),

  /**
   * Run a command in the workspace.
   *
   * A stand-in for the real thing: sessions and scheduled runs replace it in
   * steps 4 and 6. It exists so the connection can be tried by hand.
   */
  run: async (event) => {
    const form = await event.request.formData();
    const command = String(form.get('command') ?? '').trim();

    if (!command) {
      return fail(400, { problem: 'Type a command to run.' });
    }

    // Split on spaces only. Quoting and pipes belong to a shell, and
    // pretending to be one badly is worse than not pretending: ask for `sh
    // -c` if you want a shell.
    const [program, ...args] = command.split(/\s+/);

    const response = await apiJson(event, `/workspaces/${event.params.id}/run`, 'POST', {
      program,
      args
    });

    if (response.status === 409) {
      return fail(409, { problem: 'That workspace is asleep. Wake it first.', command });
    }

    if (!response.ok) {
      return fail(502, { problem: "That didn't run. Try again in a moment.", command });
    }

    return { ran: (await response.json()) as Ran, command };
  }
};
