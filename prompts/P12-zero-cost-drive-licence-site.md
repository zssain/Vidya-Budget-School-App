# PHASE 12 — ZERO-MONTHLY-COST PLATFORM: SHARED SYNC ACCOUNT, DRIVE ROUTE, OFFLINE LICENCES, STATIC SITE

## ROLE
You are a senior Rust + Android + Tauri engineer continuing **Vidya Budget School** (v2).

## STANDING RULES (same in every v2 phase)
1. Read IN FULL before anything else: `docs/00-SYSTEM-CONTEXT.md`, `docs/01-MOCK-SPEC.md`,
   `docs/02-V2-CHANGES.md` (the decision record; merged into 00 in Phase 11),
   `docs/03-PROTOTYPE-SPEC.md`, then every file in `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. You are extending a built product. **Read the actual code before changing it.** Never rebuild
   something that exists; extend it. If code and docs disagree, report it and follow the docs;
   if the docs are silent, STOP and ask.
4. Dependencies: only context §13 plus 02-V2-CHANGES §12. Anything else → STOP and ask.
5. Never invent features, rules, copy, numbers, library APIs or Google/Meta/NPCI behaviour.
   Check official docs or installed source. Unsure → STOP.
6. **The mock and the prototype win.** New screens match `design/prototype/VidyaPrototype.jsx`
   exactly, built from the app's existing components and tokens.
7. Business rules only in `crates/vidya-core`; commands thin; the server re-validates every op.
8. Every write = one transaction: row + audit + op (+ voucher and ledger entries for money).
9. Data safety: migrations are additive and numbered, tested on a copy of a v1 database (demo
   seed + fixtures). Never edit or delete a committed migration. Never delete financial or
   audit rows.
10. No fake success; statuses are honest.
11. Never delete or weaken tests. P01–P10 suites must still pass; where 02-V2-CHANGES changes
    behaviour, update the expected values and list every such change in the handoff.
12. Every visible string via `t()` in `en.json`, `hi.json` and (from Phase 13) `te.json`.
13. Zero monthly cost: nothing may require a company server.
14. Run every command you mention and paste real output. Branch `v2/p12`; small commits; no
    push, merge, tag or deploy unless the owner asks.
15. Stop conditions are real. Finish with `docs/phase-notes/phase-12.md`.


## OBJECTIVE
Remove every dependency on a company-run server while keeping all functions: phones reach the
school through the LAN or the school's shared Google "sync account"; backups go to the Principal's
private Google account; licences become offline files made on the owner's laptop; the website
becomes a static page; the old licence server is retired and the relay becomes an optional module.

## DONE MEANS
1. Real-hardware run: school PC on, phone on mobile data → attendance "Confirmed by school server"
   in ≤ 2 minutes through the sync account's Drive.
2. School PC off → two phones exchange provisional changes through Drive → PC on → everything
   confirmed within ~1 minute.
3. Fresh install: Welcome → Set up shows the machine code → owner runs `licence-maker issue` →
   licence key pasted (and `.vlic` loaded, both tested) → setup continues with no internet.
4. Existing v1 installs keep working after upgrade without any licence step.
5. Restore to a second PC with a transfer licence → the first PC becomes read-only as soon as
   it next reads Drive.
6. Release builds need no `LICENCE_API` and no `RELAY_URL`; `cloud/licence` is archived and not
   built; `site/` builds to static files.
7. Harness scenarios (below) green; docs updated.

## STEPS

### Step 0 — Spike: one shared account, many devices (report, then continue or STOP)
With one test Gmail as the sync account and the app's existing OAuth clients (desktop + Android):
sign in the school PC, one Android phone and one second desktop, each with its own OAuth flow.
Prove with the real Drive API (`drive.file` scope): each device can create files in
`Vidya/<school>/exchange/ops-<its id>/`, and list, download and write files created by the other
devices in the same folder tree; removing a device's refresh token (revoking it in the Google
account) stops only that device. Record calls, responses and timings in
`docs/phase-notes/phase-12-spike.md`. If any step fails → STOP and report options.

### Step 1 — Accounts model and migration
1. Migration: `drive_account(kind TEXT CHECK(kind IN ('sync','backup')) PRIMARY KEY, email, token_enc,
   root_folder_id, exchange_folder_id, notes_folder_id, backups_folder_id, connected_at, status)`.
2. v1 installs had one Principal Drive connection used for both exchange and backups. Migrate it
   to `kind='backup'` (backups keep working). Home "Needs attention" then shows "Connect the
   school sync account" until the Principal connects one.
3. Remove per-staff folder sharing (the Drive permissions calls from Phase 6) from the code paths;
   keep `staff.google_email` readable but unused (dropped in Phase 18). Remove per-staff Google
   sign-in from Profile.
4. Pending v1 bundles still sitting in the old exchange folder of the Principal's account are
   imported once by the server, then archived (`ops/_done/`), exactly like a normal import.

### Step 2 — Settings → Google Drive (prototype `settings` state 2)
Two cards: **School sync account** (connect / reconnect / disconnect; email; devices connected;
seconds since last check; "Encrypted — nobody can read the files") and **Backup account
(private)** (backups kept, last verified, space used of free space if the API reports it). Copy
from the prototype. Connecting uses the existing desktop OAuth (PKCE, loopback).

### Step 3 — Joining signs the device into the sync account
Join flow (prototype `join`): after "Is this your school?" add a step **"Sign this phone in to the
school sync account"** with the text "Your Principal types the password. Teachers don't need it."
The device runs its own OAuth (Android method from the Phase 6 spike). Skippable ("Skip — this phone
will sync only on school Wi-Fi"). Staff & access → device row shows "Sync account: signed in /
not signed in". After removing a device, show the hint: "To be extra safe, change the sync
account's password."

### Step 4 — Route order and timing
Sync engine routes: LAN → Drive → queue (relay only when the `instant_sync` module is on).
School PC import loop: every 30 s ± 5 s while running (was 5 min); also on start and on network
change. Devices: pull from Drive every 20 s ± 3 s in the foreground; push immediately after writes.
Keep exponential backoff on errors. Status copy unchanged.

### Step 5 — Epoch fencing through Drive
Server key pair for signing `epoch.json` (generated at setup, private key encrypted in the DB;
public key shipped to devices at join). Write `exchange/epoch.json` on setup, on every epoch change
and daily. Server reads it on every import: file epoch > own → switch to read-only fenced state
("This computer is no longer the school server" + "Export a copy for the Principal"). Devices store
the highest epoch seen from either the server or the file and refuse lower.

### Step 6 — Licence v2 (offline)
1. vidya-core `licence.rs`: `machine_code(machine_id)` (SHA-256 → 60 bits → 12 Crockford chars +
   1 checksum char, displayed `XXXX-XXXX-XXXX-C`), `parse_licence_key(text)`, `verify_v2(payload,
   sig, pubkey, own_machine_code)` → `Licence` or `LICENCE_INVALID` / `LICENCE_OTHER_MACHINE`.
   Keep verifying v1 licences (already activated installs): v1 is now treated as perpetual; remove
   the 30-day online re-check and grace logic.
2. Welcome → Set up (prototype `activate` layout, adapted): shows **"Your computer code"** with Copy,
   short instructions ("Send this code with your UPI payment reference to
   mohammedzuhairhussain28@gmail.com"), then **Licence key** field (paste; groups of 5) or **Load
   licence file (.vlic)** via the dialog plugin. "Checking" = local signature check (instant).
3. Recover flow: shows this PC's machine code and accepts a transfer licence for the same
   `licence_id`; on success epoch + 1 (Step 5).
4. Remove all calls to `LICENCE_API`; build config keeps only `licence_public_key` (release build
   fails if it's the dev key).

### Step 7 — `tools/licence-maker` (owner's laptop, never shipped)
Rust CLI, own Cargo.toml (not in the workspace), deps limited to `ed25519-dalek`, `rand`, `sha2`,
`base64`, `serde`, `serde_json`, `time`, `uuid`. Commands: `init --dir <path>`, `issue --school
--machine --utr --email [--notes]`, `transfer --licence <id> --machine <code>`, `list`, `verify
<key>`. Validates machine-code checksums; refuses duplicate UTRs (warns and asks `--force`);
appends to `register.csv` (date, licence_id, school, email, machine, utr, kind issue|transfer,
previous licence). README: how to back up the key folder and register in two places; what to do
if a laptop is lost (make a new keypair → next app release ships the new public key; old licences
stay valid because the app keeps both public keys).

### Step 8 — Retire the licence server; relay optional
Move `cloud/licence/` to `cloud/_archive/licence/` with a README "Retired in v2 — kept for
reference". Remove it from CI and release workflows. `cloud/relay/` stays, marked "Optional add-on
(Instant sync) — not needed for normal use". Release checks: `RELAY_URL` optional; `LICENCE_API`
removed.

### Step 9 — Static website `site/`
Plain HTML + CSS (no framework, no external requests except the owner's own images), using brand
tokens and the Welcome-screen look: home (what Vidya is, platforms, honest size text: "Each
download under 40 MB. Your school's data stays on your school's computer."), price `[PRICE]`,
**How to buy** (UPI QR image placeholder `site/assets/upi-qr.png` [OWNER]; steps: pay → note the
UTR → open Vidya → copy your computer code → send both by email via a prefilled `mailto:` link →
receive your licence key), Downloads (from `releases.json`: version, date, size, SHA-256, signing
status), Privacy, Terms, Refund (drafts marked "Draft — owner review"), footer "Developed by
Zuhair Hussain". `release.yml` writes `site/releases.json`. `docs/DEPLOY-SITE.md`: GitHub Pages
and Cloudflare Pages steps + DNS for `vidya.zuhairhussain.com`. Do not deploy.

### Step 10 — Owner's selling guide
`docs/SELLING.md`: one-page checklist — check UPI payment in bank app → confirm UTR not in
register → `licence-maker issue …` → email the key + download link → note in register; transfer
requests; refund handling (from the draft policy).

### Step 11 — Tests
- Harness (`sync_e2e.rs`) with the fake Drive now modelling **one shared account**: phone via
  Drive only while PC on (confirmed ≤ 2 import cycles); PC off/on; same op via LAN and Drive →
  once; v1 exchange folder import; epoch file fencing; removed device → keys rotated, its later
  bundle flagged.
- vidya-core: machine code checksum, licence v2 verify, wrong machine, v1 licence accepted.
- licence-maker: issue/verify round trip, duplicate UTR, transfer chain.
- Upgrade test: v1 database + v1 licence → v2 app starts without prompts.

## STOP CONDITIONS
Spike failure. Android sign-in to the shared account impossible without a new dependency. Any
change that would lock existing customers out.

## HANDOFF → `docs/phase-notes/phase-12.md`
Spike result · accounts migration · join changes · timings measured (LAN, Drive, PC on/off) ·
epoch design · licence v2 format + maker usage · what was archived · site pages · tests · owner
decisions touched · what Phase 13 needs.
