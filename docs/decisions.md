# Decisions

Every choice with lasting impact, newest first, with the reasoning and what was
rejected. The rejected options are the expensive part to reconstruct later, so
they are kept.

**Add to this file** rather than starting another. A decision contradicted by
experience gets a dated amendment rather than an edit, so the record still
shows what was believed and when.

Decisions 0001–0015 were made while planning, before any code existed, and are
summarised in a table at the end. Everything from 0016 was made while building,
and each has its own section.

## 0025: The repository is the source of truth, and there are four documents

- **Date:** 2026-09-30
- **Status:** Accepted

### Context

The product, the architecture and the pricing were written as documents on claude.ai and exported here as snapshots: read-only, with "the living doc wins" as the rule (decision 0019). Alongside them the repository had grown its own delivery plan, a conventions file, a folder of numbered decisions, and a 1,457-line transcript of the conversation that started it all. Eleven files, two ownership models, and a rule that the authoritative copies were the ones no agent could read and no pull request could review.

It did not hold. The snapshots fell behind twice in a fortnight — once on the repository layout, once on the whole observability stack — and each time the fix was to write down what the upstream document ought to say and hope someone pasted it in.

### Decision

- **This repository is the source of truth.** `product.md`, `architecture.md` and `pricing.md` are no longer snapshots. A change to the product or the design is made here, in the same pull request as the code it affects.
- **Four documents, and the brief.** `AGENTS.md` (with `CLAUDE.md` importing it), `docs/product.md`, `docs/architecture.md`, `docs/pricing.md` and `docs/decisions.md`. `docs/design/` keeps the design system beside the CSS the code imports.
- **The delivery plan is a section of the architecture**, not a document of its own. How a thing is built and how it is designed change together far more often than either changes alone.
- **The conventions are part of the brief.** They were a separate file that `AGENTS.md` summarised, which is two copies of one set of rules.
- **One decisions file, newest first**, added to rather than replaced. A decision contradicted by experience gets a dated amendment, so the record still shows what was believed and when.
- **The planning transcript is deleted.** It is in this repository's history if it is ever wanted; as a file it was a large document nobody read, describing a plan that has since changed in several places.

### Alternatives considered

- **Keep the snapshots and consolidate around them:** the merged architecture document would then be partly owned upstream and partly here, which is worse than either.
- **Keep the numbered decision files:** one file per decision is tidy in a listing and poor for reading. A single file can be searched in one pass, which is how a decision is actually looked up — "did we already decide this?"
- **Keep the delivery document separate:** a fair split on paper. In practice every build step changed both, and the two had already drifted.
- **Keep the transcript for provenance:** git keeps it. Carrying a 1,457-line file to preserve what version control already holds is a cost paid on every clone and every search.

### Consequences

- **The documents on claude.ai stop being authoritative.** They are where the thinking started. Nothing reads them now.
- The architecture document is the longest thing in the repository at around 430 lines, and grows as each step adds to Delivery. If it becomes unwieldy, the split to make is by subject — not design-versus-plan, which is the split that just failed.
- Recording a decision is now adding to one file rather than creating a numbered file and updating an index. That is deliberate: the friction was making decisions go unrecorded.
- Nothing is read-only any more, so nothing protects these documents from careless change. Review is what protects them, which is what protects the code too.

## 0024: WebSocket on 443 with yamux inside, and how a workspace proves who it is

- **Date:** 2026-09-30
- **Status:** Accepted

### Context

`docs/architecture.md` marked the transport **\[proposed\]**: "TLS on port 443 with a stream multiplexer (HTTP/2 or yamux)". Everything in Croncave rides on that choice — sessions, scheduled runs, logs, files and previews all travel over the one connection a workspace opens outward — so it had to be settled before any of them are built.

It also had to answer a question the architecture describes but does not specify: how a workspace proves which workspace it is, given that the thing asking is code we do not control, running in a container anyone's code may be running in.

### Decision

- **A WebSocket on 443, with yamux multiplexing inside it.** The agent dials out; the relay never dials in. WebSocket crosses proxies and corporate networks that only expect HTTP, terminates in axum beside the browser-facing routes, and needs no special network setup. yamux inside gives the independent streams the architecture lists, so a large file transfer cannot block a control message, and a terminal in R2 is a new stream kind rather than a new connection.
- **Authorisation happens before the multiplexer exists.** The hello exchange is plain WebSocket messages. Only once a credential is accepted does yamux start, so a connection that cannot prove who it is never gets streams at all.
- **A one-time bootstrap token, traded for a credential.** The orchestrator mints a bootstrap token on **every** start and puts it in the workspace's environment; the agent trades it on first connect. Both are stored only as hashes, like every other secret we issue. Trading is a single conditional `UPDATE`, so two agents racing the same token cannot both win.
- **The relay owns no database.** Who a credential belongs to is asked through an `Authoriser` the control plane implements, so the relay stays a library that can become its own process without dragging the schema along.
- **A protocol version, refused on mismatch.** An agent is baked into an image and may be months older than the relay it meets. Adding a field is safe, adding a variant is safe, changing a meaning is not — the same expand-then-contract rule the migrations follow.
- **Control messages are JSON.** The volume is tiny and being able to read a connection in a log is worth more than the bytes. Output is raw on its own stream, so the choice costs nothing where volume actually is.

### Alternatives considered

- **gRPC over HTTP/2 (tonic):** bidirectional streams and generated types for free, and HTTP/2 multiplexes natively with no yamux layer. Rejected because it shapes the protocol around gRPC's request-response model, and previews — arbitrary web traffic to a port inside a workspace — fit that model badly.
- **Raw TLS with yamux:** least overhead and total control of the wire, at the cost of WebSocket's ability to cross proxies that only speak HTTP, and of terminating TLS separately from the app.
- **One shared relay secret:** quickest to a working connection, and it means any workspace can impersonate any other. Exactly the kind of temporary path that is still there at launch.
- **A long-lived credential baked into the image:** no bootstrap exchange, and every copy of the image would hold the same key.
- **Keeping the credential across restarts:** fewer tokens minted. Rejected because a fresh start should get a fresh identity, so a container that comes back after being replaced cannot rejoin.

### Consequences

- **The `[proposed]` marker in `architecture.md` is now settled and wants updating upstream**: WebSocket with yamux inside, not "HTTP/2 or yamux".
- Adding a stream kind — files, previews, a terminal — is a variant in the protocol and a handler at each end. `StreamKind` already names the ones that are coming, and an agent refuses a kind it cannot serve rather than mishandling it.
- Credentials do not renew yet. A connection lasting longer than a day would be refused on reconnect; that matters first for step 4's long sessions, which is where renewal belongs.
- The relay holds connections in memory, so it is one process today. The architecture calls for stateless relays with a routing table, which is a change to where the registry lives rather than to the protocol.
- **Running the relay locally means binding more than loopback**, because workspaces are containers. Decision 0021's loopback default stands; local development opts out explicitly, and `.env.example` says why.

## 0023: The `ComputeDriver` interface, and what every provider must promise

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

`AGENTS.md` says nothing outside `crates/compute` may depend on a provider, and `docs/architecture.md` expects R1 on Fly Machines and a later move to our own Firecracker hosts. That only holds if the boundary is real, which means deciding what a provider is asked for, what it promises in return, and how a new one proves it.

Fly itself is deferred: it needs an organisation, a staging environment and written confirmation that running a platform for customers is allowed. None of that blocks the interface or the two drivers that run in CI.

### Decision

- **Five operations, not the eight the delivery plan listed**: `create`, `start`, `stop`, `status`, `destroy`. `suspend`, `snapshot` and `resize` are designed when a feature needs them, against a real provider. A trait is cheap to extend and expensive to get wrong in the abstract.
- **Every operation is idempotent.** Starting something already running succeeds; so does stopping something stopped, and destroying something already gone. An orchestrator whose connection drops mid-call must be able to simply try again rather than first work out how far the last attempt got.
- **`NotFound` is its own error**, separate from "the provider refused" and "the provider could not be reached". It is the one failure a caller can act on — make another — and collapsing the three would make that impossible.
- **The types carry no provider vocabulary.** No "machine", no "container", no region string that means something to one vendor. `ComputeId` is opaque: only the driver that issued it knows what it says.
- **One shared test suite, run unmodified against every driver.** It is the real deliverable of this step: a new provider is finished when the suite is green, which is a more useful definition than "it compiles".
- **Computers are made lazily, on first start.** A workspace is a record until someone runs something, so an account that signs up and never starts anything provisions nothing. On Fly a stopped machine's volume still bills.
- **The orchestrator is the only caller of a driver and the only writer of `workspaces.state`.** State written from two places is state that disagrees with itself.
- **The provider is the truth, not our database.** Reading a workspace asks the provider and writes down the answer. A computer removed behind our back is reported as asleep with its pointer forgotten, so the next start makes a new one instead of failing for ever.

### Alternatives considered

- **All eight operations now**, with the unused ones returning "not supported": every driver's shape visible from the start. Rejected because `snapshot` and `resize` would be designed before we know what they carry, and a wrong signature in a trait three drivers implement is expensive.
- **Making start fail when already running**, which is what a naive wrapper over Docker or Fly gives you: it pushes the retry problem onto every caller.
- **Creating the computer when the workspace record is created**: simpler state, and it makes every abandoned signup cost money.
- **Trusting `workspaces.state`** and refreshing it on a timer: fewer calls to the provider, and it would show someone "awake" for a workspace that is not there.
- **Shelling out to the `docker` CLI** instead of the API: no dependency, but it means parsing human-readable output and having no structured errors, which is exactly what the `NotFound` distinction depends on.

### Consequences

- Adding Fly is one file plus one line in the driver's configuration, and the suite says whether it is done.
- The local driver runs a placeholder image that **traps SIGTERM and exits**. Plain `sleep infinity` ignores signals, so Docker waited out the whole grace period and killed it: every stop took ten seconds, and the suite ran 8x slower. Whatever runs in a workspace in future must shut down on SIGTERM too.
- `bollard`'s defaults look only at `/var/run/docker.sock`, which does not exist under Docker Desktop on macOS. The driver also tries the per-user socket, or it would report "no Docker" on a machine plainly running it.
- Sleeping on idle is not here. Idle means no run or session active, no terminal or preview open and nobody looking, and every one of those signals arrives with the agent in step 3.

## 0022: Sign in with a link, and no passwords anywhere

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

`docs/product.md` promises "sign up and sign in with email, GitHub or Google" as a Must for R1. The design system settles the shape: the finished sign-in screen has one email field, a **Send sign-in link** button, the line "We'll email you a sign-in link. No password to remember," and its README says plainly that there is no Google or GitHub sign-in.

The reason to reach for passwords instead was that magic links appear to need an email provider, and `docs/architecture.md` still lists that provider as undecided. That turns out not to be true: which provider carries the message is a seam, not a prerequisite.

### Decision

- **Magic links, and no password field anywhere.** Nothing to hash, nothing to reset, nothing to leak, and no rework of two finished screens.
- **A `Mailer` trait decides nothing about the vendor.** `LogMailer` writes the link through `tracing` locally, `TestMailer` captures it, and `UnconfiguredMailer` — what a deployment gets until a provider is chosen — **refuses** rather than logging a live credential where anyone with log access could use it. Choosing a provider is one implementation.
- **The token is 32 random bytes from the operating system, and only its SHA-256 is stored.** SHA-256 rather than Argon2 on purpose: slow hashes exist to make *low-entropy* secrets expensive to guess, and there is nothing to guess in 256 random bits. A slow hash would only make every request slower.
- **A link works exactly once**, enforced by a single conditional `UPDATE … WHERE consumed_at IS NULL AND expires_at > now() RETURNING email`. Two browsers racing the same link cannot both match, so single use survives concurrency rather than depending on a read-then-write.
- **Fifteen minutes to use a link.** Long enough to switch to an email client, short enough that one left sitting in an inbox is not a standing key.
- **Asking for a link never reveals whether an account exists.** The status is identical either way, both cases send a message, and even the rate limit answers with the same 202 — because "too many requests" would itself confirm the address is in use.
- **Five links per address per fifteen minutes**, so the endpoint cannot be used to flood someone's inbox.
- **Sessions are rows, not signed cookies.** Opaque 32-byte secret, stored hashed, thirty days, `HttpOnly`, `SameSite=Lax`, `Secure` outside local development. `Lax` and not `Strict` because arriving from an email client is a cross-site navigation and `Strict` would drop the cookie exactly when it is needed.
- **An account and its personal team are created in one transaction** the first time a link is followed, so a half-made account cannot exist.

### Alternatives considered

- **Email and password first, magic links later:** what was asked for before the design was read. It would have meant a password field the design system has no component for, two finished screens redone, and a reset flow that needs email anyway.
- **Argon2 on the token:** defensible reflex, wrong tool. The input is already high-entropy.
- **Signed cookies (JWT):** no database read per request, but no revocation, and "sign out" would stop meaning anything.
- **Reading, checking, then consuming the link in separate statements:** the obvious shape, and it loses the race. Two tabs opened together would both succeed.
- **Answering "too many requests" when rate-limited:** honest to the user, and a way to enumerate accounts.
- **GitHub or Google sign-in:** ruled out by the design system, which says every user creates a Croncave account.

### Consequences

- **The email provider is still undecided and nothing is blocked by it.** Whoever it is, they implement one trait with one method.
- **A deployment without a provider cannot sign anyone in** — `UnconfiguredMailer` refuses — which is the right failure. Staging needs a real mailer before anyone can use it.
- **Invite-only access is not enforced yet.** Following a link creates an account, and the product says alpha access is by invitation. An allowlist check belongs at `request_link`, and is hardening's to add before the alpha opens.
- **Rate limiting lives in Postgres**, counted from `login_tokens`, which is fine at alpha size and will want moving if sign-in traffic ever grows.
- Sessions are checked against the database on every request. That is what makes revocation immediate; if it ever costs too much, a short-lived cache is the answer, not a signed cookie.

## 0021: How a Croncave service starts, reports health and stops

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

The control plane is the first long-running service, and the relay and AI gateway will be built the same way. Three things had to be settled once: how HTTP is served, when the schema is applied, and how an orchestrator learns whether an instance is working.

The delivery plan said migrations "run before each deploy," which describes the requirement but not the mechanism.

### Decision

- **axum, with the application built separately from the server.** `router(state)` returns the application and touches no network; `serve(config)` binds and runs it. Tests drive the routes through `oneshot` without binding a port, which is the same split as `build_subscriber` and `init` in `croncave-telemetry`.
- **Migrations run at startup, before the listener binds.** sqlx takes a lock while applying them, so several replicas starting at once is safe, and no binary can serve against a schema older than it expects. This satisfies "before each deploy" without a separate deploy step to forget. Migrations still have to work with the previous release's code during a rollout — expand first, contract later.
- **Liveness and readiness are different questions, on different paths.** `/health` says the process is running and touches nothing else. `/ready` says the database answers within two seconds, and returns 503 when it doesn't. A database blip should take an instance out of rotation, not have every instance restarted at once.
- **Loopback by default.** `CRONCAVE_BIND` defaults to `127.0.0.1:8080`. The browser reaches the control plane through the web app, and a service that binds a public interface should have to say so.
- **Graceful shutdown on SIGINT and SIGTERM**, then the pool closes, so work in flight finishes instead of being cut off mid-query.

### Alternatives considered

- **Migrations as a separate deploy step** (a job, or `sqlx migrate run` in CI): the conventional answer, and it keeps schema changes out of the serving path. Rejected for now because it is a step that can be skipped or run against the wrong environment, and because a developer's laptop then needs a second command to stay current. Worth revisiting when a migration is slow enough that startup time matters.
- **One `/health` that checks the database:** simpler, and wrong in the way that matters. An orchestrator reading it as liveness would restart every instance during a database blip, turning a recoverable problem into an outage.
- **Binding `0.0.0.0` by default:** what most examples do, and what makes a service reachable before anyone decided it should be.
- **No graceful shutdown:** fine for a stateless API, less fine once a request can be a long-running run or a session.

### Consequences

- Every later service — relay, AI gateway — copies this shape, and the `/health` and `/ready` pair is what deploy configuration will point at.
- Startup now depends on the database being reachable. That is deliberate: a control plane that cannot reach Postgres cannot do anything useful, and failing loudly at boot beats failing per request.
- A slow migration becomes slow startup. If that ever blocks a deploy, moving migrations to their own step is a contained change, since they already live in `crates/db` behind `migrate`.

## 0020: Postgres conventions — UUIDv7 keys, checked text, hashed tokens

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

The first migration sets patterns every later table copies, so the cheap moment to choose them is before there is a second one. Four questions had to be answered: what a primary key looks like, how a small set of allowed values is expressed, how sign-in secrets are stored, and what happens to a row when the thing it points at is deleted.

Two rules from `AGENTS.md` constrain the answers. Everything belongs to a team, never directly to a user, so that a personal team of one becomes a shared team without a migration. And every action is attributed.

### Decision

- **UUIDv7 primary keys**, generated by the application. Random-looking, so an id in a URL reveals nothing about how many exist or who else is on the platform, but time-ordered, so inserts stay at the end of the index instead of scattering through it.
- **Text with a `CHECK` constraint, not Postgres `enum` types**, for `teams.kind`, `memberships.role` and `workspaces.state`. A value can be added in one release and removed in a later one, which is what expand-then-contract needs; enum types make removal painful.
- **Sign-in tokens and session cookies are stored only as SHA-256 hashes**, in `bytea`. Reading `login_tokens` or `sessions` therefore grants nobody a sign-in, which is the difference between a leaked database being an incident and being a breach. Sessions are rows rather than signed cookies so they can be revoked.
- **Email is stored already lowercased and trimmed**, with a plain unique index. No `citext` extension to depend on in a managed Postgres.
- **Deletes cascade along ownership, never along attribution.** Deleting a team removes its workspaces; deleting a user removes their sessions and memberships. But `workspaces.created_by` has no cascade, so a user who made something cannot simply be deleted.
- **Migrations live in `crates/db/migrations` and are compiled into the binary** with `sqlx::migrate!`, so a deploy carries its own schema and the control plane applies it at startup.

### Alternatives considered

- **`bigserial` keys:** smaller and faster to join, but sequential ids in URLs leak volume and let one customer guess another's identifiers.
- **UUIDv4:** the same opacity without the index locality; every insert lands in a random page.
- **Postgres `enum` types:** better integrity and self-documenting, but `ALTER TYPE` cannot remove a value, so the contract half of a rollout has no clean move.
- **Storing tokens in plain text:** simpler lookups. Rejected outright — it turns a read of one table into every user's account.
- **Signed cookies (JWT) instead of session rows:** no database read per request, but no revocation either, and "sign out everywhere" stops being possible.
- **Cascading `created_by`:** would make account deletion a single statement, at the cost of silently rewriting who made what.

### Consequences

- **"Export or delete all my data" (Must, R2) needs a real design**, because deleting a user is deliberately not a `DELETE`. Reassigning attribution to a tombstone user, or anonymising in place, is the likely shape; it is a decision for that step, not this one.
- Every service needs the same UUIDv7 generation; it comes from the `uuid` crate, agreed once in `[workspace.dependencies]`.
- Hashing means a token can never be shown again after it is issued, which is correct for sign-in links and sessions but will need thought if an API key feature ever wants a "reveal" button.
- There is **no soft-delete column yet**, though the product promises deleted workspaces are hidden at once and purged after 30 days. Left out rather than guessed at; it arrives with the step that needs it.

## 0019: One build order, following architecture.md, and one owner per doc *(ownership superseded by 0025)*

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

Planning step 1 stalled on a contradiction. `AGENTS.md` said step 1 was "a browser terminal works on a local (Docker) workspace." `docs/architecture.md` said step 1 was a skeleton — sign-in, teams, Postgres, a SvelteKit shell — and put the connection third. Both were checked in, and neither said which to follow.

Auditing the two against `docs/product.md` turned up more than an ordering disagreement:

- **The build order existed three times**: in `AGENTS.md`, in `docs/architecture.md` and in `docs/architecture.md`. The two repository copies were meant to be identical and **7 of their 9 rows had already drifted in wording** — harmlessly so far, which is how duplication rots.
- **The repository's order was missing work the product marks "Must, R1."** Sign-in, teams and Postgres had no step at all. Starter templates and the first dashboard components had been dropped from the extras step.
- **`AGENTS.md`'s step 1 assumed things it never stated**: that a local compute driver existed and that a workspace record existed, because a terminal must attach to something and the relay must authorise the browser against something. That unstated assumption is what made the step unplannable.
- **The repository layout was duplicated the same way**, identical only because step 0 edited both copies by hand.

Underneath all of it is a question nobody had answered in writing: which docs may be changed here, and which are snapshots of living docs that only the account owner can change?

### Decision

- **`docs/architecture.md`'s build order wins**, and the repository's table is renumbered to match it: accounts, compute, connection, sessions, GitHub, scheduled runs, home and timeline, extras, hardening. A step 0 is kept for the repository setup that list doesn't cover. Only step 0 had shipped, so renumbering cost nothing.
- **`docs/architecture.md` owns the repository layout and the build order.** `AGENTS.md` summarises both and links there, the way it already treats conventions. Neither is copied.
- **Doc ownership is written down** in `AGENTS.md` and the Conventions section of `AGENTS.md`: `product-definition.md`, `architecture.md` and `pricing.md` are read-only snapshots; `AGENTS.md`, `delivery.md`, `conventions.md` and `decisions/` are repo-native. A decision made while building is recorded in a repo-native doc, and the gap it leaves in a snapshot is reported to the user as exact upstream wording.

### Alternatives considered

- **Keep the repository's ordering and change the living doc to match.** The repository's order reached the architecture-proving milestone — a cloud computer reachable only through Croncave — sooner, which is worth something. But it got there by leaving out an R1 commitment, and the skeleton it skipped is what the connection needs underneath it. The living doc was right.
- **Label the two orderings as "strategic" and "execution" and keep both.** Least work, and it would have papered over exactly the confusion that caused this entry.
- **Make `AGENTS.md` canonical instead of `delivery.md`.** Fewer hops for an agent, which reads `AGENTS.md` in full anyway. Rejected because `AGENTS.md` would then grow with every build step, and its own table of contents would stop matching the files it describes.
- **Sync the wording and keep both copies.** Fixes today's drift and guarantees tomorrow's.

### Consequences

- **"Step 1" now means accounts and workspaces.** The agent-and-relay work is step 3. Anything written before 2026-09-29 that refers to step numbers means the old scheme.
- Adding or changing a step edits one file.
- `docs/architecture.md` still doesn't know what step 0 decided: its stack table has no observability row, and nothing there mentions `tracing`, Sentry, `rustls`, `adapter-node`, pnpm or `crates/telemetry`. Decisions 0016–0018 live only here. The exact upstream wording has been given to the account owner; until it is applied, the snapshot lags the repository — in the direction the repository has just been corrected *to*, which is the safe direction, but still a gap.
- Snapshots will keep falling behind as we build, because that is what snapshots do. The rule that makes it survivable is that the gap gets reported rather than silently patched into the snapshot, where the next export would overwrite it.

### Amendment, 2026-09-30: the snapshot model is gone

The ownership half of this decision has been replaced by decision 0025. There
are no snapshots any more: the product, the architecture and the pricing live
in this repository and are changed here. The other half — one build order, one
owner per thing, no copies — stands, and is why the consolidation had somewhere
obvious to put everything.

## 0018: Web app on SvelteKit with adapter-node, pnpm pinned by corepack

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

`docs/architecture.md` already chose TypeScript and SvelteKit for the web app, and the view component library lives there too. Step 0 has to turn that into a skeleton that builds, lints and tests, so the CI web job can switch on and the connection step has somewhere to put a browser terminal. The details left open were the adapter, the package manager, how strict the TypeScript settings are, and where the app reads its settings from.

### Decision

- **SvelteKit 2 with Svelte 5** (runes), and **`@sveltejs/adapter-node`**. The web app ships as a container like the Rust services (`docs/architecture.md`, "What gets built and shipped"), so it needs a plain Node server, not a platform adapter. `adapter-auto` would guess, and guessing a platform is exactly what we don't want while compute providers are still open.
- **pnpm, pinned by the `packageManager` field** in `web/package.json` and enabled with `corepack enable pnpm`. No separate install, and CI and a developer machine run the same pnpm version.
- **TypeScript in strict mode, pinned to `^6`.** TypeScript 7 is released, but SvelteKit's peer range is `^5.3.3 || ^6.0.0` and `typescript-eslint` requires `<6.1.0`. Revisit when both have moved.
- **ESLint (flat config) + Prettier + `svelte-check` + Vitest**, wired to `pnpm lint`, `pnpm check` and `pnpm test`, which is what `scripts/check.sh` and CI call.
- **pnpm's `minimumReleaseAge` guard stays on.** pnpm 12 refuses packages published in the last day or so, which is a cheap defence against a compromised release being installed before anyone notices. When it blocked `typescript-eslint@8.71.0` (published the day before), we took the previous release rather than writing the exclusion list pnpm offered. **Never add `minimumReleaseAgeExclude` to get an install moving** — pin to an older version instead.
- **The app reads the repository-root `.env`**, via `kit.env.dir: '..'` in `svelte.config.js`, so one file serves the Rust services and the web app (decision 0016, and `.env.example`).
- **Environment names are parsed the same way on both sides.** `web/src/lib/environment.ts` accepts the same four names and aliases as `Environment` in `crates/telemetry`, and throws on anything else rather than assuming `local`.

### Alternatives considered

- **`adapter-auto`:** less configuration, but it picks a target from the CI environment it happens to build in. We'd rather say what we deploy.
- **npm or Bun:** npm needs no setup but installs slower and its flat `node_modules` hides undeclared dependencies; Bun is one fast tool for install, scripts and tests, but is a second runtime to pin and a less-trodden path for SvelteKit and Playwright, and our servers are Rust so its runtime speed buys us nothing.
- **TypeScript 7:** the newest, but outside SvelteKit's and typescript-eslint's peer ranges today.
- **Excluding fresh packages from the release-age policy:** pnpm writes the exclusion list for you, which makes it the path of least resistance. It also silently removes the protection for exactly the package that is new enough to be worth checking.
- **Vite's `envDir` alone:** it only covers `import.meta.env`. `$env/dynamic/private` reads `kit.env.dir`, which is the setting that actually matters. Both are set, to the same place.
- **Duplicating the environment names only in the Rust crate:** the web app would have had to hardcode strings, and the two would drift the first time a name changed.

### Consequences

- `corepack enable pnpm` is a one-time step on a new machine, and belongs in the README.
- The whole web toolchain is pinned in one lockfile, committed, and CI installs with `--frozen-lockfile`, so a dependency cannot change under us without a visible diff.
- Two definitions of the environment names now exist, in Rust and TypeScript. They are small and tested on both sides; if more shared types appear (they will, with the protocol crate in the connection step), generating the TypeScript from Rust is the answer rather than a third hand-written copy.
- Upgrading TypeScript is now gated on SvelteKit's peer range, which is worth rechecking when either moves.

## 0017: Structured logs with `tracing`, error reporting over the Sentry protocol

- **Date:** 2026-09-29
- **Status:** Accepted

### Context

Step 0 is proven when "CI runs green; logging and error tracking are wired in," but no planning document picked the tooling. Croncave's shape makes the choice matter more than usual: a run happens across the control plane, the relay and an agent inside a workspace, and when it goes wrong the founder is reading three services' output side by side to find out why. Work also happens while nobody is watching — that is the product — so a failure at 03:00 has to reach someone without a person tailing a log.

Two constraints shaped the answer. Contributors and CI must not need an account or a secret to run the code. And the agent is a static binary shipped into every workspace image, so its dependencies must not drag in OpenSSL.

### Decision

- **`tracing` for structured logs**, configured once in `crates/telemetry`. Pretty lines when a person is watching a terminal, one JSON object per line otherwise, chosen automatically by whether stdout is a terminal and overridable with `CRONCAVE_LOG_FORMAT`. Every JSON event carries the service's name, version and environment.
- **Error reporting over the Sentry protocol**, via the `sentry` and `sentry-tracing` crates. Every `tracing::error!` becomes an event, quieter events become breadcrumbs on it, and panics are reported.
- **The DSN is optional.** `SENTRY_DSN` unset means no client is created and reporting is off. Local runs and CI therefore need no account and no secret.
- **`rustls`, not `native-tls`**, so nothing in the dependency tree needs OpenSSL.
- **No performance tracing and `send_default_pii: false`** for R1. We decide what a report contains, not the SDK.

### Alternatives considered

- **OpenTelemetry over OTLP:** the most portable, and where we will probably end up when there are enough services to trace a request across. It needs a collector to run somewhere before it's useful, which is machinery to operate before there is anything to observe. Adding it later is a layer in one crate, not a rewrite.
- **Structured logs only, error tracking later:** would leave the step 0 checkpoint half met, and the first unattended overnight failure is exactly what we want to hear about.
- **A log-search product (Axiom, Better Stack) as the only destination:** good for searching, but no grouping, no deduplication and no "this started at release X" — which is what you want when a run fails at 03:00.
- **`native-tls`:** the default, and would mean OpenSSL in the agent's static binary.

### Consequences

- Where Sentry runs is not decided here and does not need to be. The same SDK speaks to hosted Sentry, self-hosted Sentry and GlitchTip; the choice is a DSN in an environment's secret store.
- The `SENTRY_DSN` for staging and production goes in each environment's secret store, never in the repository.
- Because `SENTRY_DSN` is read at startup, an invalid DSN stops the service rather than silently disabling reporting. The same applies to `CRONCAVE_ENV` and `CRONCAVE_LOG_FORMAT`.
- `Config`'s `Debug` implementation redacts the DSN, so dumping a configuration into a log cannot leak it.
- Log volume is now a cost we control with `RUST_LOG` per environment. Workspace agents buffer and forward logs over the relay (see `docs/architecture.md`), and how those reach the same place belongs to the connection step, not step 0.

### Amendment, 2026-09-29: the JavaScript SDK's collection defaults

Wiring the same posture into the web app turned out not to be symmetrical. The Rust SDK has one switch, `send_default_pii`, which is off by default. The JavaScript SDK version 11 replaced that switch with a `dataCollection` object whose defaults are permissive: cookies, request and response headers, request and response bodies, URL query parameters, database query data, queue arguments and the values of local variables in stack frames are all collected unless you say otherwise.

For a product running people's private workspaces those are the wrong defaults, and they are the kind that arrive quietly in a dependency upgrade. So `web/src/lib/sentry.ts` turns all of them off explicitly, both hooks use it, and a test asserts that every field in it collects nothing — which fails if a field is ever switched on, or if a new permissive field is added and defaulted in. Source context lines are the one thing left on: that is our own code, and it is what makes a stack trace readable.

Two related notes from the same wiring:

- Disabling performance tracing means leaving `tracesSampleRate` **unset**. Setting it to `0` still turns tracing on and then samples none of it.
- The browser can only be given values whose names begin with `PUBLIC_`, so `PUBLIC_SENTRY_DSN` and `PUBLIC_CRONCAVE_ENV` mirror the private variables for code running in the page. The private ones stay authoritative everywhere on the server.

## 0016: Cargo workspace, a shared telemetry crate, and one check script

- **Date:** 2026-09-28
- **Status:** Accepted

### Context

Step 0 creates the repository skeleton every later step builds on. Three things had to be settled before any code could land:

1. How Rust crates are arranged and how versions are agreed across them.
2. Where logging and error reporting are configured. The control plane, relay, AI gateway and agent all need the same setup, and `AGENTS.md`'s planned layout had no home for it.
3. How a developer runs the same checks CI runs. Local and CI checks that are written out twice drift apart, and the difference is always found in a failing pipeline rather than before the push.

### Decision

- **One Cargo workspace at the repository root**, with `[workspace.package]` and `[workspace.dependencies]` holding the shared version, edition and dependency versions. Crates inherit them with `version.workspace = true` and `foo.workspace = true`. Workspace lints forbid `unsafe_code` and warn on `unwrap`/`expect`; CI turns warnings into errors.
- **A shared `crates/telemetry` crate** (package `croncave-telemetry`), added to the planned layout in `AGENTS.md` and `docs/architecture.md`. Every service calls `telemetry::init` once and gets identical logging.
- **`scripts/check.sh` is the single check entry point**, and CI calls it rather than repeating the commands. It skips parts of the repository that don't exist yet, so it stays usable as crates and the web app appear.

### Alternatives considered

- **A crate per service with its own logging setup:** no new crate in the layout, but the setup would be copied four times and drift, and a change to the log shape would touch every service.
- **Put the setup in `crates/protocol`:** protocol is agent ↔ relay message types; mixing process-wide setup into it would make a dependency-free types crate depend on a subscriber stack.
- **A Makefile, `just`, or `cargo-xtask` as the runner:** all fine, but each is either an extra install (`just`, `mise`), quirky syntax (Make), or a crate that recompiles for a one-line change (xtask). A shell script needs nothing and is easy to extend when later steps add services to start.
- **Separate local and CI command lists:** rejected for the drift described above.

### Consequences

- Adding a crate is a one-line `members` change plus inherited metadata; a dependency version is agreed in one place.
- Every service's logs have the same shape from day one, which matters when the relay, control plane and agent are being read side by side to debug one run.
- `scripts/check.sh` becomes the place where any new check must be added; a check added only to CI is a mistake to catch in review.
- Bash is now a dependency of the development loop. If it becomes limiting (parallelism, Windows contributors), moving to `just` or `xtask` is a contained change, since CI calls only the one entry point.

## 0001–0015: decided while planning

Made before any code existed, and summarised rather than written out: the
reasoning is in the documents each points to, and several have since been
revisited by the entries above.

| # | Decision | Where to read more |
| --- | --- | --- |
| 0001 | Name: Croncave (working name). App at `app.croncave.com` | product.md, Decisions |
| 0002 | Core model: workspaces with tasks; standalone tasks get a hidden workspace; no desktop UI | product.md |
| 0003 | Workspaces never accept inbound connections; one outgoing agent connection carries everything | architecture.md, The outgoing connection |
| 0004 | Compute: rent Fly Machines for R1 behind a `ComputeDriver` interface; later our own Firecracker system on a cloud provider | architecture.md, Workspace compute |
| 0005 | Sleep by default: boot from disk (under 10 s target), fast memory-snapshot tier for small workspaces; 10-minute idle timeout, adjustable | architecture.md, Workspace lifecycle |
| 0006 | Previews: in-app with new-tab fallback, on a separately registered domain, one subdomain per preview | architecture.md, Private previews |
| 0007 | AI access: Claude Code unmodified; API key via our gateway by default (env-var mode built first as fallback); users' own Claude subscriptions via Claude Code's own sign-in | architecture.md, AI access and secrets |
| 0008 | Models: Claude only in R1; later opt-in US-origin open models via OpenCode; no models from non-US labs | architecture.md, Models and the router |
| 0009 | Scheduler: our own on Postgres; add workflow-engine features rather than migrating | architecture.md, Scheduler, runs and events |
| 0010 | Stack: Rust (Tokio, axum, sqlx) backend and agent; TypeScript + SvelteKit web app | architecture.md, Stack and build order |
| 0011 | Team-ready data model from day one; every workspace runs as a container inside its VM so packing can come in R2 | architecture.md, Data model; Security |
| 0012 | Pricing: Free, Plus, Pro subscriptions plus usage. AI usage at exactly cost; other usage at cost plus a percentage set after R1 | pricing.md |
| 0013 | US only at launch, for users, compute and data | product.md, Decisions |
| 0014 | One monorepo; local, CI, staging and production environments; milestone checkpoints | architecture.md, Delivery |
| 0015 | Develop locally with the founder until R1, plan-before-code for every step | AGENTS.md |
| 0001 | Name: Croncave (working name). App at `app.croncave.com` | product.md, Decisions |
| 0002 | Core model: workspaces with tasks; standalone tasks get a hidden workspace; no desktop UI | product.md |
| 0003 | Workspaces never accept inbound connections; one outgoing agent connection carries everything | architecture.md, The outgoing connection |
| 0004 | Compute: rent Fly Machines for R1 behind a `ComputeDriver` interface; later our own Firecracker system on a cloud provider | architecture.md, Workspace compute |
| 0005 | Sleep by default: boot from disk (under 10 s target), fast memory-snapshot tier for small workspaces; 10-minute idle timeout, adjustable | architecture.md, Workspace lifecycle |
| 0006 | Previews: in-app with new-tab fallback, on a separately registered domain, one subdomain per preview | architecture.md, Private previews |
| 0007 | AI access: Claude Code unmodified; API key via our gateway by default (env-var mode built first as fallback); users' own Claude subscriptions via Claude Code's own sign-in | architecture.md, AI access and secrets |
| 0008 | Models: Claude only in R1; later opt-in US-origin open models via OpenCode; no models from non-US labs | architecture.md, Models and the router |
| 0009 | Scheduler: our own on Postgres; add workflow-engine features rather than migrating | architecture.md, Scheduler, runs and events |
| 0010 | Stack: Rust (Tokio, axum, sqlx) backend and agent; TypeScript + SvelteKit web app | architecture.md, Stack and build order |
| 0011 | Team-ready data model from day one; every workspace runs as a container inside its VM so packing can come in R2 | architecture.md, Data model; Security |
| 0012 | Pricing: Free, Plus, Pro subscriptions plus usage. AI usage at exactly cost; other usage at cost plus a percentage set after R1 | pricing.md |
| 0013 | US only at launch, for users, compute and data | product.md, Decisions |
| 0014 | One monorepo; local, CI, staging and production environments; milestone checkpoints | architecture.md, Delivery |
| 0015 | Develop locally with the founder until R1, plan-before-code for every step | AGENTS.md |

### The open questions those decisions closed

Carried over from the architecture document, where they were tracked while
planning. Each was settled; the two still needing outside confirmation are
listed under Before launch in `architecture.md`.

- **Compute provider.** Decided: Fly Machines, from the side-by-side analysis in Workspace compute. Prices and acceptable-use terms still to check.
- **AI gateway and Anthropic's terms.** Decided: build the environment-variable mode first, then the gateway as the default. Confirm the terms with Anthropic during development.
- **Idle timeout.** Decided: 10 minutes by default, adjustable per workspace in the details layer.
- **Resource presets.** Decided during implementation, using real usage.
- **Preview domain,** and where previews open. Decided: inside the app, with a new-tab fallback. Domain decided: a separately registered domain with one subdomain per preview, never a path under croncave.com. The exact name is picked when registering.
- **Web app framework** and email provider. Decided: TypeScript with SvelteKit. The email provider is chosen during development.
- **Metering subscription sessions.** Decided: awake time is enough for caps, and the product says so explicitly.
- **Packing workspaces.** When is putting several of one user's workspaces in one VM worth the complexity? Decided: R2, since it cuts costs and informs pricing. R1 runs each workspace as a container, so packing is easy later.
