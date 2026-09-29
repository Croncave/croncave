/**
 * The form action behind both sign-in and sign-up.
 *
 * There is one endpoint on the control plane for both, because with a link
 * there is no difference: following it signs you in, creating the account the
 * first time. The two screens differ only in what they say.
 */

import { apiJson } from '$lib/server/api';
import { fail } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';

export async function requestSignInLink(event: RequestEvent) {
  const form = await event.request.formData();
  const email = String(form.get('email') ?? '').trim();

  if (!email) {
    return fail(400, { problem: 'Enter your email address.' });
  }

  const response = await apiJson(event, '/auth/request-link', 'POST', { email });

  if (response.status === 400) {
    return fail(400, { problem: "That doesn't look like an email address.", email });
  }

  if (!response.ok) {
    // Never blame the address: the control plane answers identically whether
    // or not an account exists, and a different message here would undo that.
    return fail(502, { problem: "We couldn't send that just now. Try again in a moment." });
  }

  return { sent: email };
}
