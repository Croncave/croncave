/**
 * After the run: remove the containers it created.
 *
 * A stopped container is a sleeping workspace, which is the product working
 * rather than a leak — but a developer's machine should not collect one per
 * test run.
 */

import { execFile } from 'node:child_process';
import { readFileSync, rmSync } from 'node:fs';
import { promisify } from 'node:util';
import { managedContainers, SNAPSHOT } from './managed-containers';

const run = promisify(execFile);

export default async function globalTeardown() {
  let before = new Set<string>();
  try {
    before = new Set(readFileSync(SNAPSHOT, 'utf8').split('\n').filter(Boolean));
    rmSync(SNAPSHOT, { force: true });
  } catch {
    // No snapshot: remove nothing rather than guess.
    return;
  }

  const ours = [...(await managedContainers())].filter((id) => !before.has(id));
  if (ours.length === 0) {
    return;
  }

  try {
    await run('docker', ['rm', '-f', ...ours]);
    console.log(`removed ${ours.length} container(s) this run created`);
  } catch (error) {
    console.warn('could not remove this run’s containers:', error);
  }
}
