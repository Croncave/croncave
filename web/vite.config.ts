import { sentrySvelteKit } from '@sentry/sveltekit/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [
    // Must come before sveltekit(). Source maps are not uploaded: that needs
    // an auth token, and belongs with the deploy pipeline, not step 0.
    sentrySvelteKit({ autoUploadSourceMaps: false }),
    sveltekit()
  ],
  // One .env at the repository root serves the Rust services and the web app,
  // so there is a single place to look when a setting is missing.
  envDir: '..',
  test: {
    include: ['src/**/*.test.ts']
  }
});
