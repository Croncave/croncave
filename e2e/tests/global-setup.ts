/**
 * Before the run: note which Croncave containers already existed, so the
 * teardown can remove only the ones this run creates and leave a workspace
 * someone is actually using alone.
 */

import { writeFileSync } from 'node:fs';
import { managedContainers, SNAPSHOT } from './managed-containers';

export default async function globalSetup() {
  const existing = await managedContainers();
  writeFileSync(SNAPSHOT, [...existing].join('\n'), 'utf8');
}
