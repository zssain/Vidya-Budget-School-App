# PHASE 6 of 10 — GOOGLE DRIVE FALLBACK (SPIKES FIRST), AUDIENCE KEYS, ANDROID SIGN-IN

## ROLE
You are a senior Rust + Android engineer continuing **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL, then every file in
   `docs/phase-notes/`.
2. Record branch, HEAD, `git status`, tool versions at the top of your handoff.
3. Only context §13 dependencies with the listed features. No Google SDK. Anything else →
   STOP and ask.
4. Never invent Google API behaviour, scopes, endpoints or quotas. Use the official Google
   Drive v3 and OAuth documentation and prove behaviour with real test accounts.
5. The mock wins. Status copy exactly as context §8.10.
6. Business rules only in vidya-core; the server re-validates Drive ops like any other op.
7. No fake success; "Shared through school Drive" appears only after a verified upload.
8. Never delete/weaken tests. Never edit an earlier phase's migration.
9. Run every command you mention; paste real output. Separate environment vs code failures.
10. Work on branch `rebuild/p06`; small commits; no push/merge/tag unless asked.
11. Stop conditions are real.
12. Finish with the handoff file.

## OBJECTIVE
When the school server is unreachable but the internet works, devices exchange encrypted
changes through the school's Google Drive, and the server imports them when it returns.
Teachers can never read other classes' data or delete backups.

## STEP 0 — TWO SPIKES. Report and STOP for owner approval before Step 1.
Use four separate test Google accounts: Principal, Teacher V-A, Teacher VI-B, Accountant.
Create a Google Cloud test project with an OAuth client (Desktop) and (Android); keep the
consent screen in Testing mode with these accounts as test users.

**Spike A — `drive.file` scope.** With the Principal account (desktop app, `drive.file`):
create the folder layout in context §11, share `exchange/` as reader with the three staff
emails and `exchange/ops-<device>/` as writer with its owner. Then with each staff
account (same OAuth client, `drive.file`): (1) can it list and download files the Principal
and other staff created inside `exchange/`? (2) can it create files in its own
`ops-<device>/`? (3) is it denied writing/deleting in other folders and in `backups/`?
(4) after unsharing, is access gone? Record exact API calls, responses and any required
extra step (e.g. the user opening the folder once through a picker). Test through the API
AND the Drive web UI (a staff member must not be able to delete `backups/` there either).

**Spike B — Android sign-in.** On a real Android device, get a Drive access token for the
staff account: try (1) system browser + loopback/App Link redirect; (2) Google's current
Android authorization API (Credential Manager / Identity AuthorizationClient) via
`plugins/vidya-android`. Record which works with the Android OAuth client, which
dependency (if any) it needs and its APK size impact. A new Android dependency needs owner
approval.

**Report** in `docs/phase-notes/phase-6-spikes.md`: results, screenshots/logs, and one of:
(a) `drive.file` works → continue; (b) it doesn't → options: restricted `drive` scope
(requires Google verification + security assessment — cost/time for the owner), or
"Principal-account-only Drive" (no staff exchange; only server backups + staff queue
locally). **Do not continue until the owner chooses.**

## STEPS (after approval)

### Step 1 — OAuth (`src-tauri/src/sync/drive/oauth.rs`)
Desktop: PKCE (S256), `state` check, system browser via `tauri-plugin-opener`, loopback
listener on `127.0.0.1:<random port>`, token exchange + refresh with `reqwest`. Android:
the method chosen in Spike B. Tokens (refresh token encrypted in the DB) per device; never
copied between devices. Revocation/expiry detected on 401 → "Reconnect Google Drive" in
Needs attention.

### Step 2 — Folder provisioning (server, Principal account)
Settings → Google Drive (desktop, derived screen): Connect / Disconnect, connected account,
folder link, sharing status per staff ("Shared", "No Google account", "Failed — retry"),
storage used. Create the context §11 layout; share per staff on invite and on assignment
change; unshare on remove/suspend. All sharing via Drive permissions API.

### Step 3 — Audience keys
Audiences: `admin`, `finance`, `class:<class_id>` (context §8.8). Server generates 32-byte
keys with a version; delivered to devices only for their audiences at join and on change
(through the normal sync channel, sealed). Assignment change → rotate that class key (new
version); devices keep old versions only if they already had them. Tests: teacher V-A
cannot decrypt a `class:VI-B` bundle; accountant cannot decrypt class bundles.

### Step 4 — Device push to Drive
When the server is unreachable (LAN and relay failed) and internet works: group outbox ops
by audience (an op's audience is decided by its table/record, in vidya-core
`audience_for(op)`); write `exchange/ops-<device_id>/<hlc>-<audience>-v<keyver>.vop`
(≤ 500 ops or 1 MB) with a temp name then rename; verify by reading back the metadata/md5
Drive returns; only then set rows `shared_drive`. Retry safely (same file name = same
content; never duplicate ops).

### Step 5 — Device pull from Drive (only while the server is unreachable)
Every 20 s ± 3 s in the foreground: list new bundles in other devices' folders; download
those whose audience key the device holds; decrypt (AEAD failure → quarantine + notify
Principal); apply as PROVISIONAL rows within the device's scope, visually marked with
"Shared through school Drive · waiting for school"; never overwrite the device's own
unsent changes (flag locally instead). Cursor per folder. Payments seen provisionally are
shown in "waiting for server" totals, never in confirmed totals.

### Step 6 — Server import
On start and every 5 min while Drive is connected: download all new bundles from all
`ops-*` folders, decrypt with the right audience key version, apply in global HLC order
through the normal op path (idempotent by op_id), write `exchange/acks/<device_id>.json`
sealed with the device's session key `{last_hlc, results[]}`, move processed bundles to
`exchange/ops-<device_id>/_done/` (delete after 30 days).

### Step 7 — Acks on devices
If the server is still unreachable directly, devices read their ack file to move ops to
`confirmed`/`rejected`/`conflict` exactly as with a direct response.

### Step 8 — Failure handling
Quota full, token revoked, folder deleted/unshared, rate limited (429/403 rate → backoff
with jitter), partial upload, clock skew: each → a specific Needs-attention item with the
exact fix, and pending work stays safely queued.

### Step 9 — Harness
Add a fake Drive (local folder implementing the same `DriveApi` trait, with injected
failures) to `sync_e2e.rs`: server off → phones exchange via Drive → server on → import →
all confirmed; same op via LAN and Drive → applied once; tampered bundle → quarantined;
revoked device's later bundle → flagged, not applied; mixed: one phone reaches the server
via relay while another uses Drive → consistent end state.

## DONE MEANS
Real-hardware run with the four test accounts: server PC off → phone A records attendance →
"Shared through school Drive · waiting for school" → phone B (same class) sees it as
provisional; phone C (other class) cannot → server PC on → imports → all phones
"Confirmed by school server". Harness green. Staff cannot delete `backups/` via API or
Drive UI.

## HANDOFF → `docs/phase-notes/phase-6.md`
Spike results + owner decision · OAuth details per platform · folder/permission model as
built · key rotation rules · failure matrix · harness results · APK size delta · questions.
