# 0018: Web app on SvelteKit with adapter-node, pnpm pinned by corepack

- **Date:** 2026-09-29
- **Status:** Accepted

## Context

`docs/architecture.md` already chose TypeScript and SvelteKit for the web app, and the view component library lives there too. Step 0 has to turn that into a skeleton that builds, lints and tests, so the CI web job can switch on and the connection step has somewhere to put a browser terminal. The details left open were the adapter, the package manager, how strict the TypeScript settings are, and where the app reads its settings from.

## Decision

- **SvelteKit 2 with Svelte 5** (runes), and **`@sveltejs/adapter-node`**. The web app ships as a container like the Rust services (`docs/delivery.md`, "What gets built and shipped"), so it needs a plain Node server, not a platform adapter. `adapter-auto` would guess, and guessing a platform is exactly what we don't want while compute providers are still open.
- **pnpm, pinned by the `packageManager` field** in `web/package.json` and enabled with `corepack enable pnpm`. No separate install, and CI and a developer machine run the same pnpm version.
- **TypeScript in strict mode, pinned to `^6`.** TypeScript 7 is released, but SvelteKit's peer range is `^5.3.3 || ^6.0.0` and `typescript-eslint` requires `<6.1.0`. Revisit when both have moved.
- **ESLint (flat config) + Prettier + `svelte-check` + Vitest**, wired to `pnpm lint`, `pnpm check` and `pnpm test`, which is what `scripts/check.sh` and CI call.
- **pnpm's `minimumReleaseAge` guard stays on.** pnpm 12 refuses packages published in the last day or so, which is a cheap defence against a compromised release being installed before anyone notices. When it blocked `typescript-eslint@8.71.0` (published the day before), we took the previous release rather than writing the exclusion list pnpm offered. **Never add `minimumReleaseAgeExclude` to get an install moving** — pin to an older version instead.
- **The app reads the repository-root `.env`**, via `kit.env.dir: '..'` in `svelte.config.js`, so one file serves the Rust services and the web app (decision 0016, and `.env.example`).
- **Environment names are parsed the same way on both sides.** `web/src/lib/environment.ts` accepts the same four names and aliases as `Environment` in `crates/telemetry`, and throws on anything else rather than assuming `local`.

## Alternatives considered

- **`adapter-auto`:** less configuration, but it picks a target from the CI environment it happens to build in. We'd rather say what we deploy.
- **npm or Bun:** npm needs no setup but installs slower and its flat `node_modules` hides undeclared dependencies; Bun is one fast tool for install, scripts and tests, but is a second runtime to pin and a less-trodden path for SvelteKit and Playwright, and our servers are Rust so its runtime speed buys us nothing.
- **TypeScript 7:** the newest, but outside SvelteKit's and typescript-eslint's peer ranges today.
- **Excluding fresh packages from the release-age policy:** pnpm writes the exclusion list for you, which makes it the path of least resistance. It also silently removes the protection for exactly the package that is new enough to be worth checking.
- **Vite's `envDir` alone:** it only covers `import.meta.env`. `$env/dynamic/private` reads `kit.env.dir`, which is the setting that actually matters. Both are set, to the same place.
- **Duplicating the environment names only in the Rust crate:** the web app would have had to hardcode strings, and the two would drift the first time a name changed.

## Consequences

- `corepack enable pnpm` is a one-time step on a new machine, and belongs in the README.
- The whole web toolchain is pinned in one lockfile, committed, and CI installs with `--frozen-lockfile`, so a dependency cannot change under us without a visible diff.
- Two definitions of the environment names now exist, in Rust and TypeScript. They are small and tested on both sides; if more shared types appear (they will, with the protocol crate in the connection step), generating the TypeScript from Rust is the answer rather than a third hand-written copy.
- Upgrading TypeScript is now gated on SvelteKit's peer range, which is worth rechecking when either moves.
