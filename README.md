# Croncave

A private computer in the cloud that keeps working after you close your laptop. Hand off overnight AI coding sessions, scheduled scripts and monitors, then come back to plain-language results, a change review, or a private preview of what was built.

**Status:** planning complete, implementation not started. This repository currently holds the project context and CI only.

## Getting started

Prerequisites: a stable Rust toolchain (`rustup`) and Node 22 or newer.

```
corepack enable pnpm     # once per machine; pins the pnpm version
cp .env.example .env     # local settings; git-ignored, never committed
./scripts/check.sh       # everything CI runs
```

To run the web app: `cd web && pnpm dev`.

`.env.example` lists every variable the code reads, with a comment on each.
Add a variable there whenever you add one to the code.

## Start here

- **[AGENTS.md](AGENTS.md):** the full project brief (product, principles, architecture, hard rules, build order). Coding agents read this first; `CLAUDE.md` imports it for Claude Code.
- **[docs/](docs/):**
  - [product-definition.md](docs/product-definition.md): PR/FAQ and the feature story map
  - [architecture.md](docs/architecture.md): technical architecture
  - [pricing.md](docs/pricing.md): tiers and usage billing
  - [delivery.md](docs/delivery.md): repository layout, environments, testing and milestones
  - [decisions/](docs/decisions/): decision log
  - [design/mockups.md](docs/design/mockups.md): UI mockups
  - [history/planning-transcript.md](docs/history/planning-transcript.md): the planning conversation

## Living documents

The docs in `docs/` are snapshots. The living versions (private to the account owner):

| Doc | Link |
| --- | --- |
| Product definition | https://claude.ai/code/artifact/239e25ad-38d2-4426-b307-9d6c29ed40d2 |
| Technical architecture | https://claude.ai/code/artifact/33224792-3002-4d1a-b0d4-a1d9686c2315 |
| Pricing | https://claude.ai/code/artifact/8315518f-e11a-4644-8ac4-1ffa6c6cdf31 |
| UI mockups (design canvas) | https://claude.ai/artifact/UiRePeq7vzwoqQfAosvUG5 |
| Planning conversation | https://claude.ai/code/session_01RBHRWXhyB6ekKdP7Udki85 |
