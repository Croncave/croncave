import { apiJson } from '$lib/server/api';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async (event) => {
    const form = await event.request.formData();
    const name = String(form.get('name') ?? '').trim();

    if (!name) {
      return fail(400, { problem: 'Give the workspace a name.' });
    }

    const response = await apiJson(event, '/workspaces', 'POST', { name });

    if (response.status === 400) {
      return fail(400, { problem: 'That name is too long. Keep it under 100 characters.', name });
    }

    if (response.status === 401) {
      redirect(303, '/sign-in');
    }

    if (!response.ok) {
      return fail(502, {
        problem: "We couldn't create that just now. Try again in a moment.",
        name
      });
    }

    redirect(303, '/');
  }
};
