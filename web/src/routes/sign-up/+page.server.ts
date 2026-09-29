import { requestSignInLink } from '$lib/server/signIn';
import type { Actions } from './$types';

export const actions: Actions = {
  default: requestSignInLink
};
