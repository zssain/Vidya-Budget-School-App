# Phase 12 — Step 0 spike runbook (one shared account, many devices)

**Status: NOT YET RUN — needs the owner's real Google accounts + three physical
devices.** This is a hard gate (a STOP condition): the live Google Drive OAuth
client and the Settings/join/route wiring (Steps 2–4) must not be built until this
spike passes, because standing rule 5 forbids asserting Google/`drive.file`
behaviour without a real test. Phase 6's `phase-6-spikes.md` left these answers
blank; Phase 12 re-raises them for the **one shared account** model.

Record every call, response body and timing in this file as you go, then paste the
verdict into `docs/phase-notes/phase-12.md` and either continue (build Steps 2–4 +
the live Drive client) or STOP and bring the options to the owner.

## What is already proven offline (does NOT need this spike)

- The `.vop` bundle seal/verify, versioned audience keys, and the exchange/pull
  logic (Phase 6, `src-tauri/src/sync/drive/`).
- The **one shared account** access model, cross-device read/write, and the
  `epoch.json` sign→write→read→verify→fence path — against the fake shared account
  (`FakeDrive::provision_shared_account`, tests in `sync/drive/fake.rs`, crypto in
  `vidya-core::epoch`). These pin the *intended* behaviour the live spike must
  confirm on real Google.

## Prerequisites

1. One throwaway **sync account** Gmail (e.g. `vidya.spike@gmail.com`).
2. A Google Cloud project with the app's **OAuth clients**: one **Desktop** client
   (installed-app PKCE, loopback) and one **Android** client, both requesting the
   single scope `https://www.googleapis.com/auth/drive.file` (the narrowest scope —
   files the app created or the user opened via a picker).
3. Three devices, each doing its OWN OAuth sign-in to the SAME sync account:
   - the school **PC** (desktop OAuth),
   - one **Android phone** (Android OAuth),
   - a **second desktop** (desktop OAuth), standing in for another staff PC/phone.

## Spike A — does `drive.file` let devices on one shared account see each other's files?

The v2 question is different from Phase 6's (which asked about *sharing between
different accounts*). Here every device is the **same account**, so the real
question is: with `drive.file`, does a file created by device X (same account, same
OAuth **client project**) appear to device Y via `files.list`/`files.get`, and can Y
download and write in the same folder tree?

Steps (record the raw HTTP for each):

1. **PC** creates the tree with Drive REST v3:
   - `POST /drive/v3/files` (folder) → `Vidya`, then `Vidya/<school> (<school_id>)`,
     then `exchange/`, `exchange/acks/`, `notes/`.
   - `POST /upload/drive/v3/files?uploadType=multipart` → `exchange/epoch.json`
     (contents from `vidya_core::epoch::sign_epoch`).
2. **PC**, **phone**, **2nd desktop** each create their own
   `exchange/ops-<device_id>/` folder and upload a small `<hlc>-admin-v1.vop`.
3. From EACH device call `GET /drive/v3/files?q='<exchange_id>' in parents` and
   `q='<other-device-ops-id>' in parents`. **Expected:** each device lists the files
   the other two created.
4. From EACH device `GET /drive/v3/files/<other-file-id>?alt=media` (download) and
   `POST` a new file into the shared `exchange/` (write). **Expected:** both succeed
   because it is one account.
5. **Revoke** one device's refresh token in the Google account's security settings.
   Re-run steps 3–4 from that device (**expected 401 → `TokenRevoked`**) and from the
   other two (**expected: still work**).

**PASS:** every device lists/reads/writes the others' files in `exchange/`, and a
revoked token stops only that device.
**FAIL / STOP options (bring to owner):**
- If `drive.file` does NOT surface sibling-created files across devices on one
  account → fall back to the broader **`drive`** scope (needs Google's sensitive-
  scope **verification**, weeks of lead time — report the steps/time), OR keep the
  exchange within a single always-signed-in identity. Do not proceed on a guess.

## Spike B — which Android sign-in works, and the APK cost?

Try, on a real Android phone, in order; record which works and the APK size delta:

1. **System browser + App Link loopback** (same PKCE flow as desktop, Android
   client id, `https` App Link redirect the app verifies). No new dependency.
2. If (1) is rejected by Google on Android → **Android authorization API**
   (`AuthorizationClient.authorize` / Credential Manager via Play Services auth) in
   the in-repo `plugins/vidya-android` Kotlin plugin. This is a **new Android
   dependency** (`androidx.credentials` / Play Services auth) that needs **owner
   approval under §13** and must keep each APK ≤ 40 MB (arm64-v8a, armeabi-v7a).

**PASS:** one method signs the phone into the sync account and obtains a refresh
token usable for Spike A. **STOP** if neither works without a disallowed dependency.

## Timings to record (for DONE-MEANS 1–2 and Step 4)

- LAN op → "Confirmed by school server" (should be ~instant).
- Phone on mobile data, PC on → op confirmed through Drive in **≤ 2 minutes**
  (import loop 30 s ± 5 s).
- PC off → two phones exchange provisional changes via Drive; PC on → everything
  confirmed within **~1 minute**.
- Restore/transfer to a 2nd PC → the 1st PC reads `epoch.json` and goes read-only
  the next time it reads Drive.

## After the spike

- Paste calls/responses/timings above and the verdict into `phase-12.md`.
- If PASS: build the live `drive.file` client behind the existing `DriveApi` trait,
  wire Settings → Google Drive (Step 2), join sign-in (Step 3), the LAN→Drive→queue
  route + import/pull timings (Step 4), and epoch.json write/read on the import loop
  (Step 5 live half). Then remove the P06 per-staff FakeDrive sharing and remodel
  the harness to two accounts.
- If FAIL: STOP; bring the scope fallback (broader `drive` scope + verification, or
  a single-identity exchange) to the owner before any Drive wiring.
