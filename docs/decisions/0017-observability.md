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
- Log volume is now a cost we control with `RUST_LOG` per environment. Workspace agents buffer and forward logs over the relay (see `docs/architecture.md`), and how those reach the same place is step 1's problem, not step 0's.
