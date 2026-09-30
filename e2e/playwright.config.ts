/**
 * End-to-end tests: a real browser, a real control plane, a real Postgres.
 *
 * These live outside web/ on purpose. They exercise the whole system — the
 * browser, the web app and the control plane together — and putting them
 * beside the web app would imply they only test that.
 *
 * Both servers are started here, and the control plane's output is captured
 * to a file because that is where a sign-in link appears in development. The
 * tests read it from there, the same way a person does. There is no test-only
 * endpoint that hands out links: that would be a bypass living in production
 * code, and the whole point of hashing the token is that nothing, including a
 * test, can read one back out of the database.
 */

import { defineConfig, devices } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const APP = 'http://localhost:5273';
const API = 'http://127.0.0.1:8180';

/** Where the control plane's output lands, for reading sign-in links. */
export const CONTROL_PLANE_LOG = new URL('./.tmp/control-plane.log', import.meta.url).pathname;

mkdirSync(new URL('./.tmp/', import.meta.url).pathname, { recursive: true });

/**
 * The same database development uses. Each test signs up with an address of
 * its own, so runs never collide with each other or with anything already
 * there — which is cheaper than creating and dropping a database, and closer
 * to how the thing really behaves.
 */
const DATABASE_URL =
  process.env.DATABASE_URL ?? 'postgres://croncave:croncave@localhost:5432/croncave';

export default defineConfig({
  testDir: './tests',
  globalSetup: './tests/global-setup.ts',
  globalTeardown: './tests/global-teardown.ts',
  // A sign-in link works exactly once, so a retried test must start over
  // rather than reuse anything.
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: APP,
    trace: 'retain-on-failure'
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: [
    {
      // Ports of its own, so a running development stack is left alone.
      command: `sh -c 'exec ../target/debug/croncave-control-plane >> ${CONTROL_PLANE_LOG} 2>&1'`,
      url: `${API}/health`,
      reuseExistingServer: false,
      env: {
        DATABASE_URL,
        // Workspaces are containers, and reach the relay from outside the
        // host's loopback.
        CRONCAVE_BIND: '0.0.0.0:8180',
        CRONCAVE_APP_URL: APP,
        CRONCAVE_ENV: 'ci',
        CRONCAVE_LOG_FORMAT: 'json',
        // A real Docker container, not the fake: the point of these tests is
        // that the whole stack works, and the fake would hide a driver bug.
        CRONCAVE_COMPUTE_DRIVER: process.env.CRONCAVE_COMPUTE_DRIVER ?? 'local',
        // What a workspace dials, from inside its container.
        CRONCAVE_WORKSPACE_RELAY_URL: 'ws://host.docker.internal:8180/agent',
        // Long enough that a test is not raced by the sweeper, short enough
        // that a run does not leave containers awake.
        CRONCAVE_IDLE_SECONDS: '120'
      }
    },
    {
      // Vite is started directly rather than through pnpm. A pnpm wrapper
      // exits without passing the signal on, orphaning the dev server, and
      // Playwright then waits forever for a port that never frees.
      command: 'node node_modules/vite/bin/vite.js dev --port 5273 --strictPort',
      cwd: new URL('../web/', import.meta.url).pathname,
      url: `${APP}/sign-in`,
      reuseExistingServer: false,
      env: {
        CRONCAVE_API_URL: API,
        CRONCAVE_ENV: 'ci'
      }
    }
  ]
});
