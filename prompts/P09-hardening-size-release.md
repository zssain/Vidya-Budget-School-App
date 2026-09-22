# PHASE 9 of 10 — HARDENING, PERFORMANCE, SIZE GATES, ACCESSIBILITY, RELEASE BUILDS

## ROLE
You are a senior release engineer and security reviewer shipping **Vidya Budget School
v1.0.0**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. **No new features.** Only fixes, tests, performance, security, packaging, docs.
4. Only context §13 dependencies. CI actions allowed: `actions/checkout`,
   `actions/setup-node`, `dtolnay/rust-toolchain`, `actions/setup-java`,
   `android-actions/setup-android`, `tauri-apps/tauri-action`, `actions/upload-artifact`,
   `softprops/action-gh-release`. Tool versions (Tauri CLI, NDK, JDK) come from lockfiles
   and phase notes — never guessed.
5. The mock wins; visual baselines must still match.
6. Missing signing secrets → build unsigned and SAY SO in release notes. Never fake it.
7. Never delete/weaken tests. Never edit an earlier phase's migration.
8. Run every command you mention; paste real output. Work on branch `rebuild/p09`.
9. Stop conditions are real. Finish with the handoff file.

## OBJECTIVE
Secure, reliable and fast on low-end hardware, fully tested, and one tag push producing
the Windows `.exe`, macOS `.dmg` and Android `.apk` files within the size limits.

## DONE MEANS
`git tag v1.0.0 && git push --tags` (by the owner) → a DRAFT GitHub Release with
`Vidya_1.0.0_x64-setup.exe`, `Vidya_1.0.0_universal.dmg` (or `_aarch64` + `_x64`),
`Vidya_1.0.0_arm64-v8a.apk`, `Vidya_1.0.0_armeabi-v7a.apk`, `SHA256SUMS.txt`; each ≤ 40 MB
download and ≤ 50 MB installed; clean-machine runs recorded; every check below passes.

## WORK

### 1. Known issues
Fix every known issue in phase notes 1–8, or get the owner's written approval to defer.

### 2. Security review (fix all findings)
- Role-matrix test: call EVERY Tauri command and EVERY server endpoint as every role and
  state (active/suspended/removed/revoked device), assert context §5.
- Tauri capabilities minimal (no shell, no broad fs, dialog/opener/deep-link only where
  used). CSP: `default-src 'self'; img-src 'self' data: asset: http://asset.localhost;
  style-src 'self' 'unsafe-inline'; font-src 'self'; connect-src ipc: http://ipc.localhost`
  (verify the exact Tauri 2 values for each platform in Tauri docs).
- Server + relay: hashed tokens, constant-time compare, rate/body limits, no debug
  endpoints in release, logs without tokens/keys/PINs/OAuth tokens/recovery keys/full
  mobiles (log-scan test).
- SQL only parameterised (grep for `format!` near SQL); deep-link payload ≤ 2 KB and
  schema-validated; OAuth state + PKCE verified; Drive bundles and relay frames rejected on
  AEAD failure; replay counters enforced.
- `seed_demo_school`, `#/__gallery`, `#/__mocks`, `design/runtime`, fixtures and the dev
  licence key are NOT in release artifacts (test: search the built frontend + binary
  strings).
- Android: `allowBackup=false`, cleartext traffic off, `FLAG_SECURE` on PIN and recovery
  screens, permissions only `INTERNET, ACCESS_NETWORK_STATE, ACCESS_WIFI_STATE,
  CHANGE_WIFI_MULTICAST_STATE` (+ `CAMERA` only if the scanner is kept).
- Windows installer: firewall rule for Vidya.exe on the PRIVATE profile via an NSIS hook,
  removed on uninstall. macOS: document the first-run local-network prompt.
- `cargo audit` and `npm audit --omit=dev` (run as tools; not dependencies) — fix or
  document each finding.

### 3. Reliability
Panic hook → rotated logs (`<app_data>/logs`, 14 days, ≤ 10 MB each) + calm "Restart
Vidya" screen · daily `integrity_check` + audit chain check → failure blocks writes and
guides to Recover · soak test: 3 clients × 10,000 ops with random route failures
(LAN/relay/Drive) → identical end state · upgrade from the previous build with data →
migrations run, data intact · power loss during a write (kill -9 in tests) → no corruption.

### 4. Performance budgets (measure on the lowest hardware; record numbers)
Desktop cold start to PIN ≤ 2.5 s · Android 2 GB ≤ 3 s · Principal Home with 1,500
students ≤ 300 ms · attendance tap ≤ 50 ms with 60 students · search ≤ 150 ms · 500 ops
LAN sync ≤ 5 s · idle memory desktop ≤ 250 MB, Android ≤ 180 MB.

### 5. Size (hard gates)
- Release profile from Phase 1 kept; `cargo tree -i aws-lc-rs` empty; tree-shaken JS;
  fonts subset to the weights and scripts actually used (use `pyftsubset` or Fontsource
  subsets as a BUILD TOOL if needed, not a dependency) — the Newsreader opsz axis must be
  kept.
- Android: split APKs per ABI, R8 minify + shrinkResources, native libs stored
  uncompressed-in-APK/not extracted, only `arm64-v8a` + `armeabi-v7a`.
- `scripts/size-report.mjs` now checks BOTH: download bytes ≤ 40,000,000 and installed
  bytes ≤ 50,000,000:
  - Windows: CI runs the NSIS installer silently into a temp folder and sums it.
  - macOS: sum of the `.app` bundle.
  - Android: APK size (+ note: with non-extracted native libs installed ≈ APK size; also
    measure on the emulator with `adb shell du` if root is available).
  - If the universal `.dmg` or `.app` exceeds a limit → switch to per-architecture DMGs
    and update context §2 + the download page text.
- Report the biggest contributors (`cargo bloat` as a tool, `vite build --report` or
  rollup visualizer ONLY as a temporary local tool, not committed).

### 6. Accessibility
Focus order and visible focus everywhere; mock `aria-*` kept; contrast ≥ 4.5:1 for body
text — check `#56657A` on `#F5F7F6`/`#FDFDFB`, gold text uses, `#9FACBF` on navy; REPORT
any mock colour that fails and ASK before changing (the mock wins unless the owner
decides); Android font scale 130% without broken layout; TalkBack reads P/A/L with student
names.

### 7. Final automated suite
Rust unit + integration + role matrix + soak (ignored by default, run nightly) · vitest
(format, i18n, router) · Playwright UI flows + visual snapshots of the five mock screens
(baseline = mock renderer, threshold 0.1%) · checks that FAIL on: en/hi key mismatch, any
hex not in tokens.css (`check-hex.mjs`), any dependency not in §13 (`check-deps.mjs` reads
package.json + `cargo metadata`), any size limit exceeded.

### 8. Packaging
- Version single-sourced from `package.json` → `Cargo.toml` + `tauri.conf.json` via
  `scripts/set-version.mjs` (plain Node).
- **Windows NSIS**: per-machine install, English/Hindi installer language, WebView2
  `embedBootstrapper` (never offline installer), shortcuts, firewall hook, uninstall keeps
  data unless "Remove school data" is ticked. Sign with signtool if
  `WINDOWS_CERTIFICATE` + `WINDOWS_CERTIFICATE_PASSWORD` exist; else unsigned (SmartScreen
  steps in INSTALL.md).
- **macOS**: universal (or per-arch), min 12.0, entitlements for network server + client
  and keychain; sign + notarize if `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` exist; else
  unsigned (right-click → Open steps).
- **Android**: split APKs, minSdk 24, targetSdk from the repo template, `versionCode =
  major*10000 + minor*100 + patch`; signing from `ANDROID_KEYSTORE_BASE64`,
  `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` via
  `keystore.properties` written by CI; no keystore → debug-signed `-UNSIGNED-TEST` APKs,
  clearly labelled. Optional `.aab`.
- Build config per environment from CI secrets/vars; release fails if any is missing.
- `cloud/relay` and `cloud/licence`: Dockerfiles (relay production-ready; licence per
  Phase 10).

### 9. CI
`ci.yml` on push/PR: tsc, vitest, cargo test, clippy, check scripts, debug builds + size
report. `release.yml` on tag `v*`: matrix windows-latest / macos-latest / ubuntu-latest
(Android: JDK 17 + SDK/NDK versions from phase notes) → npm ci → checks → tests → build →
installed-size measurement → rename → SHA-256 → DRAFT release with notes from CHANGELOG.md.

### 10. Docs
`INSTALL.md` (per platform incl. unsigned workarounds; choosing the server PC — on during
school hours, wired LAN preferred, sleep off; Wi-Fi without client isolation; Android
"Don't optimise battery" for Vidya; what is not counted in the size) ·
`ADMIN-GUIDE.md` for the Principal in plain language (activation, recovery key safekeeping,
inviting staff, Google Drive, approvals, conflicts, review flags, backups, moving to a new
PC, lost phone, new session, offline lease, the honest audit limitation) · `RELEASE.md`
(cutting a release, secrets list, local builds per OS, troubleshooting NDK / WebView2 /
notarization) · `CHANGELOG.md`.

## CLEAN-MACHINE RUNS BEFORE TAGGING (record date, device, result)
1. Fresh Windows 10 (4 GB RAM) → install → activate → setup → firewall rule present →
   invite.
2. Fresh macOS → join as accountant via relay → payment → confirmed.
3. Cheap Android (2 GB, armeabi-v7a if available) + a recent arm64 phone → join →
   attendance offline → sync via LAN, then mobile data, then with the server off (Drive).
4. Upgrade install over the previous build with data.
5. Uninstall/reinstall Windows without removing data → PIN works, data intact.
6. Restore on the Mac from the Windows server's Drive backup.

## EDGE CASES TO RECHECK
All lists from phases 2–8, plus Hindi Windows user paths (`C:\Users\प्रिया\`), paths with
spaces, antivirus quarantine (document), VPN/multiple adapters on the server, system date
far in the future, Android battery optimisation killing sync, phone storage < 100 MB (warn
before snapshot), Google token revoked mid-sync, relay certificate rotation.

## HANDOFF → `docs/phase-notes/phase-9.md`
Release checklist · sizes (download + installed) + checksums · signing status per platform
· measured performance · security findings + fixes · accessibility findings · remaining
issues (none or owner-approved) · exact commands to rebuild each binary locally.
