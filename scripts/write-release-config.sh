#!/usr/bin/env bash
# Write src-tauri/build-config/release.json from the five required build-config
# VALUES, failing with a clear message that NAMES any missing one (prompts/P09 §8).
# Called by .github/workflows/release.yml with these as env (from repo Variables):
#   LICENCE_API, LICENCE_PUBLIC_KEY, RELAY_URL,
#   GOOGLE_CLIENT_ID_DESKTOP, GOOGLE_CLIENT_ID_ANDROID
# All five are NON-secret (public URLs, the licence PUBLIC key, OAuth client IDs);
# the licence PRIVATE key lives only on the licence server as a Fly secret and is
# never part of the app build. build.rs is the hard backstop for empties + dev key.
set -euo pipefail

missing=""
for k in LICENCE_API LICENCE_PUBLIC_KEY RELAY_URL GOOGLE_CLIENT_ID_DESKTOP GOOGLE_CLIENT_ID_ANDROID; do
  if [ -z "${!k:-}" ]; then missing="$missing $k"; fi
done
if [ -n "$missing" ]; then
  echo "::error::Missing required build-config value(s):$missing — add them as repo Variables (Settings → Secrets and variables → Actions → Variables). See docs/RELEASE-CHECKLIST.md." >&2
  exit 1
fi

# Defensively refuse the dev licence key (build.rs is the hard backstop).
DEV_KEY="yLQ8lt26cM/ZdKnfaYGS/VgV6DT6CrAyLHS1br28XJs="
if [ "$LICENCE_PUBLIC_KEY" = "$DEV_KEY" ]; then
  echo "::error::LICENCE_PUBLIC_KEY is the DEV key — generate a production key (cloud/licence gen-prod-key; see docs/DEPLOY-FLY.md) before releasing." >&2
  exit 1
fi

mkdir -p src-tauri/build-config
cat > src-tauri/build-config/release.json <<JSON
{
  "licence_api": "${LICENCE_API}",
  "licence_public_key": "${LICENCE_PUBLIC_KEY}",
  "relay_url": "${RELAY_URL}",
  "google_client_id_desktop": "${GOOGLE_CLIENT_ID_DESKTOP}",
  "google_client_id_android": "${GOOGLE_CLIENT_ID_ANDROID}"
}
JSON
echo "wrote src-tauri/build-config/release.json (licence_api=${LICENCE_API}, relay_url=${RELAY_URL})"
