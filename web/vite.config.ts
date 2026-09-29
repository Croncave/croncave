import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  // One .env at the repository root serves the Rust services and the web app,
  // so there is a single place to look when a setting is missing.
  envDir: '..',
  test: {
    include: ['src/**/*.test.ts']
  }
});
