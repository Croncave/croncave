# 0023: The `ComputeDriver` interface, and what every provider must promise

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

`AGENTS.md` says nothing outside `crates/compute` may depend on a provider, and `docs/architecture.md` expects R1 on Fly Machines and a later move to our own Firecracker hosts. That only holds if the boundary is real, which means deciding what a provider is asked for, what it promises in return, and how a new one proves it.

Fly itself is deferred: it needs an organisation, a staging environment and written confirmation that running a platform for customers is allowed. None of that blocks the interface or the two drivers that run in CI.

## Decision

- **Five operations, not the eight `delivery.md` listed**: `create`, `start`, `stop`, `status`, `destroy`. `suspend`, `snapshot` and `resize` are designed when a feature needs them, against a real provider. A trait is cheap to extend and expensive to get wrong in the abstract.
- **Every operation is idempotent.** Starting something already running succeeds; so does stopping something stopped, and destroying something already gone. An orchestrator whose connection drops mid-call must be able to simply try again rather than first work out how far the last attempt got.
- **`NotFound` is its own error**, separate from "the provider refused" and "the provider could not be reached". It is the one failure a caller can act on — make another — and collapsing the three would make that impossible.
- **The types carry no provider vocabulary.** No "machine", no "container", no region string that means something to one vendor. `ComputeId` is opaque: only the driver that issued it knows what it says.
- **One shared test suite, run unmodified against every driver.** It is the real deliverable of this step: a new provider is finished when the suite is green, which is a more useful definition than "it compiles".
- **Computers are made lazily, on first start.** A workspace is a record until someone runs something, so an account that signs up and never starts anything provisions nothing. On Fly a stopped machine's volume still bills.
- **The orchestrator is the only caller of a driver and the only writer of `workspaces.state`.** State written from two places is state that disagrees with itself.
- **The provider is the truth, not our database.** Reading a workspace asks the provider and writes down the answer. A computer removed behind our back is reported as asleep with its pointer forgotten, so the next start makes a new one instead of failing for ever.

## Alternatives considered

- **All eight operations now**, with the unused ones returning "not supported": every driver's shape visible from the start. Rejected because `snapshot` and `resize` would be designed before we know what they carry, and a wrong signature in a trait three drivers implement is expensive.
- **Making start fail when already running**, which is what a naive wrapper over Docker or Fly gives you: it pushes the retry problem onto every caller.
- **Creating the computer when the workspace record is created**: simpler state, and it makes every abandoned signup cost money.
- **Trusting `workspaces.state`** and refreshing it on a timer: fewer calls to the provider, and it would show someone "awake" for a workspace that is not there.
- **Shelling out to the `docker` CLI** instead of the API: no dependency, but it means parsing human-readable output and having no structured errors, which is exactly what the `NotFound` distinction depends on.

## Consequences

- Adding Fly is one file plus one line in the driver's configuration, and the suite says whether it is done.
- The local driver runs a placeholder image that **traps SIGTERM and exits**. Plain `sleep infinity` ignores signals, so Docker waited out the whole grace period and killed it: every stop took ten seconds, and the suite ran 8x slower. Whatever runs in a workspace in future must shut down on SIGTERM too.
- `bollard`'s defaults look only at `/var/run/docker.sock`, which does not exist under Docker Desktop on macOS. The driver also tries the per-user socket, or it would report "no Docker" on a machine plainly running it.
- Sleeping on idle is not here. Idle means no run or session active, no terminal or preview open and nobody looking, and every one of those signals arrives with the agent in step 3.
