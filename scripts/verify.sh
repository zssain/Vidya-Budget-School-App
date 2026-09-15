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

step "1/9  cargo fmt --check"
cargo fmt --all -- --check

step "2/9  cargo clippy (-D warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "3/9  cargo test --workspace"
cargo test --workspace

step "4/9  eslint"
npm run lint

step "5/9  prettier --check"
npm run format:check

step "6/9  vitest"
npm test

step "7/9  API drift"
node scripts/check-api-drift.mjs

step "8/9  i18n"
node scripts/check-i18n.mjs

if [ "${VERIFY_DENY:-0}" = "1" ]; then
  step "9/9  cargo deny"
  cargo deny check licenses bans sources
else
  step "9/9  cargo deny (skipped; set VERIFY_DENY=1 to run)"
fi

echo
echo "ALL CHECKS PASSED"
