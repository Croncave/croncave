# Delivery: repository, environments and testing

How Croncave is organised, built and tested on the way to R1. Agreed during planning on 2026-09-28. Update this file when the approach changes, and log the change in `docs/decisions/`.

## One repository

The platform lives in this single repository. The agent, relay, control plane and web app share one protocol, and most features touch several of them at once. In one repository, a protocol change and every side that uses it land in a single commit and are tested together.

Separate repositories only where something is published for others:

- **SDKs** (R2), such as `croncave` on PyPI and npm, so users can inspect the code they install.
- **Public templates**, possibly later, for community contributions.

## Planned layout

Create each part only when its build step starts.

```
crates/            Rust (one Cargo workspace)
  protocol/        agent <-> relay messages and shared types (also generates TypeScript types)
  compute/         ComputeDriver interface + drivers: fake, local (Docker), fly
  control-plane/   API, scheduler, orchestrator
  relay/
  ai-gateway/
  agent/           static binary that runs inside workspaces
  telemetry/       structured logging and error reporting, shared by every service
  db/              Postgres migrations
web/               SvelteKit app, including the view component library
images/            workspace base images (agent + Claude Code + common tools)
templates/         starter templates
infra/             deploy config per environment
e2e/               end-to-end tests (Playwright)
docs/              product, architecture, pricing, delivery, decisions
```

## What gets built and shipped

| Artifact | Built from | Notes |
| --- | --- | --- |
| Server image | control-plane, relay, ai-gateway | One binary with a role flag at first. Split into separate services when scale needs it |
| Agent binary | agent | Static and versioned. Updated when a workspace wakes |
| Workspace base image | images | A standard OCI image, so it runs on any provider |
| Web app | web | SvelteKit build |
| Database migrations | db | Run before each deploy. Old and new code must both work during a rollout |

## The compute abstraction

The `ComputeDriver` interface is the only place a provider appears: create, start, stop, suspend, snapshot, resize, destroy, status. It starts with three drivers:

1. **Fake:** in memory, for fast tests.
2. **Local:** Docker containers on a developer machine. They aren't VMs, but they behave the same from the agent's side.
3. **Fly:** the real provider for R1.

One shared test suite runs against every driver. A new driver (for example our own Firecracker hosts, or another provider if Fly's terms don't work out) only has to pass that suite. The control plane also runs as an ordinary container, not tied to Fly.

## Environments

1. **Local:** one command starts Postgres, MinIO (S3-compatible storage), the server, the web app, and workspaces through the local driver. A fake Claude Code tests sessions without spending tokens.
2. **CI (GitHub Actions), on every push and pull request:**
   - unit tests
   - protocol tests
   - the driver test suite, on fake and local
   - end-to-end tests with Playwright: create a workspace, open the terminal, run a task, see it in the timeline, open a preview
   - security tests: a workspace can't be reached from outside, and egress rules block what they should
3. **Staging:** real infrastructure, kept apart from the real domain:
   - a separate Fly organisation
   - its own domain (such as `staging.croncave.com`) and a separate preview domain
   - Stripe in test mode
   - a GitHub App for development
   - a low-limit Anthropic key
   - end-to-end tests nightly against real Fly machines, measuring wake times and costs

   The founder uses staging for their own work, as the first real user.
4. **Production:** `app.croncave.com`, invite-only at first. Everything is separate from staging. Alpha users are on an allowlist, new features ship behind feature flags, and agent updates go to a few workspaces before all of them.

Secrets for staging and production live in GitHub Actions secrets and the provider's secret store. They are never committed.

## Milestone checkpoints

Each step has a check that proves it works before moving on.

| Step | Proven when |
| --- | --- |
| 0. Repo and CI | CI runs green, with logging and error tracking wired in |
| 1. Agent + relay, locally | A browser terminal works on a local workspace |
| 2. Fly driver | The same test passes on real Fly in staging, and the driver suite passes |
| 3. Sleep, wake and the scheduler | A scheduled run wakes, runs and sleeps again, and wake time is measured |
| 4. Claude Code sessions | A session keeps running after the browser closes, and caps stop it |
| 5. GitHub and change review | A session opens a pull request, and the review screen shows it |
| 6. Home, timeline, email | "Needs you," "running" and "done" reflect real events |
| 7. Previews | An app on localhost opens in the app from the preview domain |
| 8. Hardening | Security and abuse tests pass. Then the alpha goes live on the real domain |

## Before launch

- Get written confirmation from Fly that running a platform for our customers is allowed, including a markup on usage (see `docs/pricing.md`).
- Confirm with Anthropic that the AI gateway attaching a user's own key is allowed, and that built-in features on our own API account, billed at cost, are fine (see `docs/architecture.md`).
