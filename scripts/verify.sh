#!/usr/bin/env bash
# Everything that must pass before a task is considered done.
# Run with `npm run verify`. Set VERIFY_DENY=1 to also run cargo-deny.
set -euo pipefail

step() {
  echo
  echo "=================================================="
  echo "  $1"
  echo "=================================================="
}

step "1/10  cargo fmt --check"
cargo fmt --all -- --check

step "2/10  cargo clippy (-D warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "3/10  cargo test --workspace"
cargo test --workspace

step "4/10  eslint"
npm run lint

step "5/10  prettier --check"
npm run format:check

step "6/10  vitest"
npm test

step "7/10  API drift"
node scripts/check-api-drift.mjs

step "8/10  i18n"
node scripts/check-i18n.mjs

if [ "${VERIFY_DENY:-0}" = "1" ]; then
  step "9/10  cargo deny"
  cargo deny check licenses bans sources
else
  step "9/10  cargo deny (skipped; set VERIFY_DENY=1 to run)"
fi

if [ "${VERIFY_SIZE:-0}" = "1" ]; then
  step "10/10 size gate"
  node scripts/check-size.mjs
else
  step "10/10 size gate (skipped; set VERIFY_SIZE=1 to run)"
fi

echo
echo "ALL CHECKS PASSED"
