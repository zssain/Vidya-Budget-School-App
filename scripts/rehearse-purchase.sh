#!/usr/bin/env bash
# Purchase → activation rehearsal, cross-stack and re-runnable
# (docs/implementation/runbooks/purchase-rehearsal.md).
#
#   app machine code (Rust) → checkout order + signed TEST webhook → billing worker (TS)
#   → REAL licence-signer binary (Rust, TEST key) → stored .vlic / key → app activation (Rust)
#   + the retry path: signer fails → TS retryFailedIssuance → Rust signer, same licence_id.
#
# Usage:
#   scripts/rehearse-purchase.sh                 # starts + stops its own throwaway Postgres 16
#   DATABASE_URL=postgres://postgres@127.0.0.1:5432/postgres scripts/rehearse-purchase.sh
#                                                # use an existing cluster (superuser URL; a fresh
#                                                # database is created and dropped)
# Env (optional): PG_BIN (Postgres 16 bin dir), REHEARSAL_PG_PORT (default 55433),
#   VIDYA_REHEARSAL_DIR (default: a new temp dir), KEEP_REHEARSAL_DIR=1 (keep artefacts).
#
# Fake schools, a TEST webhook secret and a throwaway TEST signing key only — no real
# accounts, no network. Never touches src-tauri/build-config.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DIR="${VIDYA_REHEARSAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/vidya-rehearsal.XXXXXX")}"
mkdir -p "$DIR"
DIR="$(cd "$DIR" && pwd)"
PORT="${REHEARSAL_PG_PORT:-55433}"
OWN_PG=""

log() { printf '\n==> %s\n' "$*"; }

cleanup() {
  local rc=$?
  if [[ -n "$OWN_PG" ]]; then
    "$PG_BIN/pg_ctl" -D "$OWN_PG" -m fast -w stop >/dev/null 2>&1 || true
    rm -rf "$OWN_PG"
  fi
  rm -f "$DIR/test-signing.key" # the throwaway TEST key never outlives the run
  if [[ "${KEEP_REHEARSAL_DIR:-}" != "1" && -z "${VIDYA_REHEARSAL_DIR:-}" ]]; then
    rm -rf "$DIR"
  else
    echo "artefacts kept in $DIR"
  fi
  if [[ $rc -eq 0 ]]; then echo "REHEARSAL PASSED"; else echo "REHEARSAL FAILED (exit $rc)"; fi
}
trap cleanup EXIT

# ---- Postgres ---------------------------------------------------------------
if [[ -n "${DATABASE_URL:-}" ]]; then
  ADMIN_URL="$DATABASE_URL"
  log "using existing Postgres (DATABASE_URL)"
else
  PG_BIN="${PG_BIN:-}"
  if [[ -z "$PG_BIN" ]]; then
    for c in /opt/homebrew/opt/postgresql@16/bin /usr/local/opt/postgresql@16/bin /usr/lib/postgresql/16/bin; do
      [[ -x "$c/initdb" ]] && PG_BIN="$c" && break
    done
  fi
  [[ -x "${PG_BIN:-}/initdb" ]] || { echo "Postgres 16 not found: set PG_BIN or DATABASE_URL" >&2; exit 2; }
  # Never talk to (or fight over) something else's server on that port.
  if "$PG_BIN/pg_isready" -q -h 127.0.0.1 -p "$PORT" || (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
    echo "port $PORT is already in use — set REHEARSAL_PG_PORT to a free port, or DATABASE_URL" >&2
    exit 2
  fi
  OWN_PG="$DIR/pgdata"
  log "starting throwaway Postgres 16 on 127.0.0.1:$PORT ($OWN_PG)"
  "$PG_BIN/initdb" -D "$OWN_PG" -U postgres --auth=trust -E UTF8 >/dev/null
  # No unix socket: scratch paths can exceed the 103-byte socket path limit.
  if ! "$PG_BIN/pg_ctl" -D "$OWN_PG" -l "$DIR/pg.log" -w \
    -o "-p $PORT -c listen_addresses=127.0.0.1 -c unix_socket_directories=''" start >/dev/null; then
    tail -5 "$DIR/pg.log" >&2
    exit 2
  fi
  ADMIN_URL="postgres://postgres@127.0.0.1:$PORT/postgres"
fi

# ---- builds -----------------------------------------------------------------
log "building the real licence-signer (services/licence-signer)"
cargo build --quiet --manifest-path services/licence-signer/Cargo.toml --bin licence-signer
SIGNER_BIN="$ROOT/services/licence-signer/target/debug/licence-signer"

for p in packages/billing-core services/billing-worker; do
  [[ -d "$p/node_modules" ]] || (cd "$p" && npm ci --no-audit --no-fund --silent)
done

# ---- 1. the buyer's computer: app-generated machine id + machine code ---------
log "phase 1: fake machines via the app's machine-id + machine-code functions"
VIDYA_REHEARSAL_DIR="$DIR" cargo test --quiet -p vidya --test purchase_activation -- \
  --ignored --exact rehearsal_1_fake_machines --nocapture

# ---- 2. purchase + signing + retry (TS worker + Rust signer binary) ------------
log "phase 2: checkout → webhook → billing worker → licence-signer (+ retry path)"
(cd services/billing-worker && \
  VIDYA_REHEARSAL_DIR="$DIR" LICENCE_SIGNER_BIN="$SIGNER_BIN" BILLING_TEST_ADMIN_URL="$ADMIN_URL" \
  npm run --silent rehearse:purchase)

# ---- 3. activation in the app ---------------------------------------------------
log "phase 3: activate the stored .vlic / key with the app's activation logic"
VIDYA_REHEARSAL_DIR="$DIR" cargo test --quiet -p vidya --test purchase_activation -- \
  --ignored --exact rehearsal_2_activate --nocapture
