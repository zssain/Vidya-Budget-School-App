#!/usr/bin/env node
// CI-ONLY build config (P21 / G4). The macOS + Windows CI jobs build a real release
// bundle to enforce the size gate, and build.rs (correctly) refuses a release build
// without build-config/release.json. This writes one with a THROWAWAY random licence
// public key: nobody holds its private key, so no licence can ever verify against a
// CI build, and the dev key is never used. CI artifacts are not publishable releases —
// real releases come only from .github/workflows/release.yml (repo Variables) and
// keep every production check in build.rs. Refuses to overwrite an existing file.
import { existsSync, writeFileSync } from 'node:fs'
import { randomBytes } from 'node:crypto'

const path = 'src-tauri/build-config/release.json'
if (existsSync(path)) {
  console.error(`write-ci-config: ${path} already exists — refusing to overwrite`)
  process.exit(1)
}
const cfg = {
  licence_public_keys: [randomBytes(32).toString('base64')],
  relay_url: '',
  google_client_id_desktop: '',
  google_client_id_android: '',
  pwa_origin: '',
}
writeFileSync(path, JSON.stringify(cfg, null, 2) + '\n')
console.log('write-ci-config: wrote CI-only release.json (throwaway licence key; NOT a release)')
