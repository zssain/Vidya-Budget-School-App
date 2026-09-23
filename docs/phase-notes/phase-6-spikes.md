# Phase 6 — Step 0 spike RUNBOOK + results

Branch: `rebuild/p06` (from `rebuild/p05`). Start HEAD: `67624e6` (P05 close-out). `git status` at start: clean.
Tools: rustc/cargo 1.98.1, node v25.3.0, npm 11.7.0. **No Java runtime on PATH** (`java -version`
→ "Unable to locate a Java Runtime"); **cargo-tauri not installed** — so the Android half (Spike B)
and any APK build/measure **cannot run in this environment** and must run on the owner's machine.

> **STATUS: PLAN — RESULTS PENDING.** This file is the exact runbook to execute Step 0's two
> spikes. Standing rule 4 forbids asserting Google API behaviour without real test accounts, and
> this sandbox has **no Google accounts, no Google Cloud project, and no Android device**. So the
> "Result" fields below are left blank for the owner (or a session with credentials + a device) to
> fill from real runs. Nothing here claims a Google outcome that has not been observed. The
> endpoints/scopes/flows cited are the stable, documented Google Drive v3 + OAuth 2.0 surface; the
> **unknowns being tested are called out with `[VERIFY]`.**

---

## 0. One-time setup (owner)

**Test accounts (4).** Create four throwaway Google accounts and label them:
`principal@`, `teacher-va@` (class V-A), `teacher-vib@` (class VI-B), `accountant@`.

**Google Cloud project.**
1. console.cloud.google.com → new project "Vidya Spike".
2. APIs & Services → Enable **Google Drive API**.
3. OAuth consent screen → **External**, **Testing** mode; add all four accounts as **Test users**.
   (Testing mode avoids Google verification during the spike.)
4. Credentials → Create OAuth client **Desktop** → note `client_id` (Desktop).
5. Credentials → Create OAuth client **Android** → package `in.vidyabudget.app`, SHA-1 of the
   debug keystore → note `client_id` (Android).

Record the two client ids; they belong in `src-tauri/build-config/<env>.json`
(`google_client_id_desktop`, `google_client_id_android`) — never hard-coded elsewhere (§10).

---

## Spike A — does the `drive.file` scope let staff read each other's files in a shared folder?

**The question (§11 `[VERIFY]`, STOP if false):** `drive.file` normally limits a token to files the
app *created* or the user *opened via a picker*. Step 0 must prove whether a staff device, using the
**same OAuth client** with scope `https://www.googleapis.com/auth/drive.file`, can **list and
download** files that *another* account created inside a folder the Principal shared with it. If it
cannot, staff-to-staff Drive exchange is impossible under `drive.file` and we go to the owner with
the fallbacks (§ end).

### A.0 — Get a token per account (desktop, `drive.file`)
For each of the four accounts, run the installed-app PKCE loopback flow (the same flow Step 1 will
implement):
- code verifier = 64 random URL-safe bytes; `code_challenge = BASE64URL(SHA256(verifier))`, method `S256`.
- open in the account's browser:
  `https://accounts.google.com/o/oauth2/v2/auth?client_id=<DESKTOP>&redirect_uri=http://127.0.0.1:<port>&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&code_challenge=<challenge>&code_challenge_method=S256&state=<state>&access_type=offline&prompt=consent`
- catch the `code` on `127.0.0.1:<port>`, then
  `POST https://oauth2.googleapis.com/token` (form) `client_id, code, code_verifier, grant_type=authorization_code, redirect_uri` → `{access_token, refresh_token, expires_in}`.

Record: does consent list only "See, edit, create, and delete only the specific Google Drive files
you use with this app"? (the `drive.file` consent string). **Result:** _______

### A.1 — Principal creates the §11 layout (Principal token)
```
# create the root and subfolders (repeat, threading parent ids)
curl -X POST https://www.googleapis.com/drive/v3/files \
  -H "Authorization: Bearer $PRINCIPAL" -H "Content-Type: application/json" \
  -d '{"name":"Vidya","mimeType":"application/vnd.google-apps.folder"}'
# → {id: ROOT}. Then children with "parents":["ROOT"]:
#   backups/ (NOT shared), exchange/ (shared reader), exchange/acks/,
#   exchange/ops-<devVA>/, exchange/ops-<devVIB>/  (shared writer per owner)
```
**Result (ids):** root ___ backups ___ exchange ___ acks ___ ops-VA ___ ops-VIB ___

### A.2 — Principal shares (permissions API)
```
# exchange/ as reader to all three staff
curl -X POST https://www.googleapis.com/drive/v3/files/$EXCHANGE/permissions \
  -H "Authorization: Bearer $PRINCIPAL" -H "Content-Type: application/json" \
  -d '{"role":"reader","type":"user","emailAddress":"teacher-va@..."}'   # ×3 staff
# each ops-<device>/ as writer to just its owner
curl -X POST https://www.googleapis.com/drive/v3/files/$OPS_VA/permissions \
  -H "Authorization: Bearer $PRINCIPAL" -H "Content-Type: application/json" \
  -d '{"role":"writer","type":"user","emailAddress":"teacher-va@..."}'
```
Record each permission id (needed for A.6 unshare). **Result:** _______
`[VERIFY]` Does sharing require the staff to **open the folder once via a picker** before their
`drive.file` token can see it? Record any such required extra step. **Result:** _______

### A.3 — Principal writes a bundle into ops-VA and a backup into backups
```
# multipart upload of a fake .vop into ops-VA (Principal, to seed cross-user read)
curl -X POST "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart" \
  -H "Authorization: Bearer $PRINCIPAL" \
  -F "metadata={\"name\":\"seed.vop\",\"parents\":[\"$OPS_VA\"],\"properties\":{\"vidya_audience\":\"class:VA\",\"vidya_keyver\":\"1\"}};type=application/json" \
  -F "file=@seed.bin;type=application/octet-stream"
# a backup into backups/
curl ... parents:["$BACKUPS"] name:"2026-09-24.vidbak"
```
**Result (file ids):** seed.vop ___ backup ___

### A.4 — Teacher V-A: can it read files it did NOT create, inside exchange? `[VERIFY — the core question]`
```
export T=$TEACHER_VA
# (1) list ops-VA (Principal created seed.vop there)
curl "https://www.googleapis.com/drive/v3/files?q='$OPS_VA'+in+parents+and+trashed=false&fields=files(id,name,md5Checksum,size,properties)" -H "Authorization: Bearer $T"
# (2) download the Principal-created seed.vop
curl "https://www.googleapis.com/drive/v3/files/$SEED_VOP?alt=media" -H "Authorization: Bearer $T"
# also: can V-A read V-A's *own* uploads, and list exchange/ and ops-VIB (created by others)?
```
**Result:** list status ___ download status ___ can it see `properties`? ___
(If (1)/(2) return **200 with the file/bytes**, `drive.file` cross-user-in-shared-folder **works →
continue**. If **404/403/empty**, it does **not** → fallbacks.)

### A.5 — Teacher V-A: create in its own folder; denied elsewhere + in backups
```
# (a) create in ops-VA (should succeed — writer)
curl -X POST ".../upload/drive/v3/files?uploadType=multipart" -H "Authorization: Bearer $T" ... parents:["$OPS_VA"]
# (b) create in ops-VIB (should FAIL — only reader via exchange)
curl ... parents:["$OPS_VIB"]      # expect 403 insufficientFilePermissions
# (c) delete the Principal's seed.vop (should FAIL)
curl -X DELETE ".../files/$SEED_VOP" -H "Authorization: Bearer $T"   # expect 403
# (d) delete/enter backups/ (should FAIL / not visible)
curl ".../files?q='$BACKUPS'+in+parents" -H "Authorization: Bearer $T"  # expect empty/403
curl -X DELETE ".../files/$BACKUP" -H "Authorization: Bearer $T"        # expect 403/404
```
**Result:** (a) ___ (b) ___ (c) ___ (d) ___

### A.6 — After unsharing, is access gone?
```
curl -X DELETE ".../files/$EXCHANGE/permissions/$PERMID_VA" -H "Authorization: Bearer $PRINCIPAL"
# then repeat A.4 as V-A → expect 404/403
```
**Result:** _______

### A.7 — Drive **web UI** check (not just the API)
Sign in to drive.google.com as `teacher-va@`. Confirm: staff **cannot** see or delete `backups/`;
staff **cannot** delete files in `ops-VIB/`; staff **can** see the shared `exchange/`.
**Result:** _______

**Spike A verdict:** ☐ `drive.file` works (staff read each other in shared folder) → continue
· ☐ does not work → fallbacks below.

---

## Spike B — Android sign-in (get a `drive.file` token on a real phone)

`[VERIFY — §11]` Google restricts custom-scheme redirects for Android OAuth clients. Test both, on a
**real Android device** (cannot run in this sandbox — no Java/NDK on PATH, no device):

**(1) System browser + loopback / App Link redirect.** Same PKCE flow as A.0 but with the Android
client id; redirect to an **https App Link** the app verifies (custom `http://127.0.0.1` loopback is
generally rejected for Android clients — record the exact error if so).
**Result:** works? ___ error if not ___

**(2) Android authorization API via `plugins/vidya-android`.** Use Google's current Android
authorization path (Credential Manager / `AuthorizationClient.authorize` with the Drive scope) to
obtain an access token, bridged to Rust through the in-repo Kotlin plugin.
- Dependency needed (Play Services auth / `androidx.credentials`)? **A new Android dependency needs
  owner approval** (§13). Record artifact + version.
- **APK size delta** per ABI (arm64-v8a, armeabi-v7a) from adding it: build the debug/release APK
  with and without, `scripts/size-report.mjs`. Budget: each APK download ≤ 40 MB (§2).
**Result:** works? ___ dependency ___ APK delta arm64 ___ / armv7 ___

**Spike B verdict:** chosen method ___ ; new dependency for owner approval ___ ; APK delta ___.

---

## Report → owner decision (STOP until chosen)

Fill A.7 + B verdicts, then pick ONE (§11, prompt Step 0):
- **(a) `drive.file` works** → continue to Steps 1–9 with the desktop PKCE loopback + the Spike-B
  Android method.
- **(b) it does not** → owner chooses a fallback:
  - **Restricted `drive` scope** — full-Drive read; needs Google **verification + a security
    assessment** (real cost + weeks of lead time for the owner). 
  - **Principal-account-only Drive** — no staff-to-staff exchange; only server backups to Drive +
    staff queue changes locally until the server returns (LAN/relay). Smaller, no verification.

**Owner decision (date + choice):** _______________________________________________

---

## What is already built against the spike outcome (no Google needed)

So Steps 1–9 are mostly wiring once the gate clears, the spike-independent, offline-provable pieces
are done and tested on this branch (see `docs/phase-notes/phase-6.md`): the `DriveApi` trait + a
fake permission-enforcing Drive, the `.vop` bundle seal/verify + chunking format, the versioned
audience-key store + rotation, `vidya_core::audience::audience_for`, and the **full device-push +
server-import exchange engine** (`sync::drive::exchange`) — proven end-to-end against the fake Drive
(`server-off → import → Confirmed`, LAN+Drive dedup, tampered→quarantine, revoked→flagged, acks). The
only parts that HARD depend on the spike are the **real** `DriveApi` client (Drive v3 over `reqwest`)
and **OAuth** (desktop loopback + the Spike-B Android method), plus the UI-coupled device
provisional-pull (Step 5) — deliberately not built yet.
