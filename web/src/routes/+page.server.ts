import { version } from '$app/environment';
import { env } from '$env/dynamic/private';
import { parseEnvironment } from '$lib/environment';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  service: 'web',
  version,
  // Read at request time, from the same .env the Rust services read.
  environment: parseEnvironment(env.CRONCAVE_ENV)
});
