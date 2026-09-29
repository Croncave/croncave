/**
 * Reading a sign-in link the way a person does.
 *
 * In development the link is written to the control plane's log instead of
 * being emailed, so that is where the tests look. Nothing here reaches into
 * the database: only the token's hash is stored, so there is nothing in there
 * to read back — which is the property worth keeping.
 */

import { readFileSync } from 'node:fs';
import { CONTROL_PLANE_LOG } from '../playwright.config';

/** An address nothing else has used, so runs never collide. */
export function freshEmail(): string {
  return `e2e-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@example.com`;
}

/** The most recent sign-in link sent to this address. */
export async function signInLinkFor(email: string, timeoutMs = 10_000): Promise<string> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const link = findLink(email);
    if (link) {
      return link;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error(`no sign-in link for ${email} appeared in ${CONTROL_PLANE_LOG}`);
}

function findLink(email: string): string | null {
  let log: string;
  try {
    log = readFileSync(CONTROL_PLANE_LOG, 'utf8');
  } catch {
    return null;
  }

  for (const line of log.split('\n').reverse()) {
    if (!line.startsWith('{')) continue;

    let event: { email?: string; message?: string };
    try {
      event = JSON.parse(line);
    } catch {
      continue;
    }

    const marker = 'development only): ';
    if (event.email === email && event.message?.includes(marker)) {
      return event.message.slice(event.message.indexOf(marker) + marker.length);
    }
  }

  return null;
}
