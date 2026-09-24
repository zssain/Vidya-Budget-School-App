# Phase 9 — Hardening, performance, size gates, accessibility, release builds

## Environment (Standing Rule 2)

- **Branch:** `rebuild/p09` (branched from `c15bc0d`, the P08 Part F HEAD).
- **HEAD at start:** `c15bc0d904c7ad30fec8c89719916b2c16e4229d`
- **git status at start:** clean.
- **Tool versions:** Node v25.3.0 · npm 11.7.0 · rustc 1.98.1 · cargo 1.98.1 ·
  tauri-cli 2.11.5 · Python 3.14.6. **No JDK** (Android/APK builds cannot run in
  this environment) · **no Windows toolchain** · macOS host (darwin 25.2) ·
  Docker daemon not running · no signing secrets · `pyftsubset` not installed.
- **Consequence:** all source/config/test/docs work was done and verified here;
  everything that needs a device, a cross-toolchain, Docker, hosting, or a signing
  secret is **authored and documented**, not executed — see *Remaining / owner*.

---

## Summary

P09 was a hardening/release pass, not new features. The headline result is a
real **SQL-injection fix in the sync engine**, a set of **security/size/quality
gates** wired into `npm run verify` and CI, **reliability** infrastructure
(rotated logs + panic hook + integrity gate + a power-loss test), **role-matrix**
proof of §5, and complete **release packaging + CI/CD + docs**. Two pre-existing
issues were fixed (a wall-clock-flaky e2e test; a dev-route string leak in the
production bundle). The desktop size budget remains comfortable (P1: DMG 5.58 MB,
`.app` 9.86 MB — 7×/5× under the limits).

All local gates are green: `npm run verify`, `cargo clippy -D warnings`, the Rust
workspace test suite, `cargo audit` (0 vulns), `npm audit` (0 vulns).

---

## Release checklist

| Item | Status |
|---|---|
| Version single-sourced (`package.json` → Cargo/tauri/Android) | ✅ `scripts/set-version.mjs`; repo bumped to **1.0.0**, Android `versionCode 10000` |
| CSP set to the exact §2 string | ✅ `tauri.conf.json` |
| Tauri capabilities minimal (dialog/opener only where used) | ✅ `capabilities/default.json` |
| SQL only parameterised (identifier injection closed) | ✅ sync `is_safe_ident` gate + tests |
| Dev artifacts absent from the built frontend | ✅ fixed `__gallery`/`__mocks` leak; `check-release-clean.mjs` gate |
| Android hardening (perms, allowBackup, cleartext, non-extracted libs) | ✅ manifest + gradle |
| Rotated logs + panic hook + crash marker | ✅ `reliability.rs` (+ tests) |
| Integrity gate (`PRAGMA integrity_check` + audit chain) | ✅ `reliability::integrity_gate` |
| Power-loss / kill-9 durability | ✅ `tests/durability.rs` |
| Role-matrix over §5 | ✅ `crates/vidya-core/tests/role_matrix.rs` |
| Dependency gate vs §13 | ✅ `check-deps.mjs` |
| Size gate (download ≤ 40 MB **and** installed ≤ 50 MB) | ✅ `size-report.mjs` |
| Contrast gate (≥ 4.5:1 body) | ✅ `check-contrast.mjs` (all pass) |
| `cargo audit` / `npm audit` | ✅ 0 vulnerabilities each |
| CI (`ci.yml`) fixed + release pipeline (`release.yml`) | ✅ only §4 allow-listed actions |
| NSIS firewall hook + per-machine + EN/HI + macOS entitlements | ✅ `installer-hooks.nsh`, `entitlements.plist` |
| Docs (INSTALL / ADMIN-GUIDE / RELEASE / CHANGELOG) | ✅ at repo root |
| Signed artifacts + clean-machine runs + Android/Windows builds | ⛔ needs secrets/hardware — see below |

---

## Security findings + fixes

### 1. SQL identifier injection in the sync engine (HIGH — fixed)
`sync/engine.rs::upsert_canonical`, `sync/apply.rs` (`current_version`,
`row_field_differs`, `record_conflict`) and the pull `DELETE` built SQL with the
**table name and column names taken straight from network payloads** (peer-device
ops / server pull changes) via `format!` — only the *values* were parameterised.
Identifiers can't be bound as parameters, so a crafted op could inject SQL through
the table/column position.

**Fix:** `sync::protocol::is_safe_ident` / `op_identifiers_safe` — a fail-closed
strict-identifier check (`^[A-Za-z_][A-Za-z0-9_]{0,63}$`). `apply_op` now rejects
any op whose table or a payload column isn't a plain identifier (`MALFORMED`)
before any SQL is built; the client pull path skips such rows. A value containing
a quote/semicolon/paren/space cannot pass, which closes the vector. Unit tests in
`protocol.rs`; all 26 sync/relay/drive e2e tests still pass (no legitimate op
uses a non-identifier). `scope.rs` was reviewed — its dynamic table names are
hard-coded string literals, not network input (safe).

### 2. CSP was `null` (fixed)
Set to the exact §2 value:
`default-src 'self'; img-src 'self' data: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; font-src 'self'; connect-src ipc: http://ipc.localhost`.

### 3. Capabilities were over/under-scoped (fixed)
`core:default` alone would have **denied** the frontend's real `dialog.save`,
`dialog.open` and `opener.openUrl` calls (`src/lib/files.ts`) at runtime. Scoped
to exactly `dialog:allow-open`, `dialog:allow-save`, `opener:allow-open-url`.
Deep-link is handled only in Rust, so it needs no frontend permission. No shell,
no fs, no broad plugin defaults.

### 4. Dev artifacts leaking into the release bundle (fixed)
A production `vite build` shipped the `'/__gallery'` and `'/__mocks'` route
strings because `isDevRoute` in `App.tsx` was computed **outside** the
`import.meta.env.DEV` guard. Guarded it so Vite dead-code-eliminates the strings.
`seed_demo_school` (`#[cfg(debug_assertions)]`), the Gallery/Mocks components,
fixtures and the dev licence key were already absent. `scripts/check-release-clean.mjs`
now builds and scans the frontend for all of these and fails on any hit.

### 5. Android hardening (fixed)
`AndroidManifest.xml`: added `ACCESS_NETWORK_STATE`, `ACCESS_WIFI_STATE`,
`CHANGE_WIFI_MULTICAST_STATE` (INTERNET already present; **no CAMERA** — the QR
scanner was deferred), `android:allowBackup="false"`, `android:extractNativeLibs="false"`,
cleartext off in release (`usesCleartextTraffic=false`). `build.gradle.kts`:
`useLegacyPackaging = false` so native libs are stored **uncompressed and not
extracted** (installed ≈ APK, per §5) — this replaced the previous compressed
setting. Split APKs per ABI + R8 minify + `shrinkResources` + `arm64-v8a`/`armeabi-v7a`
were already correct.

### 6. Logs (verified clean)
`scripts/check-logs.mjs` scans every Rust source for logging macros that
interpolate a secret-bearing identifier (token/key/PIN/OAuth/recovery/full
mobile). **0 findings** — the codebase already redacts.

### 7. Supply-chain audits
- `npm audit --omit=dev` → **0 vulnerabilities**.
- `cargo audit` → **0 vulnerabilities** across 601 crates; **7 warnings**, all
  transitive and non-blocking: `unic-common` / `unic-ucd-ident` / `unic-ucd-version`
  (unmaintained) come from `urlpattern` ← `tauri-utils` ← **`tauri-build`** (a
  build-only dep, not in the shipped binary); `glib 0.18.5` (unsound
  `RUSTSEC-2024-0429`) is the **Linux** GTK/WebKit webview — Vidya ships on
  Windows/macOS/Android, not Linux. No fixable direct action; documented.

### 8. Role matrix (§5) — `crates/vidya-core/tests/role_matrix.rs`
`permissions::can` is the single decision point (`require_allow` on desktop,
`apply_op` on the server both call it). The test encodes §5 as exhaustive
invariants over all 40 Actions × 3 roles × 4 states: non-active (invited/
suspended/removed) is denied **everything**; active Principal may do everything;
the six "Everyone" actions are allowed for all active roles; Teacher is denied all
fees/staff/admin/admissions; Accountant is denied all attendance/marks/staff/admin;
Teacher scope (own class attendance, own subject marks, own class guardian
address) is enforced. 7/7 pass. (`apply_op` additionally flags a non-active
author as `revoked_author` before permission — verified in `sync_e2e`.)

---

## Reliability (`src-tauri/src/reliability.rs`, wired in `lib.rs`)

- **Rotated file logs** at `<app_data>/logs/vidya-YYYY-MM-DD.log`, one per UTC
  day, pruned to **14 days** and **≤ 10 MB** each on startup (`prune`, unit-tested;
  no `tracing-appender` — not on §13, rotation is plain `std::fs`). A `MakeWriter`
  routes `tracing` INFO+ to today's file, reopening on the day boundary.
- **Panic hook** writes the panic (location + message) to the log and drops a
  `last-crash` marker; `take_last_crash()` is a one-shot the frontend reads on
  startup to show the calm "Restart Vidya" screen. Release uses `panic = "abort"`
  (Cargo.toml), so the panic is captured once and the calm screen appears next launch.
- **Integrity gate** `integrity_gate(conn)` runs `PRAGMA integrity_check` + the
  audit-chain check (`security::audit::verify_chain`) and returns `Ok` /
  `DbCorrupt` / `AuditBroken(seq)`. Failure must block writes and route to Recover.
- **Power-loss / kill-9** `tests/durability.rs`: a write abandoned before COMMIT
  (via `mem::forget`, the in-process abrupt kill) leaves the committed row intact,
  no half-written row, and `PRAGMA integrity_check = ok` on reopen (WAL + SQLCipher
  atomicity). 2/2 pass.
- **Soak:** the 3-client × 10,000-op random-route-failure convergence is
  substantially covered by the existing `sync_e2e` (5,000 queued ops after an
  outage, idempotency by `op_id`, conflict handling → identical end state). A
  dedicated 30k-op soak harness is listed under *Remaining* (nightly, `#[ignore]`).

---

## Performance (measured here; dev hardware, not the low-end target)

`cargo test -p vidya --test benchmark -- --ignored` at 1,500 students / 60 classes
/ 20 days:

| Aggregate | Time | Budget |
|---|---|---|
| active students | 0.23 ms | — |
| collected today | 0.26 ms | — |
| attendance today by mark | 17.54 ms | — |
| attendance by class today | 1.10 ms | — |
| fee collection last 6 days | 0.95 ms | — |
| **Principal Home total** | **~20 ms** | **≤ 300 ms** ✅ |

The remaining budgets (desktop cold-start ≤ 2.5 s, Android 2 GB ≤ 3 s, attendance
tap ≤ 50 ms, search ≤ 150 ms, 500-op LAN sync ≤ 5 s, idle memory) need a running
app / device and are listed under *Remaining*.

---

## Size

- Release profile from P1 kept (`lto`, `opt-level="s"`, `strip`, `panic="abort"`).
- `cargo tree -i aws-lc-rs` → empty (ring is the sole provider); asserted in CI.
- `scripts/size-report.mjs` now enforces **both** gates and writes `size-report.json`:
  download ≤ 40,000,000 and installed ≤ 50,000,000 bytes. macOS installed = summed
  `.app`; Android installed ≈ APK (non-extracted libs); Windows installed = the CI
  `pwsh` step that silent-installs and sums (`scripts/.windows-installed-bytes`).
- Frontend bundle: `index.js` 454 kB (108.7 kB gzip), CSS 17 kB. Biggest assets are
  the Newsreader variable-font woff2 files (132 + 147 kB) — the `opsz` axis is kept
  as required. **Font subsetting** (`pyftsubset`) is *not* applied (tool not present
  here, and the total is far under budget); it's an optional future optimisation,
  documented, not gate-blocking.

---

## Accessibility

`scripts/check-contrast.mjs` reads `tokens.css` and checks every §6-flagged pair.
**All pass ≥ 4.5:1** — no mock colour fails, so nothing needs the owner's decision:

- `--muted #56657a` on `#f5f7f6`/`#fdfdfb`/`#ffffff` → 5.52 / 5.83 / 5.94
- `--gold-text #8c6a2f` on surface/bg → 4.89 / 4.63
- `--on-navy-muted #9facbf` on navy → 7.42
- status pills (`part-paid`/`unpaid`/`marks`) → 6.27 / 6.61 / 7.61
- `--ink` on bg → 14.57; on-navy text → 11–13.

Focus order / visible focus, Android 130% font scale, and TalkBack reading P/A/L
with student names need a running app/device to verify (mock `aria-*` are
preserved) — listed under *Remaining*.

---

## Packaging

- **Version** single-sourced by `scripts/set-version.mjs` (`--check` gate in CI);
  Android `versionCode = major*10000+minor*100+patch` (1.0.0 → 10000, correcting
  the previous `1000`).
- **Windows NSIS:** `installMode: perMachine`, `languages: [English, Hindi]`,
  `webviewInstallMode: embedBootstrapper`, and `installerHooks: installer-hooks.nsh`
  which adds a firewall rule for `Vidya.exe` on the **private** profile on install
  and removes it on uninstall. Signing via `TAURI_SIGNING_*` if the cert secrets
  exist, else unsigned + `-UNSIGNED` suffix.
- **macOS:** `entitlements.plist` (network server + client for the LAN server /
  relay / Drive; keychain works via the login keychain, no sandbox), min 12.0,
  universal target in `release.yml`. Sign+notarize if the Apple secrets exist.
- **Android:** split APKs, minSdk 24, keystore.properties written from secrets in
  CI, `-UNSIGNED-TEST` suffix when absent.
- `cloud/relay` Dockerfile already production-shaped (distroless); `cloud/licence`
  is Phase-10.

---

## Automated suite / gates

`npm run verify` = `typecheck` + `check:hex` + `check:i18n` + `check:deps` +
`check:version` + `check:contrast` + `check:logs` + `vitest`. New scripts:
`check-deps.mjs`, `set-version.mjs`, `check-contrast.mjs`, `check-logs.mjs`,
`check-release-clean.mjs`; `size-report.mjs` upgraded. New Rust tests:
`role_matrix.rs`, `durability.rs`, `reliability` unit tests, `protocol` identifier
tests. The previously-broken CI (`npm run verify` and `scripts/check-size.mjs`
missing; a non-existent `vidya-db` package; non-allow-listed `Swatinem/rust-cache`
and `taiki-e/install-action`) was rewritten.

**`check-deps.mjs` warns (does not fail) on two §13 exceptions for the owner to
fold into §13:** `futures-util` (the Sink/Stream companion `tokio-tungstenite`
needs for the relay tunnel — already flagged in phase-5) and `@types/node` (a
types-only dev dependency needed by the Node build scripts / `vite.config.ts`).

---

## Rebuild each binary locally (exact commands)

```bash
# 0. one-time: sync the version everywhere
node scripts/set-version.mjs 1.0.0

# macOS universal DMG (host: macOS)
npm ci
npm run tauri build -- --target universal-apple-darwin --bundles dmg app
node scripts/size-report.mjs

# Windows NSIS installer (host: Windows, VS Build Tools + WebView2)
npm ci
npm run tauri build -- --bundles nsis
node scripts/size-report.mjs

# Android split APKs (JDK 17 + Android SDK/NDK; rustup targets
#   aarch64-linux-android, armv7-linux-androideabi)
npm ci
npm run tauri android build -- --apk --split-per-abi
node scripts/size-report.mjs
```

CI/CD: `.github/workflows/ci.yml` on push/PR; `.github/workflows/release.yml` on
tag `v*` builds every platform, renames to the §2 names, measures installed size,
writes `SHA256SUMS.txt` (via the `gh` CLI, since `download-artifact` is not on the
§4 allow-list) and opens a **DRAFT** release.

---

## Signing status per platform

**All three platforms are UNSIGNED by default** (owner decision #11). The release
pipeline signs only when the matching secrets exist and otherwise ships unsigned
with an explicit `-UNSIGNED` / `-UNSIGNED-TEST` filename suffix and a note — it
never fakes a signature (rule 6). Windows unsigned → SmartScreen "More info → Run
anyway" (INSTALL.md); macOS unsigned → right-click → Open (INSTALL.md); Android →
debug-signed test APKs.

Secrets to supply for signed builds: Windows `WINDOWS_CERTIFICATE(_PASSWORD)`;
macOS `APPLE_CERTIFICATE(_PASSWORD)`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
`APPLE_PASSWORD`, `APPLE_TEAM_ID`; Android `ANDROID_KEYSTORE_BASE64`,
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.

---

## Sizes + checksums

Not produced in this environment — building signed release artifacts and computing
`SHA256SUMS.txt` requires the per-OS toolchains + secrets and runs in `release.yml`
on tag push. P1 desktop measurements (release, universal target) remain the best
current signal: DMG 5.58 MB, `.app` 9.86 MB — both far under the 40/50 MB gates.
Windows NSIS and Android APK sizes are measured by the size job on the first
release build.

---

## Remaining / owner decisions

**Cannot run in this environment (needs hardware / secrets / hosting):**
- Signed Windows `.exe`, macOS `.dmg` (+notarize) and Android `.apk` builds; the
  6 clean-machine runs; the upgrade-over-previous-build and restore-on-Mac runs.
- Device performance numbers (cold start, attendance tap, search, LAN-sync, memory)
  and the Android 130% font-scale + TalkBack checks.
- `docker build cloud/relay` (daemon down) and relay deployment (**hosting
  undecided — STOP**, unchanged from P05).

**Deferred UI/runtime wiring on top of the tested Rust core (recommend owner
approval to keep as follow-ups, since P09 forbids new features):**
- The calm "Restart Vidya" React screen consuming `take_last_crash()`; the daily
  `integrity_gate` scheduler + app-wide write-block on failure.
- The dedicated 3×10,000-op soak test (nightly, `#[ignore]`).
- NSIS "Remove school data" opt-in uninstall checkbox (default already keeps data).
- Font subsetting via `pyftsubset` (optional; not gate-blocking).
- The larger backlog carried from phases 5–8 (fenced-screen UI, public-address
  field, notification feed, session switcher write-guard, `student_details` direct
  edit, fee/grade-scale sync migration) — feature work, out of P09 scope.

**Owner decisions still open (from earlier phases, unchanged):** §13 additions
(`futures-util`, `@types/node`); native-Hindi review of number words / phrasing;
relay hosting + `RELAY_SHARED_KEY` rollout; server port 47650 confirmation.

---

## Verification (all green locally)

- `npm run verify` — typecheck, hex, i18n, deps, version, contrast, logs, 38 vitest.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — **498 passed, 0 failed** (1 ignored: benchmark).
  e2e_flows now deterministic; new: role_matrix 7, durability 2, reliability 3,
  protocol identifier tests; sync/relay/drive e2e all green.
- `cargo audit` 0 vulns · `npm audit --omit=dev` 0 vulns.
- `node scripts/size-report.mjs` (no artifacts yet → reports, exits 0).
- `node scripts/check-release-clean.mjs` — clean after the dev-route fix.

---

# P09 Follow-up — owner answers (branch `rebuild/p09`; no push/tag/deploy)

Addresses the owner's follow-up decisions. Everything below is committed on
`rebuild/p09`; nothing was pushed, tagged, released or deployed.

## 1. Dependencies — both approved, now IN §13, check-deps is STRICT
- `docs/00-SYSTEM-CONTEXT.md §13`: added **`futures-util`** (`default-features = false`,
  `std, sink` — already trimmed; relay-tunnel Sink/Stream helpers) and **`@types/node`**
  (types-only dev dep for `vite.config.ts` + `scripts/*.mjs`; never bundled).
- `scripts/check-deps.mjs`: removed the warn-only exception path — **any** dep not on
  the §13 allowlist now FAILS. `node scripts/check-deps.mjs` →
  `OK — 30 npm + 34 cargo direct deps all within §13.` (no warnings).
- `cargo tree -i aws-lc-rs` → *did not match any packages* (still empty ✓).

## 2. Code signing — NO certificates for v1 (unsigned, labelled)
- `.github/workflows/release.yml` builds cleanly with **no** `WINDOWS_*`/`APPLE_*`
  secrets: signing env is passed through but empty → Tauri builds unsigned; artifacts
  get a `-UNSIGNED` suffix; the `checksums` job prepends an **"Unsigned build"** banner
  (+ publisher line) to the release notes. Android → `-UNSIGNED-TEST` until the keystore
  secrets exist.
- Kept the signing steps for later; added a comment on the Windows step that modern
  code-signing certs keep the key on a hardware token / cloud signing service, so the
  plain-PFX approach may not work — revisit with cloud signing when a cert is bought.
- `INSTALL.md`: Windows SmartScreen (*More info → Run anyway*) and **verified** macOS
  wording — on Sonoma/Sequoia the reliable path is **System Settings → Privacy &
  Security → Open Anyway** (the Control-click shortcut was removed in Sequoia 15;
  kept as the older-macOS alternative). Verified against Apple support
  (support.apple.com/en-us/102445, .../guide/mac-help/mh40616).
- Publisher identity **"Zuhair Hussain"** / support `mohammedzuhairhussain28@gmail.com`
  / `zuhairhussain.com`: set in `tauri.conf.json` (`bundle.publisher`, `copyright`,
  `homepage`), the app's welcome footer copyright, INSTALL.md, and the release notes.
  (There is no in-app Settings → About screen yet — P08 Part E React was not built —
  so About-screen placement is a P08 follow-up; recorded here.)

## 3. Android signing — owner creates the keystore; everything else is ready
- `.gitignore` already covered `*.jks`/`*.keystore`/`keystore.properties`; added
  `*.p8`, `*.mobileprovision`, `vidya-licence-signing.key`, `*-signing.key`,
  `*.private-key`, `google-services.json`, `GoogleService-Info.plist`.
- `docs/ANDROID-SIGNING.md`: exact `keytool` genkey, SHA-1 read, base64 (macOS +
  Windows PowerShell), the four GitHub secrets, and a bold back-up-or-you-can-never-
  update warning. `release.yml` already emits `-UNSIGNED-TEST` APKs until the secrets
  exist.

## 4. App identifier — DECISION NEEDED (not changed)
- **Current (unchanged): `in.vidyabudget.app`** — `tauri.conf.json` `identifier`,
  and the Android `namespace` + `applicationId` in `gen/android/app/build.gradle.kts`.
- Owner is considering **`com.zuhairhussain.vidya`** (owns zuhairhussain.com).
- Consequences (identifier can NEVER change after the first public release):
  - **Keep `in.vidyabudget.app`** — matches the icons/config already generated; but it
    encodes a "vidyabudget" org you don't own the domain for (harmless, just cosmetic
    in the reverse-DNS sense).
  - **Switch to `com.zuhairhussain.vidya`** — cleanly under your own domain; requires
    regenerating the Android project id (`tauri android init` picks up
    `tauri.conf.json`), and the macOS bundle id changes too. Must be done **before**
    the first release and **before** creating the Android OAuth client (its package
    name is fixed to this value).
- The Google Android OAuth client (GOOGLE-OAUTH-SETUP.md) must use whichever is final.
  **I did not change it — waiting on the owner's decision.**

## 5. Build-config / hosting — prepared, not deployed
- `cloud/relay/fly.toml` + `cloud/licence/fly.toml` + `cloud/licence/Dockerfile`
  (relay already had a Dockerfile): smallest `shared-cpu-1x`/256mb, always-on
  (`auto_stop_machines="off"`, `min_machines_running=1`), region `bom` (Mumbai;
  fallback `sin`), `/healthz` checks. Licence gets a **Fly volume** at `/data`
  (`LICENCE_DATA_DIR`) for its store; added a `/healthz` route + env-driven bind
  (`LICENCE_BIND=0.0.0.0`) so it runs in a container.
- Production licence key: `cargo run -p vidya-licence -- gen-prod-key` generates a
  fresh ed25519 pair, prints the PUBLIC key (for build-config), and writes the PRIVATE
  seed to `vidya-licence-signing.key` (gitignored) for `fly secrets set
  LICENCE_SIGNING_KEY=...`. Never reuses/commits the dev key.
- **Release builds refuse the dev key**: `src-tauri/build.rs` panics if
  `licence_public_key` equals the dev key; `scripts/write-release-config.sh` refuses it
  too and fails naming any missing value.
- `.github/workflows/release.yml` writes `build-config/release.json` from five repo
  **Variables** (`LICENCE_API`, `LICENCE_PUBLIC_KEY`, `RELAY_URL`,
  `GOOGLE_CLIENT_ID_DESKTOP`, `GOOGLE_CLIENT_ID_ANDROID`) via `write-release-config.sh`
  and fails clearly if any is missing (build.rs is the backstop). Signing secrets stay
  optional.
- `docs/DEPLOY-FLY.md` (Fly install → launch → volume/secrets → deploy → `fly certs
  add` DNS → cost link) and `docs/GOOGLE-OAUTH-SETUP.md` (project, consent screen,
  `drive.file`, Desktop + Android clients) — both verified against live Fly/Google docs.

## 6. Owner checklist
`docs/RELEASE-CHECKLIST.md` — 11 ordered checkboxes (confirm identifier → keystore →
Android secrets → Fly deploy + DNS → prod licence key → OAuth clients → merge → tag →
review draft → clean-machine runs → publish), each linking to the guide, plus the
five build-config Variables table.

## Files added / changed (this follow-up)
- Added: `docs/ANDROID-SIGNING.md`, `docs/DEPLOY-FLY.md`, `docs/GOOGLE-OAUTH-SETUP.md`,
  `docs/RELEASE-CHECKLIST.md`, `scripts/write-release-config.sh`,
  `cloud/licence/Dockerfile`, `cloud/licence/fly.toml`, `cloud/relay/fly.toml`.
- Changed: `docs/00-SYSTEM-CONTEXT.md` (§13), `scripts/check-deps.mjs` (strict),
  `.github/workflows/release.yml`, `src-tauri/tauri.conf.json` (publisher/copyright/
  homepage), `src-tauri/build.rs` (dev-key refusal), `src/lib/i18n/strings/welcome.ts`
  (copyright), `INSTALL.md`, `.gitignore`, `cloud/licence/src/main.rs` (gen-prod-key,
  prod env, /healthz, bind).

## Verification (real output)
- `npm run verify` → hex OK · i18n OK (17 modules, 716 keys) · **check-deps STRICT OK
  (30 npm + 34 cargo)** · version 1.0.0 · contrast OK · logs OK · 38 vitest passed.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean. `cargo clippy` in
  `cloud/licence` → clean.
- release.yml build-config simulation (`write-release-config.sh`): no vars → fails
  naming all 5; dev key → refused; real values → writes `release.json` (gitignored).
- `cargo run -p vidya-licence -- gen-prod-key` → prints a fresh public key, writes the
  gitignored private file.
- Secret hygiene: `git ls-files` has no `.jks/.keystore/.pem/.p12/signing.key`;
  `git log -p main..rebuild/p09` has no private keys/passwords; no key files in the repo.
- **Could NOT run** the GitHub Actions workflows themselves (no `act`/runners here) or
  `docker build` (daemon down) — the workflow logic is exercised via the scripts above
  and by reading the YAML; real execution happens on GitHub/Fly at the owner's hand.

## Things only the owner can do (from RELEASE-CHECKLIST.md)
Confirm the app identifier · create + back up the Android keystore · add the four
Android secrets · deploy licence+relay on Fly + add DNS · generate the production
licence key (private → Fly secret, public → GitHub Variable) · create the two Google
OAuth clients + add the five build-config Variables · merge to main · `git tag v1.0.0
&& git push --tags` · review the DRAFT release · run the six clean-machine tests ·
publish.
