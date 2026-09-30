/**
 * What the Sentry SDK is allowed to collect.
 *
 * Version 11 replaced the single `sendDefaultPii` switch with `dataCollection`,
 * and its defaults are permissive: cookies, request and response headers,
 * request bodies, query parameters, database query data and the values of
 * local variables are all collected unless you say otherwise.
 *
 * Croncave runs people's private workspaces, so we turn all of that off and
 * send only what we put in a report ourselves — the same posture as
 * `send_default_pii: false` on the Rust side. See
 * `docs/decisions.md`.
 *
 * Source context lines are left at their default: that is our own code, and
 * it is what makes a stack trace readable.
 */
export const DATA_COLLECTION = {
  userInfo: false,
  cookies: false,
  httpHeaders: false,
  httpBodies: [],
  urlQueryParams: false,
  databaseQueryData: false,
  queues: false,
  stackFrameVariables: false
};
