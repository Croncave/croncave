# 0016: Cargo workspace, a shared telemetry crate, and one check script

- **Date:** 2026-09-28
- **Status:** Accepted

## Context

Step 0 creates the repository skeleton every later step builds on. Three things had to be settled before any code could land:

1. How Rust crates are arranged and how versions are agreed across them.
2. Where logging and error reporting are configured. The control plane, relay, AI gateway and agent all need the same setup, and `AGENTS.md`'s planned layout had no home for it.
3. How a developer runs the same checks CI runs. Local and CI checks that are written out twice drift apart, and the difference is always found in a failing pipeline rather than before the push.

## Decision

- **One Cargo workspace at the repository root**, with `[workspace.package]` and `[workspace.dependencies]` holding the shared version, edition and dependency versions. Crates inherit them with `version.workspace = true` and `foo.workspace = true`. Workspace lints forbid `unsafe_code` and warn on `unwrap`/`expect`; CI turns warnings into errors.
- **A shared `crates/telemetry` crate** (package `croncave-telemetry`), added to the planned layout in `AGENTS.md` and `docs/delivery.md`. Every service calls `telemetry::init` once and gets identical logging.
- **`scripts/check.sh` is the single check entry point**, and CI calls it rather than repeating the commands. It skips parts of the repository that don't exist yet, so it stays usable as crates and the web app appear.

## Alternatives considered

- **A crate per service with its own logging setup:** no new crate in the layout, but the setup would be copied four times and drift, and a change to the log shape would touch every service.
- **Put the setup in `crates/protocol`:** protocol is agent ↔ relay message types; mixing process-wide setup into it would make a dependency-free types crate depend on a subscriber stack.
- **A Makefile, `just`, or `cargo-xtask` as the runner:** all fine, but each is either an extra install (`just`, `mise`), quirky syntax (Make), or a crate that recompiles for a one-line change (xtask). A shell script needs nothing and is easy to extend when later steps add services to start.
- **Separate local and CI command lists:** rejected for the drift described above.

## Consequences

- Adding a crate is a one-line `members` change plus inherited metadata; a dependency version is agreed in one place.
- Every service's logs have the same shape from day one, which matters when the relay, control plane and agent are being read side by side to debug one run.
- `scripts/check.sh` becomes the place where any new check must be added; a check added only to CI is a mistake to catch in review.
- Bash is now a dependency of the development loop. If it becomes limiting (parallelism, Windows contributors), moving to `just` or `xtask` is a contained change, since CI calls only the one entry point.
