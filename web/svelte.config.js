import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    // The web app ships as a container, like the Rust services.
    adapter: adapter(),
    // Makes `version` from $app/environment the package version, so a page
    // can say which build it is.
    version: { name: pkg.version },
    // One .env at the repository root serves the Rust services and the web
    // app, so there is a single place to look when a setting is missing.
    // This is what $env/* reads; Vite's own envDir (vite.config.ts) only
    // covers import.meta.env.
    env: { dir: '..' }
  }
};
