import { sentrySvelteKit } from '@sentry/sveltekit/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

/**
 * The design system's components.css opens with an @import of Google Fonts,
 * which is right for a standalone mockup and wrong for the app: we self-host
 * Inter and JetBrains Mono through @fontsource, so that import would make
 * every visitor fetch the same fonts again from a third party.
 *
 * docs/design/ is a snapshot of the design canvas and read-only here, so the
 * rule is dropped as the stylesheet is processed rather than edited in place.
 * It has to be a PostCSS plugin: remote @import rules are hoisted before any
 * Vite transform runs. Remove this once the canvas export stops emitting it.
 */
const dropWebFontImport = {
  postcssPlugin: 'croncave:drop-web-font-import',
  AtRule: {
    import: (rule: { params: string; remove: () => void }) => {
      if (rule.params.includes('fonts.googleapis.com')) {
        rule.remove();
      }
    }
  }
};

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
  css: {
    postcss: { plugins: [dropWebFontImport] }
  },
  server: {
    fs: {
      // src/app.css imports the design system from docs/design/system rather
      // than copying it, so Vite has to be allowed to read above the project
      // root. Limited to that folder.
      allow: ['.', '../docs/design/system']
    }
  },
  test: {
    include: ['src/**/*.test.ts']
  }
});
