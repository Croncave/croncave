import { requestSignInLink } from '$lib/server/signIn';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ url }) => ({
  // Set when a link turned out to be used or expired.
  expired: url.searchParams.get('link') === 'expired'
});

export const actions: Actions = {
  default: requestSignInLink
};
