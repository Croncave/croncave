# 0019: One build order, following architecture.md, and one owner per doc

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

Planning step 1 stalled on a contradiction. `AGENTS.md` said step 1 was "a browser terminal works on a local (Docker) workspace." `docs/architecture.md` said step 1 was a skeleton — sign-in, teams, Postgres, a SvelteKit shell — and put the connection third. Both were checked in, and neither said which to follow.

Auditing the two against `docs/product-definition.md` turned up more than an ordering disagreement:

- **The build order existed three times**: in `AGENTS.md`, in `docs/delivery.md` and in `docs/architecture.md`. The two repository copies were meant to be identical and **7 of their 9 rows had already drifted in wording** — harmlessly so far, which is how duplication rots.
- **The repository's order was missing work the product marks "Must, R1."** Sign-in, teams and Postgres had no step at all. Starter templates and the first dashboard components had been dropped from the extras step.
- **`AGENTS.md`'s step 1 assumed things it never stated**: that a local compute driver existed and that a workspace record existed, because a terminal must attach to something and the relay must authorise the browser against something. That unstated assumption is what made the step unplannable.
- **The repository layout was duplicated the same way**, identical only because step 0 edited both copies by hand.

Underneath all of it is a question nobody had answered in writing: which docs may be changed here, and which are snapshots of living docs that only the account owner can change?

## Decision

- **`docs/architecture.md`'s build order wins**, and the repository's table is renumbered to match it: accounts, compute, connection, sessions, GitHub, scheduled runs, home and timeline, extras, hardening. A step 0 is kept for the repository setup that list doesn't cover. Only step 0 had shipped, so renumbering cost nothing.
- **`docs/delivery.md` owns the repository layout and the build order.** `AGENTS.md` summarises both and links there, the way it already treats conventions. Neither is copied.
- **Doc ownership is written down** in `AGENTS.md` and `docs/conventions.md`: `product-definition.md`, `architecture.md` and `pricing.md` are read-only snapshots; `AGENTS.md`, `delivery.md`, `conventions.md` and `decisions/` are repo-native. A decision made while building is recorded in a repo-native doc, and the gap it leaves in a snapshot is reported to the user as exact upstream wording.

## Alternatives considered

- **Keep the repository's ordering and change the living doc to match.** The repository's order reached the architecture-proving milestone — a cloud computer reachable only through Croncave — sooner, which is worth something. But it got there by leaving out an R1 commitment, and the skeleton it skipped is what the connection needs underneath it. The living doc was right.
- **Label the two orderings as "strategic" and "execution" and keep both.** Least work, and it would have papered over exactly the confusion that caused this entry.
- **Make `AGENTS.md` canonical instead of `delivery.md`.** Fewer hops for an agent, which reads `AGENTS.md` in full anyway. Rejected because `AGENTS.md` would then grow with every build step, and its own table of contents would stop matching the files it describes.
- **Sync the wording and keep both copies.** Fixes today's drift and guarantees tomorrow's.

## Consequences

- **"Step 1" now means accounts and workspaces.** The agent-and-relay work is step 3. Anything written before 2026-09-29 that refers to step numbers means the old scheme.
- Adding or changing a step edits one file.
- `docs/architecture.md` still doesn't know what step 0 decided: its stack table has no observability row, and nothing there mentions `tracing`, Sentry, `rustls`, `adapter-node`, pnpm or `crates/telemetry`. Decisions 0016–0018 live only here. The exact upstream wording has been given to the account owner; until it is applied, the snapshot lags the repository — in the direction the repository has just been corrected *to*, which is the safe direction, but still a gap.
- Snapshots will keep falling behind as we build, because that is what snapshots do. The rule that makes it survivable is that the gap gets reported rather than silently patched into the snapshot, where the next export would overwrite it.
