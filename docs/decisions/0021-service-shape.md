# 0021: How a Croncave service starts, reports health and stops

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

The control plane is the first long-running service, and the relay and AI gateway will be built the same way. Three things had to be settled once: how HTTP is served, when the schema is applied, and how an orchestrator learns whether an instance is working.

`docs/delivery.md` says migrations "run before each deploy," which describes the requirement but not the mechanism.

## Decision

- **axum, with the application built separately from the server.** `router(state)` returns the application and touches no network; `serve(config)` binds and runs it. Tests drive the routes through `oneshot` without binding a port, which is the same split as `build_subscriber` and `init` in `croncave-telemetry`.
- **Migrations run at startup, before the listener binds.** sqlx takes a lock while applying them, so several replicas starting at once is safe, and no binary can serve against a schema older than it expects. This satisfies "before each deploy" without a separate deploy step to forget. Migrations still have to work with the previous release's code during a rollout — expand first, contract later.
- **Liveness and readiness are different questions, on different paths.** `/health` says the process is running and touches nothing else. `/ready` says the database answers within two seconds, and returns 503 when it doesn't. A database blip should take an instance out of rotation, not have every instance restarted at once.
- **Loopback by default.** `CRONCAVE_BIND` defaults to `127.0.0.1:8080`. The browser reaches the control plane through the web app, and a service that binds a public interface should have to say so.
- **Graceful shutdown on SIGINT and SIGTERM**, then the pool closes, so work in flight finishes instead of being cut off mid-query.

## Alternatives considered

- **Migrations as a separate deploy step** (a job, or `sqlx migrate run` in CI): the conventional answer, and it keeps schema changes out of the serving path. Rejected for now because it is a step that can be skipped or run against the wrong environment, and because a developer's laptop then needs a second command to stay current. Worth revisiting when a migration is slow enough that startup time matters.
- **One `/health` that checks the database:** simpler, and wrong in the way that matters. An orchestrator reading it as liveness would restart every instance during a database blip, turning a recoverable problem into an outage.
- **Binding `0.0.0.0` by default:** what most examples do, and what makes a service reachable before anyone decided it should be.
- **No graceful shutdown:** fine for a stateless API, less fine once a request can be a long-running run or a session.

## Consequences

- Every later service — relay, AI gateway — copies this shape, and the `/health` and `/ready` pair is what deploy configuration will point at.
- Startup now depends on the database being reachable. That is deliberate: a control plane that cannot reach Postgres cannot do anything useful, and failing loudly at boot beats failing per request.
- A slow migration becomes slow startup. If that ever blocks a deploy, moving migrations to their own step is a contained change, since they already live in `crates/db` behind `migrate`.
