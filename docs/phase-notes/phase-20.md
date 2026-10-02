# Phase 20 handoff — making sync/Drive actually live (desktop Drive backup; sync audit; Drive spike)

## Start state / branch

- Branch `v2/p20-live-sync` (off `v2/p19`). Not merged, not tagged, not pushed.
- Goal for this phase (owner): build the four "make it actually work" items from the
  runtime audit, in order — (1) desktop Google Drive backup, (2) automatic LAN sync
  loop, (3) multi-device Drive sync, (4) iPhone PWA usable.
- Built + verified on macOS; Rust stable + reqwest/keyring (already deps), wasm-pack,
  Playwright/WebKit all present.

## The honest runtime audit (what was LIVE vs scaffold, start of phase)

- **Single-PC desktop app:** LIVE (189 commands, real DB) — a complete, sellable
  single-PC product with **local** backups.
- **Google Drive backup:** NOT built (local only; `run_backup(..., None)`).
- **LAN multi-device:** server runs + advertises, but **no client runtime** (device
  mode hardcoded Server; no client-join; no sync loop).
- **Drive exchange / iPhone sync:** scaffold only; blocked by an unanswered research
  question since P6.
- **iPhone PWA:** foundation only (web backend serves `create_pin` + `lock`).

---

## What this phase delivered

### ✅ #1 Desktop Google Drive backup — COMPLETE (code), owner live-test pending
Commits `b02c2ea`→`6181f67` (A1–A5). The Mac/Windows app now backs up to the owner's
Google Drive in addition to the PC:
- **A1** `sync/drive/oauth.rs` — installed-app OAuth 2.0 **PKCE loopback** (verified vs
  Google's official docs; no client secret), tokens in the encrypted `app_kv`.
- **A2** `sync/drive/google.rs` — real **Drive REST v3** client (`impl DriveApi`):
  upload (multipart/related) / list / download / folder / rename / delete, Drive error
  mapping.
- **A3** wired into `backup::schedule` (every backup also uploads + verifies + prunes on
  Drive when connected; local-only + never-fails otherwise) + `drive_connect` /
  `drive_status` / `drive_disconnect` commands + the scheduler tick moved to
  `spawn_blocking`.
- **A4** Backups screen "Google Drive (off-site copy)" card (en/hi).
- **A5** guard test + `docs/DRIVE-BACKUP.md`.
- **Verified:** `cargo test -p vidya --lib` 310 pass; clippy clean; `npm run verify`
  (typecheck + hex + i18n + api-consistency + 40 tests). **Owner must run the live
  Google sign-in + upload once** — `docs/DRIVE-BACKUP.md` §2 (the only part CI can't do).

### 🔁 #2 LAN multi-device client — scoped, deferred by owner
On investigation this is **not** "add a loop" — the whole **client lifecycle** is
greenfield: there is no client-side identity store (this device's token/session_key/
audience-keys), no client-join HTTP caller, no startup role detection, no mDNS-driven
sync loop. The pieces that DO exist: the server `/v1/join` route, the sync **engine**
(`sync_once` / `sync_once_routed`), the pinned-cert **HttpsTransport**, mDNS
`discover`, and the `JoinResp`/invite contracts. Owner chose to defer this and pivot to
the Drive path (which also serves the iPhone). It is only engine-verifiable here; the
true two-PC sync needs two machines.

### 🔬 #3 Drive multi-device — research spike done; STOP on one owner test
Commits `621480a`, `ee2ec6c`. Verified vs Google's official docs: `drive.file` is
per-file / per-user. The **decisive, undocumented** question (`docs/phase-notes/
drive-file-spike.md`, "Q-B"): **can the iPhone's Web OAuth client read files the
Desktop client created** (same account, same Cloud project) under `drive.file`? §18's
zero-cost iPhone-via-Drive design depends on this being **YES**. Built a **dev-only
harness** (`web-pwa/drive-test.html`, reuses the real P19 web OAuth) that prints a
YES/NO verdict. **This is a hard STOP** — the Drive sync client and the iPhone sync
(Phase D) are not built until the owner runs the harness and reports Q-B.

### 🧰 Also: domain rebrand to `neverworks.org` + OAuth docs
Commit `20bcd54`. Swapped the placeholder domain across active docs/site/app strings;
`GOOGLE-OAUTH-SETUP.md` now covers all **three** OAuth clients (Desktop/Web/Android)
and fixes the Secrets→**Variables** error (the workflow reads `${{ vars.* }}`).

---

## Owner action items (the real unblockers — ~10 min, one sitting)

Signed in as the **school's Google account**, on a desktop build with
`GOOGLE_CLIENT_ID_DESKTOP` configured (`docs/DRIVE-BACKUP.md` §1):

1. **Confirm #1 live:** Backups → Connect Google Drive → Back up now → see
   "Backed up · this PC and Drive" + a `Vidya/backups/*.vbak` file in Drive.
2. **Answer Q-B:** add `http://localhost:5273` to the Web client's Authorized JS
   origins → `npm run dev:web` → open `http://localhost:5273/drive-test.html` → sign in
   as the same account → read the verdict.

Then report **Q-B = YES / NO**.

## Design tree (what gets built after Q-B)

- **Q-B = YES →** build one Drive-sync client (sealed `exchange/` bundles; all devices
  sign in as the one shared school account; sealing + audience keys still gate
  decryption) serving desktop multi-device **and** the iPhone PWA (Phase D). §18 design.
- **Q-B = NO →** iPhone can't use the Drive exchange → wire it to the **relay** (P05,
  built; normal TLS cert) and keep Drive for desktop backup only. Cost/hosting change →
  explicit owner decision.

## Still remaining (not started)

- Phase B client lifecycle (if the owner wants LAN multi-PC): client identity store +
  join caller + role detection + mDNS loop. Large; two-PC verification.
- Phase C Drive sync client — after Q-B = YES.
- Phase D iPhone PWA usable — port the phone-screen commands to the web backend
  (currently only `create_pin`/`lock`) + wire sync/join; depends on the #3 outcome.

## STOP conditions — status

- **No dependency added** — reqwest/keyring already §13; no new crates.
- **Invented Google behaviour** — none; OAuth flow verified vs official docs, and the
  one undocumented question (Q-B) was turned into an owner-runnable test, **not
  guessed** (the explicit STOP).
- **Real blocker worked around silently** — NO. Q-B is documented as a hard stop; the
  Phase B greenfield scope + the "live tests are owner-only" limits are stated plainly.
- **Weakened/deleted a test** — none; suites expanded, all green.

## Re-verify

```bash
npm run verify                      # typecheck + gates + 40 JS tests
cargo test -p vidya --lib           # 310 Rust lib tests
cargo clippy -p vidya --lib         # clean
GOOGLE_CLIENT_ID_DESKTOP=… npm run tauri build   # (owner) real desktop build to live-test #1
```
