# Croncave: context for coding agents

This file is the shared brief for any AI coding agent working in this repository (Claude Code reads it through `CLAUDE.md`). Read it fully before doing anything, then read the docs it points to when a task touches them.

## What Croncave is

Croncave gives people a private computer in the cloud that keeps working after they close their laptop. It's managed entirely through a web app (no remote desktop). Users hand off overnight AI coding sessions, scheduled scripts, monitors and small apps. They come back to plain-language results, a change review, or a private preview of what was built.

- **App:** `app.croncave.com` (not live yet). **Previews:** a separately registered domain, one subdomain per preview (name to be picked).
- **Status:** planning is done; no product code exists yet. We are at build step 0 (see [Build order](#build-order-for-r1)).
- **First customers:** technical founders (three are lined up for the R1 alpha). Non-technical users are the long-term goal.
- **Region:** US only, for users, compute and data.

## Where the context lives

| Doc | What it covers |
| --- | --- |
| [docs/product-definition.md](docs/product-definition.md) | PR/FAQ, every feature as a user story with MoSCoW priority and release (R1, R2, R3, Later), decisions |
| [docs/architecture.md](docs/architecture.md) | System design: compute, lifecycle, the outgoing connection, previews, AI access, models, scheduler, data model, security, stack, build order |
| [docs/pricing.md](docs/pricing.md) | Tiers, usage billing, caps, free-tier guardrails |
| [docs/delivery.md](docs/delivery.md) | Repository layout, build artifacts, environments, testing, milestone checkpoints |
| [docs/decisions/](docs/decisions/) | Decision log. Add an entry for every meaningful decision |
| [docs/design/mockups.md](docs/design/mockups.md) | Links to the UI mockups |
| [docs/history/planning-transcript.md](docs/history/planning-transcript.md) | The full planning conversation, for the reasoning behind decisions |

The three main docs are **snapshots** of living docs on claude.ai (links at the top of each). If a snapshot and the living doc disagree, the living doc wins. Ask the user to refresh the snapshot rather than guessing.

## How we work (the user wants close visibility and input)

1. **Plan before code.** For every task, first propose a short plan: what you'll build, which files change, how it will be tested. Wait for the user's approval before writing code.
2. **Small steps.** Each change should be reviewable in one sitting and end with a test the user can run.
3. **Record decisions.** Any choice with lasting impact (a library, a protocol shape, a schema pattern) gets a short entry in `docs/decisions/`.
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

## Planned repository layout

Nothing below exists yet. Create each part only when its build step starts (see [docs/delivery.md](docs/delivery.md)).

```
crates/            Rust (one Cargo workspace)
  protocol/        agent <-> relay messages and shared types (also generates TypeScript types)
  compute/         ComputeDriver interface + drivers: fake, local (Docker), fly
  control-plane/   API, scheduler, orchestrator
  relay/
  ai-gateway/
  agent/           static binary that runs inside workspaces
  db/              Postgres migrations
web/               SvelteKit app, including the view component library
images/            workspace base images (agent + Claude Code + common tools)
templates/         starter templates
infra/             deploy config per environment
e2e/               end-to-end tests (Playwright)
docs/              product, architecture, pricing, delivery, decisions
```

## Build order for R1

Each step ends with a check that proves it works before moving on.

| Step | Proven when |
| --- | --- |
| 0. Repo and CI | CI runs green; logging and error tracking are wired in |
| 1. Agent + relay, locally | A browser terminal works on a local (Docker) workspace |
| 2. Fly driver | The same test passes on real Fly in staging; the driver test suite passes |
| 3. Sleep, wake, scheduler | A scheduled run wakes, runs and sleeps again; wake time is measured |
| 4. Claude Code sessions | A session keeps running after the browser closes; caps stop it |
| 5. GitHub and change review | A session opens a pull request; the review screen shows it |
| 6. Home, timeline, email | "Needs you," "running" and "done" reflect real events |
| 7. Previews | An app on localhost opens in the app from the preview domain |
| 8. Hardening | Security and abuse tests pass; then the alpha opens on the real domain |

## Testing and environments

- **Local:** one command starts Postgres, MinIO, the server, the web app and workspaces through the local (Docker) driver. A fake Claude Code tests sessions without spending tokens.
- **CI** (GitHub Actions, `.github/workflows/ci.yml`): unit tests, protocol tests, the shared driver test suite (fake + local), Playwright end-to-end tests, and security tests (a workspace can't be reached from outside; egress rules hold). CI jobs switch on automatically as each part of the repo appears.
- **Staging:** separate Fly organisation, its own domain and preview domain, Stripe test mode, a development GitHub App, a low-limit Anthropic key. Nightly end-to-end tests against real Fly machines.
- **Production:** `app.croncave.com`, invite-only for the alpha, feature flags for new features, agent updates rolled out to a few workspaces first.

## Conventions (to be refined in step 0)

- Rust: stable toolchain, `cargo fmt`, `cargo clippy -- -D warnings`, tests alongside code. Errors via `thiserror` in libraries and `anyhow` at binaries.
- TypeScript: strict mode, `pnpm`, Prettier + ESLint, `svelte-check`.
- Database migrations must work with both the old and new code during a rollout (expand, then contract).
- Commit messages: imperative summary line, body explaining why.
