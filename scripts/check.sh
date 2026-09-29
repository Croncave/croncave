#!/usr/bin/env bash
# Every check CI runs, in the order CI runs them. CI calls this same script,
# so what passes here passes there.
#
#   scripts/check.sh          everything that exists in the repository
#   scripts/check.sh rust     the Rust workspace only
#   scripts/check.sh web      the web app only
#   scripts/check.sh e2e      the end-to-end tests only
#
# Parts that don't exist yet are skipped, so this stays usable as the
# repository grows (see the build order in AGENTS.md).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target="${1:-all}"
case "$target" in
  all | rust | web | e2e) ;;
  *)
    echo "usage: scripts/check.sh [all|rust|web|e2e]" >&2
    exit 2
    ;;
esac

say() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
skip() { printf '\n--- skipped: %s\n' "$1"; }

# Take DATABASE_URL from .env when the shell doesn't already have one, so
# `./scripts/check.sh` works straight after `cp .env.example .env`. An
# exported value always wins, which is how CI sets it.
if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  DATABASE_URL="$(sed -n 's/^DATABASE_URL=//p' .env | tail -1)"
  export DATABASE_URL
fi

check_rust() {
  if [ ! -f Cargo.toml ]; then
    skip "no Cargo.toml yet"
    return
  fi

  say "cargo fmt"
  cargo fmt --all --check

  say "cargo clippy"
  cargo clippy --workspace --all-targets -- -D warnings

  # The database tests need a real Postgres. Skip them loudly rather than
  # failing, so the checks stay runnable before `docker compose up -d`.
  # CI always sets DATABASE_URL, so CI never takes this branch.
  if [ -z "${DATABASE_URL:-}" ] && [ -n "${CI:-}" ]; then
    echo "DATABASE_URL is not set, but this is CI. The database tests must run here." >&2
    exit 1
  fi

  if [ -n "${DATABASE_URL:-}" ]; then
    say "cargo test"
    cargo test --workspace
  else
    say "cargo test (without the database)"
    cargo test --workspace --exclude croncave-db
    printf '\n\033[33m!!! skipped the database tests: DATABASE_URL is not set.\033[0m\n'
    printf '    Run: docker compose up -d, and set DATABASE_URL (see .env.example)\n'
  fi
}

check_web() {
  if [ ! -f web/package.json ]; then
    skip "no web/package.json yet"
    return
  fi

  if ! command -v pnpm >/dev/null 2>&1; then
    echo "pnpm is not available. Run: corepack enable pnpm" >&2
    exit 1
  fi

  if [ ! -d web/node_modules ]; then
    say "pnpm install"
    (cd web && pnpm install)
  fi

  say "pnpm lint"
  (cd web && pnpm run --if-present lint)

  say "pnpm check"
  (cd web && pnpm run --if-present check)

  say "pnpm test"
  (cd web && pnpm run --if-present test)

  # A build that fails, or quietly starts fetching fonts from a third party,
  # is only visible once everything is bundled together.
  say "pnpm build"
  (cd web && pnpm run --if-present build >/dev/null)

  say "built assets"
  (cd web && pnpm run --if-present check:assets)
}

check_e2e() {
  if [ ! -f e2e/package.json ]; then
    skip "no e2e/package.json yet"
    return
  fi

  if ! command -v pnpm >/dev/null 2>&1; then
    echo "pnpm is not available. Run: corepack enable pnpm" >&2
    exit 1
  fi

  if [ ! -d e2e/node_modules ]; then
    say "pnpm install (e2e)"
    (cd e2e && pnpm install)
  fi

  # The end-to-end tests drive a real browser against a real control plane and
  # a real Postgres. Skip them loudly when either is missing, the same way the
  # database tests do; in CI a skip is a failure.
  if [ -z "${DATABASE_URL:-}" ]; then
    skip "end-to-end tests: no DATABASE_URL"
    [ -n "${CI:-}" ] && { echo "but this is CI, where they must run." >&2; exit 1; }
    return
  fi

  if ! (cd e2e && pnpm exec playwright install --dry-run chromium >/dev/null 2>&1); then
    skip "end-to-end tests: no browser. Run: cd e2e && pnpm run install-browsers"
    [ -n "${CI:-}" ] && { echo "but this is CI, where they must run." >&2; exit 1; }
    return
  fi

  # They drive the built binary, not cargo run, so a stale one would test the
  # wrong code.
  say "cargo build (for the end-to-end tests)"
  cargo build -p croncave-control-plane

  say "playwright"
  (cd e2e && pnpm run test)
}

case "$target" in
  all)
    check_rust
    check_web
    check_e2e
    ;;
  rust) check_rust ;;
  web) check_web ;;
  e2e) check_e2e ;;
esac

printf '\n\033[1m==> all checks passed\033[0m\n'
