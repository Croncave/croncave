/**
 * Finding the containers Croncave made on this machine.
 */

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const run = promisify(execFile);

const MANAGED = 'label=com.croncave.managed';

/** Where the setup records what already existed. */
export const SNAPSHOT = new URL('../.tmp/containers-before.txt', import.meta.url).pathname;

/** Every container Croncave made, running or not. */
export async function managedContainers(): Promise<Set<string>> {
  try {
    const { stdout } = await run('docker', ['ps', '-aq', '--filter', MANAGED]);
    return new Set(stdout.split('\n').filter(Boolean));
  } catch {
    // No Docker, or no permission: nothing to clear up either way.
    return new Set();
  }
}
