# Conventions

How code in this repository is written, checked and landed. Refined during build step 0; `AGENTS.md` points here. When a convention changes, change it here and say why in `docs/decisions/`.

## Running the checks

`./scripts/check.sh` runs everything CI runs: `rust` or `web` narrows it to one side. **CI calls the same script**, so a check added only to the workflow file is a bug — add it to the script instead.

```
corepack enable pnpm     # once per machine
cp .env.example .env     # once per clone
./scripts/check.sh
```

## Rust

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

## Logging and errors

- **Every service uses `croncave-telemetry`.** Call `init` once at startup and hold the `Guard` until the process exits. Don't build a second subscriber.
- **Never log a secret.** No tokens, API keys, DSNs, GitHub credentials or workspace credentials in a field, a message or a `Debug` implementation. `Config` in `croncave-telemetry` redacts its DSN; do the same for any type that carries one.
- **`tracing::error!` means "a person should see this."** It becomes a Sentry event. Use `warn!` for something recovered from, and put the detail in fields rather than in the message text, so it stays searchable.
- **Error reporting is off without a DSN**, by design. Don't add a fallback that reports somewhere else.

## Configuration

- **Every variable the code reads has an entry in `.env.example`**, with a comment. That file is the list a developer checks when something isn't set.
- **Binaries load `.env` first**, with `dotenvy::dotenv().ok()`, before reading any configuration. Libraries never do.
- **An unrecognised value is an error, not a guess.** A typo in a deployment's `CRONCAVE_ENV` must stop the service, never quietly make production look local. Blank or unset may fall back to a default; wrong may not.
- **Secrets are never committed**, in code, fixtures, logs or test data. They live in each environment's secret store.

## TypeScript and the web app

- **Strict mode, no `any`.** `pnpm check` runs `svelte-check`, and it must be clean.
- **pnpm, pinned by the `packageManager` field** and enabled with `corepack enable pnpm`. The lockfile is committed and CI installs with `--frozen-lockfile`.
- **Tests are colocated** as `*.test.ts` next to the module, run by Vitest.
- **Prettier and ESLint are not negotiable**; `pnpm format` fixes what it can.
- **The design system is imported, never copied.** `web/src/app.css` reads `docs/design/system/tokens.css` and `components.css` directly, because that folder is a snapshot of the canvas and gets re-exported. Use tokens (`var(--space-4)`), never raw values.
- **Shared meaning stays in step with Rust.** `web/src/lib/environment.ts` deliberately mirrors `Environment` in `crates/telemetry`. When the protocol crate arrives, generate TypeScript from Rust rather than writing a third copy by hand.

## Dependencies

- **pnpm refuses packages published in the last day or so.** That is a supply-chain guard, and pnpm will offer to write a `minimumReleaseAgeExclude` list to get past it. **Never accept that offer** — pin to the previous release instead, as `typescript-eslint` is pinned to `~8.70.1`.
- **Prefer `rustls` over `native-tls`**, so nothing drags OpenSSL into the agent's static binary.
- **Check a new dependency's defaults, not just its API.** Sentry's JavaScript SDK defaults to collecting cookies, headers and request bodies; `web/src/lib/sentry.ts` turns that off and a test holds it off.

## Talking to Postgres

**Queries are runtime-checked (`sqlx::query`), not the `query!` macros.** The macros check SQL against a live schema at compile time, which is genuinely valuable, but it means `cargo build` needs either a running database or an `.sqlx` cache that someone has to remember to regenerate. Step 0 deliberately made the checks runnable without services, and that is worth more. The tests cover the queries against a real Postgres instead.

## Tests

- **Unit and integration tests live with their crate**; the database ones run against a real Postgres through `#[sqlx::test]`, which gives each test its own database.
- **End-to-end tests live in `e2e/`**, not in `web/`. They drive a real browser against a real control plane against a real Postgres, so they belong to the whole system rather than to the web app.
- **Never add a test-only endpoint to production code.** The end-to-end tests read a sign-in link from the control plane's log, the way a developer does, because only the token's hash is stored and there is deliberately no way to read one back. A convenience endpoint would be a bypass living in the shipped binary.
- **Give each end-to-end test its own account** (`freshEmail()`), so runs never collide with each other or with whatever is already in the database.
- **A test that cannot fail is not a test.** When a test guards something that matters, break the thing once and watch that test — and ideally only that test — go red.

## Database migrations

Migrations will live in `crates/db` (created in the accounts and workspaces step) and run before each deploy. **Old and new code must both work while a rollout is in progress:** expand first (add the column, backfill, start writing to it), then contract in a later release (stop reading the old column, drop it). Never rename or drop in the same release that stops using something.

## Git

- **Branch per build step or fix**, named for the work: `step-0-repo-and-ci`.
- **Commit messages:** an imperative summary line, then a body explaining *why*. The diff already shows what changed; the body is for the reasoning that isn't in it.
- **Small, reviewable commits.** A commit should be readable in one sitting and leave the checks green.
- **The author commits, not an agent.** An agent finishes the work, runs the checks and reports; the person reviews the working tree and decides when it is committed and pushed.
- **Pull requests** use `.github/pull_request_template.md`. Its checklist is not decoration: workspaces never listen, nothing outside the compute driver depends on a provider, no secrets, docs updated.

## Decisions

**Know which docs you may change.** `product-definition.md`, `architecture.md` and `pricing.md` are snapshots of living docs on claude.ai, and `design/` is a snapshot of the design canvas: read-only here, and the living doc or canvas wins. `AGENTS.md`, `delivery.md`, this file and `decisions/` are repo-native and are where decisions made while building are recorded. When a decision leaves a gap in a snapshot, tell the user the exact upstream wording rather than editing the snapshot.

**One copy of each thing.** `delivery.md` owns the repository layout and the build order; `AGENTS.md` summarises and links. Don't paste either back — keeping two copies is what let them drift.

Anything with lasting impact — a library, a protocol shape, a schema pattern, a default that affects privacy or cost — gets a numbered entry in `docs/decisions/`, using the template, with a row added to the index. Record the alternatives you rejected and why; that is the part that is expensive to reconstruct later. If a decision is later contradicted by experience, amend the entry with a dated section rather than editing history.
