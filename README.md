# Croncave

A private computer in the cloud that keeps working after you close your laptop. Hand off overnight AI coding sessions, scheduled scripts and monitors, then come back to plain-language results, a change review, or a private preview of what was built.

**Status:** step 1 is built — sign in with an emailed link, a personal team, and workspaces you can create and see. They have no computer yet; that is step 2.

## Getting started

Prerequisites: a stable Rust toolchain (`rustup`), Node 22 or newer, and Docker.

```
corepack enable pnpm     # once per machine; pins the pnpm version
cp .env.example .env     # local settings; git-ignored, never committed
docker compose up -d     # Postgres
./scripts/check.sh       # everything CI runs
```

To run the app, two processes:

```
cargo run -p croncave-control-plane    # the API, on :8080
cd web && pnpm dev                     # the web app, on :5173
```

Then open http://localhost:5173. There is no email provider yet, so a sign-in
link is written to the control plane's log — copy it from there.

The end-to-end tests need a browser once: `cd e2e && pnpm run install-browsers`.

`.env.example` lists every variable the code reads, with a comment on each.
Add a variable there whenever you add one to the code.

## Start here

- **[AGENTS.md](AGENTS.md):** the brief every coding agent reads first — product,
  principles, hard rules, and the conventions code is held to. `CLAUDE.md`
  imports it for Claude Code.
- **[docs/product.md](docs/product.md):** what Croncave is for, and every
  feature as a user story with its priority and release.
- **[docs/architecture.md](docs/architecture.md):** how it is designed, and in
  what order it is built.
- **[docs/pricing.md](docs/pricing.md):** tiers, usage billing and caps.
- **[docs/decisions.md](docs/decisions.md):** every choice with lasting impact,
  and what was rejected.
- **[docs/design/](docs/design/):** the design system and every designed screen.
