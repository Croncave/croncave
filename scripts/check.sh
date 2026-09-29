#!/usr/bin/env bash
# Every check CI runs, in the order CI runs them. CI calls this same script,
# so what passes here passes there.
#
#   scripts/check.sh          everything that exists in the repository
#   scripts/check.sh rust     the Rust workspace only
#   scripts/check.sh web      the web app only
#
# Parts that don't exist yet are skipped, so this stays usable as the
# repository grows (see the build order in AGENTS.md).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target="${1:-all}"
case "$target" in
  all | rust | web) ;;
  *)
    echo "usage: scripts/check.sh [all|rust|web]" >&2
    exit 2
    ;;
esac

say() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
skip() { printf '\n--- skipped: %s\n' "$1"; }

check_rust() {
  if [ ! -f Cargo.toml ]; then
    skip "no Cargo.toml yet"
    return
  fi

  say "cargo fmt"
  cargo fmt --all --check

  say "cargo clippy"
  cargo clippy --workspace --all-targets -- -D warnings

  say "cargo test"
  cargo test --workspace
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
}

case "$target" in
  all)
    check_rust
    check_web
    ;;
  rust) check_rust ;;
  web) check_web ;;
esac

printf '\n\033[1m==> all checks passed\033[0m\n'
