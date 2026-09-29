# Decision log

Every decision with lasting impact gets an entry here, so the reasoning survives. Add new decisions as numbered files, starting at `0019-short-title.md`, using [the template](template.md), and add a row to the table below.

Decisions made during planning (2026-09-28) are summarised in this table. The full reasoning is in the three docs and in `docs/history/planning-transcript.md`.

| # | Decision | Status | Where to read more |
| --- | --- | --- | --- |
| 0001 | Name: Croncave (working name). App at `app.croncave.com` | Accepted | product-definition.md, Decisions |
| 0002 | Core model: workspaces with tasks; standalone tasks get a hidden workspace; no desktop UI | Accepted | product-definition.md |
| 0003 | Workspaces never accept inbound connections; one outgoing agent connection carries everything | Accepted | architecture.md, The outgoing connection |
| 0004 | Compute: rent Fly Machines for R1 behind a `ComputeDriver` interface; later our own Firecracker system on a cloud provider | Accepted | architecture.md, Workspace compute |
| 0005 | Sleep by default: boot from disk (under 10 s target), fast memory-snapshot tier for small workspaces; 10-minute idle timeout, adjustable | Accepted | architecture.md, Workspace lifecycle |
| 0006 | Previews: in-app with new-tab fallback, on a separately registered domain, one subdomain per preview | Accepted | architecture.md, Private previews |
| 0007 | AI access: Claude Code unmodified; API key via our gateway by default (env-var mode built first as fallback); users' own Claude subscriptions via Claude Code's own sign-in | Accepted | architecture.md, AI access and secrets |
| 0008 | Models: Claude only in R1; later opt-in US-origin open models via OpenCode; no models from non-US labs | Accepted | architecture.md, Models and the router |
| 0009 | Scheduler: our own on Postgres; add workflow-engine features rather than migrating | Accepted | architecture.md, Scheduler, runs and events |
| 0010 | Stack: Rust (Tokio, axum, sqlx) backend and agent; TypeScript + SvelteKit web app | Accepted | architecture.md, Stack and build order |
| 0011 | Team-ready data model from day one; every workspace runs as a container inside its VM so packing can come in R2 | Accepted | architecture.md, Data model; Security |
| 0012 | Pricing: Free, Plus, Pro subscriptions plus usage. AI usage at exactly cost; other usage at cost plus a percentage set after R1 | Accepted | pricing.md |
| 0013 | US only at launch, for users, compute and data | Accepted | product-definition.md, Decisions |
| 0014 | One monorepo; local, CI, staging and production environments; milestone checkpoints | Accepted | delivery.md |
| 0015 | Develop locally with the founder until R1, plan-before-code for every step | Accepted | AGENTS.md, How we work |
| 0016 | Repo tooling: one Cargo workspace, a shared `crates/telemetry` crate, and `scripts/check.sh` as the only check entry point | Accepted | [0016-repo-tooling.md](0016-repo-tooling.md) |
| 0017 | Observability: `tracing` for structured logs, Sentry-protocol error reporting with an optional DSN, `rustls` throughout | Accepted | [0017-observability.md](0017-observability.md) |
| 0018 | Web app: SvelteKit 2 + Svelte 5 with adapter-node, pnpm pinned by corepack, TypeScript strict at `^6`, reading the root `.env` | Accepted | [0018-web-toolchain.md](0018-web-toolchain.md) |
