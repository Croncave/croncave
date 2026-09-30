# Croncave: the architecture, and the plan for building it

How the system is designed and in what order it is built. **This file is the
source of truth**, consolidated here on 2026-09-30. The separate
delivery document was folded into the Delivery section below, so the design
and the plan for realising it stay in step.

## Summary

How Croncave R1 is built. Every \[target\] is a number still to be confirmed by
measurement. It follows [the product definition](product.md), and nothing here
should contradict it; where building has taught us something the design did not
know, this document is updated and the reason recorded in
[the decisions](decisions.md).

**Goals: what R1 must do**

1. **Workspaces that sleep and wake.** A workspace keeps its disk while asleep and wakes quickly enough for its job \[target: under 10 seconds by default, with a faster tier for latency-sensitive cases\].
2. **Nothing connects in.** Workspaces have no public address and no open ports. Every interaction travels over a connection the workspace opens outward.
3. **Long unattended work.** An agent session can run for hours after the user leaves, and survive the user's browser closing.
4. **Scheduled runs** that wake a workspace, run, record the result and let it sleep.
5. **Private previews** of anything a workspace serves on localhost, usable like a local browser tab.
6. **Claude Code, set up in one step,** with either the user's API key or their own Claude subscription.
7. **Isolation** strong enough to run untrusted and AI-written code: no user can reach another user's workspace or data.
8. **A team-ready data model,** with owners, attribution and sessions, even though R1 has no team features.
9. **US-only** data and compute.

**Non-goals for R1**

- Public hosting, or accepting inbound requests
- GPUs
- Regions outside the US
- Collaboration features (only the data model is prepared)
- Webhook and polling triggers (R3), though the scheduler should not rule them out
- Multi-cloud or self-hosted deployments

## System overview

The design rests on one rule: a workspace never listens, it only dials out. Everything a user does in the app reaches the workspace through the relay, over a single connection the workspace's agent opened itself.

[Diagram in the source doc: system overview · R1 components]

- **Edge** terminates HTTPS for the app and for previews, checks sign-in, and routes requests.
- **Control plane** is the Rust service holding the API, the scheduler and the orchestrator, backed by Postgres, object storage and an event stream.
- **Relay** is where workspace agents connect. It forwards terminal, logs, file and preview traffic between browsers and workspaces.
- **AI gateway** receives Claude Code's model calls, attaches the user's key, meters usage and enforces spending caps.
- **Compute hosts** run the workspaces. The orchestrator talks to the host, never to the workspace directly.
- **Filtered egress** lets user code reach GitHub and the wider internet, subject to abuse controls.

## Workspace compute

**Decision: rent from one provider for R1, Fly Machines, behind our own compute interface. Later, move to our own Firecracker system running on a cloud provider.** The orchestrator talks to compute only through a small driver interface (create, start, stop, snapshot, resize, destroy), so the move later doesn't touch the rest of the system. We use one provider, not several, to keep operations simple.

What we need from compute: a disk that survives while the workspace sleeps, sessions that can run for many hours, VM-level isolation for untrusted and AI-written code, US regions, and per-second billing.

| Option | Isolation | Keeps state while asleep | Long runs | Fit for Croncave |
| --- | --- | --- | --- | --- |
| **Fly Machines** ([docs](https://docs.fly.io/reference/suspend-resume)) | Firecracker microVMs | Stopped machines keep their disk. Suspend saves memory too and resumes in a few hundred milliseconds, but only for machines with 2 GB of memory or less | No session cap known | Best match for "a computer that sleeps." General-purpose, not built for agents |
| **E2B** ([docs](https://docs.e2b.dev/sandbox/persistence)) | Firecracker microVMs | Pause saves disk and memory, about 4 s per GB to pause and about 1 s to resume. Retention is unclear: the docs say paused sandboxes are kept until deleted, one report says 30 days ([bex.co](https://bex.co/blog/2026/09/11/e2b-sandbox-time-limits)) | 24 h continuous on Pro, $150/month ([Morph](https://www.morphllm.com/e2b-pricing)) | Built for agents, and open source with a self-host path. The session cap and retention need confirming |
| **Daytona** | Docker containers, shared host kernel | Stateful, with unlimited persistence | No stated limit | Weaker isolation for untrusted code. Core went closed-source in June 2026 ([bex.co](https://bex.co/blog/2026/09/14/ai-agent-sandbox-market-e2b-modal-daytona)) |
| **Modal** | gVisor containers | Ephemeral, with separate volumes | No stated limit | Good for bursts of compute, but not shaped like a persistent workspace. No self-hosting |
| **Our own Firecracker** | Firecracker microVMs | Whatever we build | Whatever we build | Full control and the lowest cost at scale. Now runs on regular AWS C8i, M8i and R8i instances through nested virtualization, launched February 2026 ([AWS](https://aws.amazon.com/about-aws/whats-new/2026/02/amazon-ec2-nested-virtualization-on-virtual)). One estimate puts it at a senior infrastructure engineer for two quarters ([bex.co](https://bex.co/blog/2026/09/14/ai-agent-sandbox-market-e2b-modal-daytona)) |

**Why rent first.** Building compute would delay R1 by months, and the three alpha founders need a working product, not an efficient one. Renting also gives real usage data (how long workspaces stay awake, how big their disks get) before we design our own hosts.

**Why Fly over E2B**

| Need | Fly Machines | E2B |
| --- | --- | --- |
| A disk that survives long sleeps | Volumes persist until deleted | Paused sandboxes may be deleted after 30 days, and the docs and reports disagree |
| Sessions of many hours | No known cap | 24 h continuous on Pro before a pause is needed |
| Sleeping large workspaces | Stop and boot from disk at any size | Pausing takes about 4 s per GB of memory |
| Fast wake for small workspaces | Suspend resumes in a few hundred milliseconds, at 2 GB of memory or less | Resume in about 1 s |
| Hardware | Fly's own servers ([architecture](https://docs.fly.io/reference/architecture)) | Nested inside Google Cloud VMs, with disk and network overhead ([bex.co](https://bex.co/blog/2026/09/24/e2b-l2-guests-firecracker-nested-virtualization-vs-hetzner-microvms)) |
| Model | General-purpose VMs, which matches "a computer that sleeps" | Sandboxes built around sessions |

**Fly limits we design around** ([volumes](https://fly.io/docs/volumes/overview/)):

- A volume lives on one physical server and can't move. If that server fails, we restore from a snapshot. Fly keeps daily snapshots for 5 days by default (configurable to 60), so we take our own backups too, and code lives in git anyway.
- Volumes max out at 500 GB, and they can grow but not shrink.
- Suspend (fast wake) only works up to 2 GB of memory, so it's the fast tier, not the default.

**Resizing.** A standalone task already lives in a hidden workspace, so growing it into a bigger workspace is a resize, not a migration. More CPU or memory is a machine update plus a restart of a few seconds. More disk is a volume extension. Separation doesn't change: each workspace is its own VM.

Still to check: Fly's prices for our expected workspace sizes, and its acceptable-use terms for hosting other people's code.

**Migration plan: our own system**

The long-term goal is our own Firecracker system running on a cloud provider, for better cost and performance. It becomes practical because AWS now supports nested virtualization on C8i, M8i and R8i instances, so it no longer needs bare-metal servers.

1. **From day one:** keep the driver interface narrow, keep workspace images provider-neutral, and record real usage (awake hours, disk sizes, wake times) to size our own hosts.
2. **Build alongside:** a host daemon that runs Firecracker VMs, a placement service, disk snapshots to object storage, and memory snapshots for the fast tier.
3. **Move gradually:** new workspaces go to our hosts first. Existing ones move when they next sleep, by copying the disk and switching the driver. Users see nothing.
4. **Trigger:** start once provider costs are a large share of revenue, or once a provider limit blocks a feature we need.

## Workspace lifecycle

A workspace has four states. Billing follows them directly: awake time plus disk, and disk only while asleep.

[Diagram in the source doc: workspace lifecycle · 4 states]

- **Awake** means at least one of these is true: a run or agent session is active, a terminal or preview is open, or the user has the workspace open.
- **Going to sleep.** When none of those has been true for the idle timeout, the orchestrator stops the VM and keeps its disk. The timeout defaults to 10 minutes and can be changed per workspace in the details layer, with a note on cost.
- **Two wake speeds.** By default a workspace boots from its disk \[target: under 10 s\], which works at any size. A fast tier suspends memory too and wakes in under a second. It's for small workspaces (2 GB of memory or less on Fly) where speed matters, like previews or quick-reacting tasks.
- **Waking.** A scheduled run, a user opening the workspace, or "Run now" asks the orchestrator to start it. The agent reconnects to the relay, and the UI shows "waking" until it does.
- **Keep awake.** Bots and long processes can pin a workspace awake. The UI shows the monthly cost before the user confirms.
- **Agent sessions never sleep mid-task.** A running session counts as work, however long it takes.
- **Deleted.** Deletion hides the workspace at once and purges its data after 30 days, matching the retention default.

## The outgoing connection

Each workspace runs a small agent, a static Rust binary started at boot. It opens one long-lived, encrypted connection out to the relay on port 443 and carries everything over it as separate streams. Nothing inside the workspace listens for connections from outside.

**Identity.** When the orchestrator starts a workspace, it hands the VM a one-time bootstrap token. The agent exchanges it for a short-lived credential tied to that one workspace, and that credential is renewed while the connection stays up.

**Transport.** A WebSocket on port 443 with yamux multiplexing inside it. It crosses proxies and corporate networks that only expect HTTP, terminates beside the browser-facing routes, and gives the independent streams above over one outgoing connection. Authorisation happens before the multiplexer starts, so a connection that cannot prove who it is never gets streams at all. QUIC is a later option if reconnect speed matters. See decision 0024.

**What travels over the connection**

| Stream | Direction | Used for |
| --- | --- | --- |
| Control | Relay → agent | Start a run or session, stop, update settings, health checks |
| Events and logs | Agent → relay | Run output, exit codes, progress, SDK data for dashboards |
| Terminal | Both | The browser terminal and live agent session view |
| Files | Both | Uploading inputs and downloading results |
| Previews | Both | Web traffic between the browser and a localhost port |
| Heartbeat | Both | Detecting a dropped connection quickly |

**Staying reliable**

- **Work never depends on the connection.** Runs and agent sessions are processes inside the workspace, supervised by the agent. If the connection drops, work continues, and the agent reconnects with backoff.
- **Logs are buffered.** They're written to local disk and sent until the relay acknowledges them, so a reconnect loses nothing.
- **Relays are stateless.** A routing table maps each workspace to the relay node holding its connection, so any browser request can reach any workspace.
- **Every stream is authorised.** The relay checks each request against the control plane's permissions before opening a stream. The agent accepts commands only from the relay.
- **Agent updates** are versioned and applied when a workspace wakes.

**Build or adopt.** Tunnels like Cloudflare Tunnel, frp or Tailscale solve part of this, but our needs are specific: per-stream authorisation, terminal and log semantics, and wake-on-demand. A purpose-built agent and relay is a modest amount of Rust and keeps the security model in one place.

## Private previews

A preview lets the owner use whatever a workspace serves on localhost, as if it were running on their own computer. Previews reuse the outgoing connection, so they need no open ports.

**How a request travels**

1. The agent notices a new port listening on localhost (say 3000) and reports it. The UI shows "Your app is running on port 3000. Open preview."
2. The user clicks Open. The app mints a short-lived signed token for that workspace and port, and opens the preview's own address with it.
3. The preview address exchanges the token for a cookie scoped to that one preview.
4. Each request goes from the browser to the edge, then the relay, then down a preview stream to the agent, which forwards it to localhost:3000.

**Design rules**

- **A separate domain for previews** (decided: a separately registered domain with one subdomain per preview, such as \<id>.croncave-preview.com or \<id>.croncave.dev; name to pick when registering). User code runs there, so it must never share cookies or storage with app.croncave.com. Each preview gets a random subdomain, giving it its own browser origin.
- **It must behave like localhost.** The agent sends requests with `Host: localhost:<port>`, so dev servers that check the host (like Vite) accept them. The original address travels in forwarding headers. Websockets and live reload pass straight through.
- **Owner only in R1.** There's no public or anonymous access. Sharing with signed-in people arrives with collaboration.
- **An open preview keeps the workspace awake,** and closing it lets the idle timer start.

**Decision: in-app previews in R1, with a new tab as the fallback.** Previews show inside the app in an embedded frame. Browsers restrict cookies in embedded frames, so sign-in inside the frame uses partitioned cookies. We test that early, because it's the riskiest part of previews. If a browser blocks it, the preview opens in a new tab instead.

## AI access and secrets

Claude Code runs unmodified in every workspace, as Anthropic's terms require ([Claude Code legal and compliance](https://code.claude.com/docs/en/legal-and-compliance)). Users connect it in one of two ways, and the setup screen offers both.

**Path A: the user's API key, through our AI gateway (the default).** We build the simpler environment-variable mode first (below), then add the gateway and make it the default.

- The key is stored encrypted in the control plane and never enters the workspace.
- Each workspace gets `ANTHROPIC_BASE_URL` pointing at our AI gateway, plus a gateway credential scoped to that workspace. This is Claude Code's documented gateway setup ([LLM gateways](https://code.claude.com/docs/en/llm-gateway)).
- The gateway checks the credential, attaches the user's real key, forwards the call to Anthropic, reads token usage from the response, and stops the session when the spending cap is reached.
- Upkeep: Claude Code adds features with each release, and a gateway that doesn't forward them breaks those features. The gateway must follow Anthropic's compatibility guide and be tested against each new Claude Code version.
- Confirm with Anthropic during development that a gateway attaching the user's own key, with usage billed to that user, counts as provisioning their key rather than intermediating. If not, or if a user prefers it, the fallback is placing the key in the workspace as an environment variable, which the terms explicitly allow.

**Why the gateway is the default.** Prompt-injected code or a bad dependency can't steal a key that isn't in the workspace. The gateway also gives us hard spending caps, cost per session and instant revocation.

**Path A fallback: environment variable (built first).** The key is placed in the workspace as an environment variable. It's simpler, but the key sits on the workspace disk, and caps can only use awake time. Users can choose this mode, and it's the fallback if the gateway is unavailable or not allowed.

**Path B: the user's Claude subscription**

- A guided "Connect Claude" step starts Claude Code's own sign-in in the workspace and shows the link it prints. The user completes sign-in on Anthropic's page.
- Claude Code stores the login inside the workspace. We never read, copy or relay it.
- Subscription traffic goes straight from the workspace to Anthropic through filtered egress, not through our gateway, so we never carry the user's session tokens.
- We can't meter tokens on this path, and that's fine: caps use awake time only. The setup screen and the usage page say this explicitly, so users know Anthropic's own subscription limits apply to tokens.
- Tradeoff: code running in the workspace could read those credentials. The blast radius is the user's own subscription, and the UI says so before connecting.

**GitHub**

- A GitHub App with access to only the repositories the user picks.
- For each session, the control plane mints an installation token that expires within an hour and hands it to git through a credential helper run by the agent. The helper fetches a fresh token whenever git needs one, so a 10-hour session simply uses several tokens without anyone noticing. Long-lived GitHub tokens never sit on the workspace disk.

GitHub is optional. Many sessions, like analysing a dataset, never touch a repository.

**Secrets**

- Encrypted at rest with envelope encryption under a cloud key-management service.
- Delivered to a run as environment variables when it starts.
- Scrubbed from logs, and shown only in the expert layer.

## Models and the router

Claude is the default everywhere: in coding sessions and in built-in AI features. From R2, users can opt in to US-origin open models as a cheaper alternative. We don't offer models from non-US labs. **R1 uses Claude only.**

| Use | Default | Opt-in, from R2 | How it's paid |
| --- | --- | --- | --- |
| Built-in AI features: plan cards, dashboards, summaries, templates | Claude, through Croncave's own Anthropic API account. A fast, small model for quick tasks and a stronger one for plans, chosen by evaluation | US-origin open models, such as Nvidia Nemotron 3 Ultra ([OpenRouter](https://openrouter.ai/blog/insights/the-open-weight-models-that-matter-june-2026/)) | Billed to the user at exactly our cost |
| Coding sessions | Claude Code, with the user's API key or subscription | US-origin open models, through OpenCode | Anthropic bills the user directly. Open models are billed at cost |

Built-in features call Claude as a normal API product under Anthropic's Commercial Terms. The no-resale rule covers Claude Code usage on users' behalf. We confirm this with Anthropic alongside the gateway question.

**Router rules**

- **US-origin models only.** Models from non-US labs aren't offered.
- **One gateway, many models.** Each model has a route with its provider, price and origin. Open models go only to US-based providers with zero data retention, confirmed per model before enabling.
- **Kill switch.** Any model or provider can be disabled in minutes. Affected features fall back to the default.
- **Evaluations before changes.** Each built-in feature has a small set of real requests. A model, including a newer Claude model, only replaces another after passing it.
- **Billing and AI-off.** Usage is metered per model and billed at exactly provider cost. The AI-off switch blocks every provider.

**The second agent.** Claude Code only supports Claude models ([LLM gateways](https://code.claude.com/docs/en/llm-gateway)), so opt-in open models run in OpenCode. It's MIT-licensed, works with any provider, runs unattended, and points at the same gateway.

**Timing.** R1 is Claude only: built-in features call Claude through our account, and coding sessions run Claude Code. The router still ships in R1 with one provider, so adding open models later needs no redesign. Opt-in US-origin open models and OpenCode arrive with "choice of agents" in R2.

## Scheduler, runs and events

All work, whether scheduled, manual or an agent session, becomes a **run** recorded in Postgres. Runs follow one path, so the timeline, logs, notifications and billing work the same way for everything.

**How a scheduled run happens**

1. The scheduler finds a task that is due and queues a run. It's our own scheduler: a Rust job library (such as apalis) for cron timing, over a queue in Postgres claimed with row locking, so no extra service is needed. When we want features from workflow engines like Temporal or Restate, we add them to our scheduler rather than migrating to one.
2. The orchestrator makes sure the workspace is awake, waking it if needed.
3. The control plane sends a "start run" command over the workspace's connection, with the task's command, secrets and limits.
4. The agent runs it, streams logs and progress back, and reports the exit code.
5. The run record is closed. Notification rules are checked, and the workspace's idle timer starts.

**Guarantees**

- **Each scheduled time runs exactly once from the user's view.** A run is keyed by task and scheduled time, so a retry or duplicate can't create a second run.
- **Missed runs.** If the platform was down when a run was due, it runs once on recovery, not once per missed slot.
- **Overlap.** If a run is still going when the next is due, the task's "don't overlap" setting decides whether to skip or queue.
- **Limits.** The agent enforces maximum runtime and the control plane enforces spending caps. Both end the run cleanly and record why.

**Agent sessions** use the same records with a different shape: they have a starting prompt, a branch, a pull request, and "needs you" pauses when Claude Code asks a question. The branch and pull request are optional: a session's output can just as well be files, a report or dashboard data.

**Runs and sessions are linked.** Every run records what started it: a schedule, a person, a session or another run. Sessions list the runs they started. That chain costs nothing now and lets a later workflow feature ("run B with A's output") be mostly UI.

**Events and the live UI.** Every change emits an event: run started, log line, question asked, pull request opened. Events are stored for the timeline and pushed to open browsers over a streaming connection. Timeline sentences like "Ran at 8:00, found 3 new listings, emailed you" are built from structured events, not written by AI, so they're always accurate.

**Triggers later.** Webhook and polling triggers (R3) only add new ways to queue a run. Everything after step 1 stays the same.

**Notifications in R1:** in-app, plus email through a transactional email provider.

## Data model and storage

The schema is team-ready from day one: every user gets a personal team of one, and everything belongs to a team, not a user. Every action records who did it, whether a person, AI or the system.

| Entity | Holds | Notes |
| --- | --- | --- |
| User | Sign-in identity, profile | Belongs to one or more teams |
| Team | Owner of everything below, billing, limits | A personal team is created at sign-up |
| Membership | User, team, role | One row per user in R1, many later |
| Workspace | Team, name, state, resource preset, compute reference, AI-off flag | State follows the lifecycle above |
| Task | Workspace, the five answers: when, what, memory, results, limits | Written the same way by AI and by hand |
| Run | Task, started by (schedule, person, session or another run), status, start and end, exit code, awake seconds, AI cost | One per execution, including agent sessions |
| Session | Workspace, started by, prompt, optional branch and pull request, outputs, status | Lists the runs it started. Collaboration adds "in progress by" here |
| Event | Workspace, actor type and id, type, data, time | Feeds the timeline and live UI |
| Connection | Team, kind (GitHub, Anthropic), encrypted credential | Personal or shared keys come later |
| Secret | Workspace, name, encrypted value | Never logged |
| View | Workspace, component layout | The dashboard spec for the component library |
| Usage | Workspace, day, awake time, disk, tokens, cost | Drives caps and billing |

**Where data lives (all US regions)**

- **Postgres** (managed): everything in the table above.
- **Object storage:** run logs in chunks, session transcripts, uploaded and downloaded files.
- **Workspace disks** stay with the compute provider and are included in workspace backups.

**Retention.** A nightly job applies each team's retention settings, starting from the defaults in the product definition: logs 30 days, transcripts 90 days, timeline 1 year, deleted workspaces purged after 30 days.

## Security and abuse controls

We run untrusted, AI-written code for strangers, so the defaults assume any workspace may be hostile.

**Isolation**

- One microVM per workspace in R1, with the workspace running as its own container inside it, supervised by the agent. Packing several of one user's workspaces into one VM is planned for R2, to cut costs and inform pricing. It then becomes a change of placement, not of protocol. Workspaces from different users never share a VM.
- No inbound: no public address, no exposed services, and a firewall inside the VM that drops inbound traffic.
- A workspace's credentials (relay, gateway, GitHub) work only for that workspace and expire quickly, which limits what a prompt-injected agent could take.

**Egress filtering**

- Block cloud metadata endpoints and all internal platform addresses, so user code can't reach our own infrastructure.
- Block outbound email ports, to stop spam.
- Block known crypto-mining pools, and rate-limit bandwidth per workspace.

**Abuse detection and limits**

- Flag sustained full-CPU use with mining-like patterns and unusual network volume.
- Per-account limits on concurrent awake workspaces and on spending.
- Alpha access by invitation. Public sign-up needs a verified card and a US address.

**Data protection**

- Encryption in transit and at rest, with secrets and keys under a key-management service.
- An audit log of sign-ins, connection changes, secret access and deletions.

**Supply chain.** Claude Code is installed from Anthropic's official source, unmodified, and updated on a regular schedule. Base images are rebuilt weekly with security patches.

**US only.** Compute and data live in US regions. Sign-up checks location and billing address.

## Stack

| Layer | Choice | Why |
| --- | --- | --- |
| Control plane, relay, AI gateway | Rust: Tokio, axum, sqlx | Long-running, concurrent services where predictable performance matters |
| Workspace agent | Rust static binary: Tokio and yamux, built against musl | One file with no dependencies, baked into every workspace image |
| Database | Managed Postgres, US region | Records, the run queue and events in one well-understood store |
| Files and logs | S3-compatible object storage, US region | Cheap and durable |
| Compute | Fly Machines, behind our driver interface | See Workspace compute |
| Web app | TypeScript with SvelteKit | The component library and views live here |
| Email | Chosen during development: a transactional email provider | R1 notifications |
| Observability | `tracing` for structured logs; Sentry-protocol error reporting with an optional DSN; `rustls` everywhere | One shared crate, `crates/telemetry`, so every service logs the same shape |
| Agent ↔ relay transport | A WebSocket on 443 with yamux multiplexing inside it | Crosses proxies, terminates beside the browser routes, gives independent streams over one outgoing connection |

## Delivery

How the system is built, tested and shipped. This was a separate document
until 2026-09-30; keeping it beside the design means a change to one is made
with the other in view.

### One repository

The platform lives in this single repository. The agent, relay, control plane and web app share one protocol, and most features touch several of them at once. In one repository, a protocol change and every side that uses it land in a single commit and are tested together.

Separate repositories only where something is published for others:

- **SDKs** (R2), such as `croncave` on PyPI and npm, so users can inspect the code they install.
- **Public templates**, possibly later, for community contributions.

### Planned layout

**This document owns the layout and the build order.** `AGENTS.md` summarises both and links here; neither is copied, because the two copies we once kept had already drifted apart.

Everything but `crates/ai-gateway`, `templates/` and `infra/` exists. Create each other part when its build step starts.

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

### What gets built and shipped

| Artifact | Built from | Notes |
| --- | --- | --- |
| Server image | control-plane, relay, ai-gateway | One binary with a role flag at first. Split into separate services when scale needs it |
| Agent binary | agent | Static and versioned. Updated when a workspace wakes |
| Workspace base image | images | A standard OCI image, so it runs on any provider |
| Web app | web | SvelteKit build |
| Database migrations | db | Run before each deploy. Old and new code must both work during a rollout |

### The compute abstraction

The `ComputeDriver` interface is the only place a provider appears: create, start, stop, suspend, snapshot, resize, destroy, status. It starts with three drivers:

1. **Fake:** in memory, for fast tests.
2. **Local:** Docker containers on a developer machine. They aren't VMs, but they behave the same from the agent's side.
3. **Fly:** the real provider for R1.

One shared test suite runs against every driver. A new driver (for example our own Firecracker hosts, or another provider if Fly's terms don't work out) only has to pass that suite. The control plane also runs as an ordinary container, not tied to Fly.

### Environments

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

### Build order and milestone checkpoints

Each step ends with a check that proves it works before the next one starts, and each is delivered in slices small enough to review in one sitting — step 0 took five.

The order is the repository's own. It began as the sequence sketched while planning, with a step 0 added for the repository setup that sketch did not cover, and has been changed since when building proved a different order better — each change recorded in `docs/decisions.md`.

| Step | Proven when |
| --- | --- |
| 0. Repo and CI ✅ | CI runs green, with logging and error tracking wired in |
| 1. Accounts and workspaces ✅ | A person signs in, and a user, a personal team and a workspace record exist and show in the app |
| 2. Workspace compute ✅ | The shared driver suite passes on fake and local (Docker) in CI, and a workspace starts, reports its state and stops from the app. The same suite against real Fly in staging follows once that account and its terms are settled |
| 3. Connection ✅ | A command sent from the control plane runs inside a local (Docker) workspace and its output comes back, over a connection the workspace opened itself. Nothing in the workspace listens, and it sleeps once nothing is happening. This is the first real milestone: a cloud computer reachable only through Croncave |
| 4. Claude Code sessions | A session keeps running after the browser closes, and caps stop it |
| 5. GitHub and change review | A session opens a pull request, and the review screen shows it |
| 6. Scheduled runs | A scheduled run wakes a workspace, runs, records the result and lets it sleep, and wake time is measured |
| 7. Home and timeline | "Needs you," "running" and "done" reflect real events, with usage and spending caps |
| 8. Previews, templates and dashboards | An app on localhost opens in the app from the preview domain, a template creates a working task, and a workspace shows a dashboard built from standard components |
| 9. Hardening | Security and abuse tests pass. Then the alpha goes live on the real domain |

**The browser terminal is not in step 3.** It was the demo this step used to prove itself, never an R1 commitment: `product.md` has "Open a terminal in the browser (expert layer)" as Should, R2, and the design canvas has no terminal screen. What step 3 proves instead is the thing the product is about — work happening in a workspace and results coming back. The connection underneath is not optional: steps 4, 6 and 8 all ride on it.

**Sleeping moved to step 3 on purpose.** Idle means no run or session active, no terminal or preview open and nobody looking — every one of those signals arrives with the agent. A timer in step 2 would sleep a workspace in the middle of work and call it idle.

Two notes on the checkpoints. **Step 2's is two-part on purpose** — local proven in CI, Fly proven in staging — so progress on the driver interface isn't blocked on confirming Fly's terms. **Step 8 covers three R1 "Must" features**, not just previews; templates and the first dashboard components belong there too.

### Before launch

- Get written confirmation from Fly that running a platform for our customers is allowed, including a markup on usage (see `pricing.md`).
- Confirm with Anthropic that the AI gateway attaching a user's own key is allowed, and that built-in features on our own API account, billed at cost, are fine (see AI access and secrets, above).
