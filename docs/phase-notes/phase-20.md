# Phase 20 handoff — making sync/Drive actually live (desktop Drive backup; sync audit; Drive spike)

## ⟳ Session 2 — live Drive sync (the P12 blocker) + iPhone PWA onboarding

All on `v2/p20-live-sync`, verified, **not pushed**. This session built the whole
live-sync spine end to end: the school-PC Drive loop, then the iPhone's join + sync +
lock cycle on top of it.

**Phase C1 — the school-PC live Drive client (the carried P12 blocker) — DONE ✅**
- `src-tauri/src/sync/drive/live.rs` `server_drive_tick` (`fe82447`): one pass answers
  pending PWA join requests in `exchange/joins/` (writes the sealed response), then
  `import_all` + a sealed `write_ack` per device. `tests/drive_server_tick.rs` proves a
  pass answers a join + provisions the web device + is idempotent (fake Drive).
- `server_drive_pass` + the background loop (`c89dca2`): on a Server with Drive
  connected, every 20 s (own DB conn, blocking thread) it builds the real `GoogleDrive`
  (reusing the Phase-A desktop sign-in), resolves `Vidya/<school> (<id>)/exchange` (the
  SAME path clients use), and runs the tick. The real Google round-trip is owner-verified.

**Phase C2 — iPhone PWA join + Drive sync loop — DONE ✅**
- `app_state` on the web backend + a pure boot-route mapper (`72899d1`): the PWA leaves
  the status screen and routes (join → PIN gate → home). Fixed the raw-key display bug
  (`error.NOT_AVAILABLE_ON_WEB` en/hi).
- `src/screens/web/JoinScreen.tsx` + App wiring (`4240e8c`): invite fragment → Connect
  Google Drive (GIS, `drive.file`, no secret) → write request → poll for the school PC's
  sealed response → set PIN. An idempotent effect starts the foreground Drive sync loop
  whenever the PWA is unlocked. This closes the loop with C1: PWA requests → school PC
  answers → PWA opens + syncs. Join/seal contract covered by the Rust + web suites.

**Phase C3 — port the phone commands — STARTED 🟡**
- `list_staff` + `unlock` (`725a433`): the PIN lock/unlock cycle now works on the PWA
  (relaunch / 5-min auto-lock → PIN gate → home). `unlock` rejects with the SAME
  CmdError the desktop throws (PIN_LOCKED `{until}` / PIN_WRONG `{remaining}`); pure
  `unlockError` mapper unit-tested.
- **Remaining C3 (the big chunk):** the teacher-screen DATA + ACTION commands, each a
  store-backed port with exact op/HLC/DTO parity (a wrong op shape silently breaks sync):
  - read: `my_staff_day`, `my_timetable`, `list_classes`, `list_absent`,
    `list_homework_notes`
  - write (emit ops): `save_attendance_draft`, `save_homework_note`,
    `delete_homework_note`, `email_homework_note`, `staff_check_in`, `staff_check_out`,
    `request_leave`, `create_request`, `record_message`
  - `server_status` is intentionally LEFT rejecting — on the PWA it is a LAN-reachability
    probe MyAttendanceScreen catches to choose the Drive route (correct behaviour).

**Verification each step:** `npm run verify` (typecheck + i18n parity + hex + contrast +
logs + api-consistency + 49 vitest) green; the Rust side `cargo build/clippy -p vidya
--lib` + `cargo test --test drive_server_tick` green.

**Still owner-only (hardware/Google):** `drive-test.html` (Q-B) — the whole Drive DATA
path for the iPhone rests on it; a desktop Drive backup+restore round-trip; a two-PC LAN
sync; and the iPhone end-to-end (join approved by a connected school PC). The two cheap
checks (Q-B + desktop Drive) de-risk the entire remaining C3 port — worth doing first.

---

## ⟳ Session update — desktop restore + LAN client now COMPLETE

Since the sections below were first written, two more owner-ordered pieces landed
(all on `v2/p20-live-sync`, verified, not pushed):

- **Desktop is now fully complete** — restore was the missing half of backup and is
  now wired end-to-end (local file + in-app Google Drive):
  - R1 salt-carrying (`.vbak.meta` + Drive appProperties) `9c5b28e`
  - R2 restore commands (summary + staged install + restart) `1b4c0b0`
  - R3 restore-from-Drive `57a4ff6`; R4 Recover screen `2d93716`; docs `58e900f`
  - A web-downloaded `.vbak` alone can't be restored (salt is in Drive properties) →
    use in-app "Restore from Google Drive". See `docs/DRIVE-BACKUP.md`.
- **Phase B (LAN multi-device client) is COMPLETE** — it was greenfield (no client
  runtime existed); now built + CI-verified with a real server-start harness:
  - B1 client-identity store `ef06c8c`
  - B2 join over pinned TLS + `tests/sync_lan_e2e.rs` (real join + authed pull) `4c3574f`
  - B3 `client_sync_tick` `1e3f4bc` + startup role detection & background sync loop
    `a4a431c` + the `join_school` command & Welcome "Join a school" wiring `12abacd`
  - The harness proves join + pinned-TLS sync end-to-end over localhost; the real
    **two-PC** run is the owner's final check. Code-only (mDNS) join + mDNS
    re-discovery of a moved server IP are noted follow-ups.

**Still gated on the owner:** the Q-B Drive test (unblocks the Drive sync client +
iPhone — #3/#4 below), plus the live desktop round-trip and a real two-PC LAN test.

---

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
