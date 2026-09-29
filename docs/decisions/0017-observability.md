# 0017: Structured logs with `tracing`, error reporting over the Sentry protocol

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

Step 0 is proven when "CI runs green; logging and error tracking are wired in," but no planning document picked the tooling. Croncave's shape makes the choice matter more than usual: a run happens across the control plane, the relay and an agent inside a workspace, and when it goes wrong the founder is reading three services' output side by side to find out why. Work also happens while nobody is watching — that is the product — so a failure at 03:00 has to reach someone without a person tailing a log.

Two constraints shaped the answer. Contributors and CI must not need an account or a secret to run the code. And the agent is a static binary shipped into every workspace image, so its dependencies must not drag in OpenSSL.

## Decision

- **`tracing` for structured logs**, configured once in `crates/telemetry`. Pretty lines when a person is watching a terminal, one JSON object per line otherwise, chosen automatically by whether stdout is a terminal and overridable with `CRONCAVE_LOG_FORMAT`. Every JSON event carries the service's name, version and environment.
- **Error reporting over the Sentry protocol**, via the `sentry` and `sentry-tracing` crates. Every `tracing::error!` becomes an event, quieter events become breadcrumbs on it, and panics are reported.
- **The DSN is optional.** `SENTRY_DSN` unset means no client is created and reporting is off. Local runs and CI therefore need no account and no secret.
- **`rustls`, not `native-tls`**, so nothing in the dependency tree needs OpenSSL.
- **No performance tracing and `send_default_pii: false`** for R1. We decide what a report contains, not the SDK.

## Alternatives considered

- **OpenTelemetry over OTLP:** the most portable, and where we will probably end up when there are enough services to trace a request across. It needs a collector to run somewhere before it's useful, which is machinery to operate before there is anything to observe. Adding it later is a layer in one crate, not a rewrite.
- **Structured logs only, error tracking later:** would leave the step 0 checkpoint half met, and the first unattended overnight failure is exactly what we want to hear about.
- **A log-search product (Axiom, Better Stack) as the only destination:** good for searching, but no grouping, no deduplication and no "this started at release X" — which is what you want when a run fails at 03:00.
- **`native-tls`:** the default, and would mean OpenSSL in the agent's static binary.

## Consequences

- Where Sentry runs is not decided here and does not need to be. The same SDK speaks to hosted Sentry, self-hosted Sentry and GlitchTip; the choice is a DSN in an environment's secret store.
- The `SENTRY_DSN` for staging and production goes in each environment's secret store, never in the repository.
- Because `SENTRY_DSN` is read at startup, an invalid DSN stops the service rather than silently disabling reporting. The same applies to `CRONCAVE_ENV` and `CRONCAVE_LOG_FORMAT`.
- `Config`'s `Debug` implementation redacts the DSN, so dumping a configuration into a log cannot leak it.
- Log volume is now a cost we control with `RUST_LOG` per environment. Workspace agents buffer and forward logs over the relay (see `docs/architecture.md`), and how those reach the same place belongs to the connection step, not step 0.

## Amendment, 2026-09-29: the JavaScript SDK's collection defaults

Wiring the same posture into the web app turned out not to be symmetrical. The Rust SDK has one switch, `send_default_pii`, which is off by default. The JavaScript SDK version 11 replaced that switch with a `dataCollection` object whose defaults are permissive: cookies, request and response headers, request and response bodies, URL query parameters, database query data, queue arguments and the values of local variables in stack frames are all collected unless you say otherwise.

For a product running people's private workspaces those are the wrong defaults, and they are the kind that arrive quietly in a dependency upgrade. So `web/src/lib/sentry.ts` turns all of them off explicitly, both hooks use it, and a test asserts that every field in it collects nothing — which fails if a field is ever switched on, or if a new permissive field is added and defaulted in. Source context lines are the one thing left on: that is our own code, and it is what makes a stack trace readable.

Two related notes from the same wiring:

- Disabling performance tracing means leaving `tracesSampleRate` **unset**. Setting it to `0` still turns tracing on and then samples none of it.
- The browser can only be given values whose names begin with `PUBLIC_`, so `PUBLIC_SENTRY_DSN` and `PUBLIC_CRONCAVE_ENV` mirror the private variables for code running in the page. The private ones stay authoritative everywhere on the server.
