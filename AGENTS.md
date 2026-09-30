# Croncave: context for coding agents

This file is the shared brief for any AI coding agent working in this repository (Claude Code reads it through `CLAUDE.md`). Read it fully before doing anything, then read the docs it points to when a task touches them.

## What Croncave is

Croncave gives people a private computer in the cloud that keeps working after they close their laptop. It's managed entirely through a web app (no remote desktop). Users hand off overnight AI coding sessions, scheduled scripts, monitors and small apps. They come back to plain-language results, a change review, or a private preview of what was built.

- **App:** `app.croncave.com` (not live yet). **Previews:** a separately registered domain, one subdomain per preview (name to be picked).
- **Status:** step 3 (the connection) is built — a workspace's agent dials out, runs what it is asked and streams the output back, and workspaces sleep when nothing is happening — and awaits its first green CI run. The Fly driver still waits on that account and its terms. Step 4 (Claude Code sessions) is next (see [Build order](#build-order-for-r1)).
- **First customers:** technical founders (three are lined up for the R1 alpha). Non-technical users are the long-term goal.
- **Region:** US only, for users, compute and data.

## Where the context lives

Four documents, and this one. **This repository is the source of truth**: a
change to the product or the architecture is made here, in the same pull
request as the code it affects.

| Doc | What it covers |
| --- | --- |
| [docs/product.md](docs/product.md) | What Croncave is for. PR/FAQ, every feature as a user story with its priority and release, and the product decisions |
| [docs/architecture.md](docs/architecture.md) | How it is designed and in what order it is built. **Owns the repository layout and the build order** |
| [docs/pricing.md](docs/pricing.md) | Tiers, usage billing, caps, free-tier guardrails |
| [docs/decisions.md](docs/decisions.md) | Every choice with lasting impact, newest first, with what was rejected. **Add to it** |
| [docs/design/](docs/design/) | The design system and every designed screen. Read `design/system/README.md` before any UI work |

## How we work (the user wants close visibility and input)

1. **Plan before code.** For every task, first propose a short plan: what you'll build, which files change, how it will be tested. Wait for the user's approval before writing code.
2. **Small steps.** Each change should be reviewable in one sitting and end with a test the user can run.
3. **Record decisions.** Any choice with lasting impact (a library, a protocol shape, a schema pattern) gets an entry at the top of `docs/decisions.md`.
4. **Don't expand scope silently.** If something outside the agreed step seems necessary, say so and ask.
5. **Explain in plain words.** The user is a software engineer. Be precise, skip filler.

## Product principles (every feature must respect these)

1. **The work is the product, not the machine.** Users see workspaces and results. VMs and providers appear only in a details layer.
2. **Nothing connects in.** Workspaces never accept inbound connections: no public address, no open ports. Everything travels over one connection the workspace's agent opens outward.
3. **AI and manual are one system.** Every task is defined by five answers: when it runs, what it does, what it remembers, where results go, and its limits. AI fills in the same form a person would.
4. **Standard views, not generated UI.** Dashboards are composed from a fixed component library. AI chooses and connects components; manual code feeds them through an SDK.
5. **Layered disclosure.** Default, details and expert layers on every screen.
6. **AI proposes, people approve.** Changes that matter arrive as proposals: change reviews, plan cards, notes with statuses.
7. **Sleep by default.** A workspace is awake only while work runs or someone is looking.
8. **Team-ready data model from day one.** Everything belongs to a team (a personal team of one in R1), every action is attributed, and work happens in sessions.

## Architecture in brief

- **Control plane** (Rust: Tokio, axum, sqlx): API, scheduler, orchestrator. Postgres for records, the run queue and events. S3-compatible object storage for logs, transcripts and files.
- **Relay:** where workspace agents connect. It forwards terminal, logs, files and preview traffic, and authorises every stream.
- **Agent:** a static Rust binary inside each workspace. It dials out to the relay over TLS on 443 with a stream multiplexer, supervises runs and sessions, and buffers logs.
- **AI gateway:** Claude Code points `ANTHROPIC_BASE_URL` here. It attaches the user's key, meters usage and enforces caps.
- **Compute:** behind a `ComputeDriver` interface (create, start, stop, suspend, snapshot, resize, destroy, status). R1 runs on **Fly Machines**. Later we move to our own Firecracker system on a cloud provider. Fly's terms need written confirmation before launch, so **nothing outside the compute driver may depend on Fly.**
- **Web app:** TypeScript + SvelteKit, including the view component library.
- **Scheduler:** our own, on Postgres (a Rust job library such as apalis for cron timing; queue claimed with row locking). Add workflow-engine features to it rather than migrating to Temporal or Restate.

## Hard rules

- **Workspaces never listen.** Don't add any code path that exposes a port or public address on a workspace.
- **Provider-neutral outside the driver.** No Fly-specific code or assumptions outside `crates/compute`'s Fly driver. Workspace images are standard OCI images.
- **Claude Code runs unmodified.** Never patch it or remove its sign-in methods. Never collect, store or relay users' Claude credentials, and never offer our own "Sign in with Claude" button. Users sign in to Claude Code through Anthropic's own flow inside the workspace.
- **AI keys:** the default path keeps the user's API key in our gateway, never in the workspace. The environment-variable mode (key in the workspace) is built first and stays as a fallback.
- **Models:** R1 uses Claude only (Claude Code for sessions; Claude via our own API account for built-in features, billed to users at exactly our cost). Later, only US-origin open models, opt-in. Never add models from non-US labs.
- **Previews** are served from a separate registered domain, one subdomain per preview, never under `croncave.com`.
- **Secrets** never go in the repo, logs or test fixtures. Use environment variables and the secrets manager of each environment.
- **US only:** no infrastructure outside US regions.

## Repository layout

**[docs/architecture.md](docs/architecture.md) has the full layout.** Rust crates live in `crates/` as one Cargo workspace (`protocol`, `compute`, `control-plane`, `relay`, `ai-gateway`, `agent`, `telemetry`, `db`); the SvelteKit app and its view component library live in `web/`; then `images/`, `templates/`, `infra/`, `e2e/` and `docs/`.

Everything but `crates/ai-gateway`, `templates/` and `infra/` exists. Create each other part when its build step starts.

## Build order for R1

**[docs/architecture.md](docs/architecture.md) has the table, with the check that proves each step.** It follows the build order in [docs/architecture.md](docs/architecture.md), with a step 0 added for the repository setup that list doesn't cover.

**0** Repo and CI ✅ · **1** Accounts and workspaces ✅ · **2** Workspace compute ✅ · **3** Connection ✅ · **4** Claude Code sessions · **5** GitHub and change review · **6** Scheduled runs · **7** Home and timeline · **8** Previews, templates and dashboards · **9** Hardening

**Step 2 is next.** Each step is delivered in slices small enough to review in one sitting — step 0 took five, step 1 took eight.

## Testing and environments

**[docs/architecture.md](docs/architecture.md) has the detail.** In brief:

- **Local:** one command starts Postgres, MinIO, the server, the web app and workspaces through the local (Docker) driver. A fake Claude Code tests sessions without spending tokens.
- **CI** (GitHub Actions, `.github/workflows/ci.yml`): unit tests, protocol tests, the shared driver test suite (fake + local), Playwright end-to-end tests, and security tests (a workspace can't be reached from outside; egress rules hold). CI jobs switch on automatically as each part of the repo appears.
- **Staging:** separate Fly organisation, its own domain and preview domain, Stripe test mode, a development GitHub App, a low-limit Anthropic key. Nightly end-to-end tests against real Fly machines.
- **Production:** `app.croncave.com`, invite-only for the alpha, feature flags for new features, agent updates rolled out to a few workspaces first.

## Conventions

### Running the checks

`./scripts/check.sh` runs everything CI runs: `rust` or `web` narrows it to one side. **CI calls the same script**, so a check added only to the workflow file is a bug — add it to the script instead.

```
corepack enable pnpm     # once per machine
cp .env.example .env     # once per clone
./scripts/check.sh
```

### Rust

- **Edition 2024, stable toolchain.** `rust-toolchain.toml` pins the channel; `cargo fmt` defaults are the house style.
- **Crates are named `croncave-*`** and live in `crates/<short-name>/` (`crates/telemetry` holds `croncave-telemetry`). Add the directory to `members` in the root `Cargo.toml`.
- **Versions are agreed once.** Dependencies go in `[workspace.dependencies]` and crates take them with `foo.workspace = true`. Same for `version`, `edition`, `rust-version`, `license` and `lints`.
- **Errors:** `thiserror` in libraries, `anyhow` at binaries. A library's error type should say what a caller can do about it, not just what went wrong.
- **`unsafe` is forbidden** workspace-wide. `unwrap` and `expect` warn, and CI turns warnings into errors. In a test module, opt out at the top rather than sprinkling attributes:
  ```rust
  #[cfg(test)]
  mod tests {
      #![allow(clippy::expect_used, clippy::unwrap_used)]
  ```
- **Tests live beside the code** they test, in a `mod tests`. Prefer a test that exercises real behaviour over one that restates a constant.
- **Keep the globals out of the unit under test.** Split the pure part from the part that installs process-wide state, so tests can call the pure part: `build_subscriber` and `init` in `crates/telemetry/src/lib.rs` are the pattern.

### Logging and errors

- **Every service uses `croncave-telemetry`.** Call `init` once at startup and hold the `Guard` until the process exits. Don't build a second subscriber.
- **Never log a secret.** No tokens, API keys, DSNs, GitHub credentials or workspace credentials in a field, a message or a `Debug` implementation. `Config` in `croncave-telemetry` redacts its DSN; do the same for any type that carries one.
- **`tracing::error!` means "a person should see this."** It becomes a Sentry event. Use `warn!` for something recovered from, and put the detail in fields rather than in the message text, so it stays searchable.
- **Error reporting is off without a DSN**, by design. Don't add a fallback that reports somewhere else.

### Configuration

- **Every variable the code reads has an entry in `.env.example`**, with a comment. That file is the list a developer checks when something isn't set.
- **Binaries load `.env` first**, with `dotenvy::dotenv().ok()`, before reading any configuration. Libraries never do.
- **An unrecognised value is an error, not a guess.** A typo in a deployment's `CRONCAVE_ENV` must stop the service, never quietly make production look local. Blank or unset may fall back to a default; wrong may not.
- **Secrets are never committed**, in code, fixtures, logs or test data. They live in each environment's secret store.

### TypeScript and the web app

- **Strict mode, no `any`.** `pnpm check` runs `svelte-check`, and it must be clean.
- **pnpm, pinned by the `packageManager` field** and enabled with `corepack enable pnpm`. The lockfile is committed and CI installs with `--frozen-lockfile`.
- **Tests are colocated** as `*.test.ts` next to the module, run by Vitest.
- **Prettier and ESLint are not negotiable**; `pnpm format` fixes what it can.
- **The design system is imported, never copied.** `web/src/app.css` reads `docs/design/system/tokens.css` and `components.css` directly, because that folder is a snapshot of the canvas and gets re-exported. Use tokens (`var(--space-4)`), never raw values.
- **Shared meaning stays in step with Rust.** `web/src/lib/environment.ts` deliberately mirrors `Environment` in `crates/telemetry`. When the protocol crate arrives, generate TypeScript from Rust rather than writing a third copy by hand.

### Dependencies

- **pnpm refuses packages published in the last day or so.** That is a supply-chain guard, and pnpm will offer to write a `minimumReleaseAgeExclude` list to get past it. **Never accept that offer** — pin to the previous release instead, as `typescript-eslint` is pinned to `~8.70.1`.
- **Prefer `rustls` over `native-tls`**, so nothing drags OpenSSL into the agent's static binary.
- **Check a new dependency's defaults, not just its API.** Sentry's JavaScript SDK defaults to collecting cookies, headers and request bodies; `web/src/lib/sentry.ts` turns that off and a test holds it off.

### Talking to Postgres

**Queries are runtime-checked (`sqlx::query`), not the `query!` macros.** The macros check SQL against a live schema at compile time, which is genuinely valuable, but it means `cargo build` needs either a running database or an `.sqlx` cache that someone has to remember to regenerate. Step 0 deliberately made the checks runnable without services, and that is worth more. The tests cover the queries against a real Postgres instead.

### Tests

- **Unit and integration tests live with their crate**; the database ones run against a real Postgres through `#[sqlx::test]`, which gives each test its own database.
- **End-to-end tests live in `e2e/`**, not in `web/`. They drive a real browser against a real control plane against a real Postgres, so they belong to the whole system rather than to the web app.
- **Never add a test-only endpoint to production code.** The end-to-end tests read a sign-in link from the control plane's log, the way a developer does, because only the token's hash is stored and there is deliberately no way to read one back. A convenience endpoint would be a bypass living in the shipped binary.
- **Give each end-to-end test its own account** (`freshEmail()`), so runs never collide with each other or with whatever is already in the database.
- **A test that cannot fail is not a test.** When a test guards something that matters, break the thing once and watch that test — and ideally only that test — go red.

### Database migrations

Migrations will live in `crates/db` (created in the accounts and workspaces step) and run before each deploy. **Old and new code must both work while a rollout is in progress:** expand first (add the column, backfill, start writing to it), then contract in a later release (stop reading the old column, drop it). Never rename or drop in the same release that stops using something.

### Git

- **Branch per build step or fix**, named for the work: `step-0-repo-and-ci`.
- **Commit messages:** an imperative summary line, then a body explaining *why*. The diff already shows what changed; the body is for the reasoning that isn't in it.
- **Small, reviewable commits.** A commit should be readable in one sitting and leave the checks green.
- **The author commits, not an agent.** An agent finishes the work, runs the checks and reports; the person reviews the working tree and decides when it is committed and pushed.
- **Pull requests** use `.github/pull_request_template.md`. Its checklist is not decoration: workspaces never listen, nothing outside the compute driver depends on a provider, no secrets, docs updated.

### Decisions

**One copy of each thing.** `architecture.md` owns the layout and the build order; this file summarises and links. Don't paste either back — keeping two copies is what let them drift before.

Anything with lasting impact — a library, a protocol shape, a schema pattern, a default that affects privacy or cost — gets an entry at the top of `docs/decisions.md`. Record the alternatives you rejected and why; that is the part that is expensive to reconstruct later. If a decision is later contradicted by experience, amend the entry with a dated section rather than editing history.
