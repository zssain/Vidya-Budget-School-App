#!/usr/bin/env bash
# Write src-tauri/build-config/release.json from the required build-config VALUES,
# failing with a clear message that NAMES any missing one (prompts/P09 §8, P12 §6).
# Called by .github/workflows/release.yml with these as env (from repo Variables):
#   LICENCE_PUBLIC_KEY (required)
# Optional (empty → that feature is simply unavailable in the build; the app shows
# a clear "not set up in this build" state, never a fake success):
#   GOOGLE_CLIENT_ID_DESKTOP — desktop Google Drive backup. Empty → Drive off.
#   GOOGLE_CLIENT_ID_ANDROID — Android Google Drive sync. Empty → Drive off.
#   LICENCE_PUBLIC_KEY_PREV  — a previously used public key kept trusted after a
#                              signing-key rotation (tools/licence-maker README).
#   RELAY_URL                — only for the optional "Instant sync" add-on (§14).
# All are NON-secret (the licence PUBLIC key, OAuth client IDs, a public relay URL);
# the licence PRIVATE key lives ONLY in the owner's licence-maker key folder and is
# never part of the app build. There is no LICENCE_API in v2 (offline licences).
# build.rs is the hard backstop for empties + the dev key.
set -euo pipefail

# Only the licence public key is strictly required to build (build.rs enforces it);
# the OAuth client ids are optional add-ons and may be empty for a test build.
if [ -z "${LICENCE_PUBLIC_KEY:-}" ]; then
  echo "::error::Missing required build-config value: LICENCE_PUBLIC_KEY — add it as a repo Variable (Settings → Secrets and variables → Actions → Variables). Mint one with tools/licence-maker init. See docs/RELEASE-CHECKLIST.md." >&2
  exit 1
fi

# Defensively refuse the dev licence key (build.rs is the hard backstop).
DEV_KEY="yLQ8lt26cM/ZdKnfaYGS/VgV6DT6CrAyLHS1br28XJs="
if [ "$LICENCE_PUBLIC_KEY" = "$DEV_KEY" ]; then
  echo "::error::LICENCE_PUBLIC_KEY is the DEV key — mint a production keypair (tools/licence-maker init) before releasing." >&2
  exit 1
fi

# Build the licence_public_keys JSON array: the current key, plus an optional
# previous key kept trusted across a rotation.
KEYS_JSON="\"${LICENCE_PUBLIC_KEY}\""
if [ -n "${LICENCE_PUBLIC_KEY_PREV:-}" ]; then
  KEYS_JSON="${KEYS_JSON}, \"${LICENCE_PUBLIC_KEY_PREV}\""
fi

mkdir -p src-tauri/build-config
cat > src-tauri/build-config/release.json <<JSON
{
  "licence_public_keys": [${KEYS_JSON}],
  "relay_url": "${RELAY_URL:-}",
  "google_client_id_desktop": "${GOOGLE_CLIENT_ID_DESKTOP:-}",
  "google_client_id_android": "${GOOGLE_CLIENT_ID_ANDROID:-}"
}
JSON
echo "wrote src-tauri/build-config/release.json (licence_public_keys count=$([ -n "${LICENCE_PUBLIC_KEY_PREV:-}" ] && echo 2 || echo 1), relay_url=${RELAY_URL:-<none>})"
