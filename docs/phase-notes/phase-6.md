# Phase 6 handoff — Google Drive fallback (STOPPED at the Step 0 gate; offline scaffolding built)

Branch: `rebuild/p06` (from `rebuild/p05`). Start HEAD: `67624e6` (P05 close-out). `git status` at
start: clean.
Tools: rustc/cargo 1.98.1, clippy 0.1.98, node v25.3.0, npm 11.7.0.
**Environment gaps (real, not code):** no Java runtime on PATH (`java -version` fails), cargo-tauri
not installed, **no Google accounts / Google Cloud project / Android device** in this sandbox. So the
Step-0 spikes and any Android/APK work cannot run here (rule 9: environment vs code).

## The gate (why this phase is a STOP, not a full build)
Step 0 is a hard STOP: *"TWO SPIKES. Report and STOP for owner approval before Step 1… Do not
continue until the owner chooses."* And standing rule 4 forbids asserting Google API behaviour
without real test accounts. I therefore did **not** build Steps 1–9's Google-dependent parts.

**Owner decision this phase (the STOP):** asked how to clear Step 0 given the sandbox can't run the
spikes, the owner delegated — *"choose the best option and tell me what you did."* I took the option
that produces the most real, tested, honest progress without crossing the gate or inventing Google
behaviour: **write the executable spike runbook (the Step-0 deliverable) AND build only the
spike-independent, offline-provable scaffolding.** Everything that hard-depends on the spike (the
real Drive v3 client + OAuth) is left for after the owner's (a)/(b) decision.

## Step 0 — spikes: RUNBOOK delivered, results PENDING
`docs/phase-notes/phase-6-spikes.md` is the exact, runnable Step-0 runbook: the 4 test accounts +
Cloud/OAuth setup, every Drive v3 + OAuth 2.0 call for Spike A (`drive.file` cross-user-in-a-shared-
folder — the core `[VERIFY]`), the Drive-web-UI checks, and the Spike B Android sign-in tests + APK
size deltas. Results fields are blank for a run with real accounts + a device. **No Google outcome is
claimed.** Owner must run it and pick (a) `drive.file` works → continue, or (b) fallback (restricted
`drive` scope + Google verification, or Principal-account-only Drive).

## Built this phase (spike-independent, tested; maps to later steps)
All new code is provable offline and interchangeable with the real Drive client via one trait.

| Piece | File | For step | Proven by |
|---|---|---|---|
| `audience_for(table, class_id)` + `Audience` | `crates/vidya-core/src/audience.rs` | 4 | 7 unit tests |
| `.vop` bundle seal/verify + chunking (≤500 ops/1 MB) | `src-tauri/src/sync/drive/bundle.rs` | 4/5 | 8 unit tests |
| Versioned audience-key store + rotation | `src-tauri/src/sync/drive/keys.rs` | 3 | 5 unit tests |
| `DriveApi` trait + `DriveFile`/`DriveError` | `src-tauri/src/sync/drive/mod.rs` | 2/4/5/6 | (surface) |
| Fake permission-enforcing, fault-injecting Drive | `src-tauri/src/sync/drive/fake.rs` | 9 | 6 unit tests |
| End-to-end exchange harness (offline slice) | `src-tauri/tests/drive_e2e.rs` | 9 | 5 integration tests |

### Bug fixed (pre-P06)
`server::service::audience_keys_for` minted a **fresh random key on every call and never persisted
it** — two devices in the same audience (e.g. two accountants on `finance`) would get *different*
keys and could never read each other's Drive bundles. It now uses the persisted, versioned
`sync::drive::keys::get_or_create`. Join still returns the same shape (`AudienceKey{audience,key_b64,
version}`); all 8 `sync_e2e` + join tests stay green.

### `.vop` format as built (§11)
- Body = `nonce(12) ‖ ChaCha20-Poly1305(audience_key, JSON array of ops)` — reuses the vetted
  `sync::seal` primitives. **AAD binds `audience` + `key_version`**, so a bundle can't be relabelled
  to another audience/version folder without failing the open.
- Filename = `<hlc>-<audience>-v<keyver>.vop` (§11), kept human-readable + chronologically sortable.
  Because an HLC and a `class:<uuid>` audience both contain `-`/`:`, the name is **not parsed** for
  routing — the audience + key version travel in the Drive file `properties` so a reader can pick the
  right key *without downloading*. **`[VERIFY — Spike A]`** whether other `drive.file` users can read
  a file's public `properties` in a shared folder; if not, the fallback is to base64url-encode the
  audience in the name (needs a parseable delimiter — flagged).
- Chunking splits by op count (≤500) and serialized size (≤1 MB with headroom for nonce+tag).

### Key rotation as built (§8.8)
`keys::rotate(audience)` mints a new key at `current+1` and **keeps all old versions**; the server
holds every version (opens old bundles on import); a device keeps only versions it was delivered.
Proven: teacher V-A's key cannot open a `class:VI-B` bundle; accountant (finance) cannot open class
bundles; a v1 bundle stays readable after rotation to v2.

## `[OWNER]` decision needed — audience of cross-cutting tables
`audience_for` covers the tables whose audience is unambiguous:
- `finance`: fee_head, fee_due, payment, payment_allocation, reversal
- `class:<id>`: attendance_sheet, attendance_mark, marks_sheet, mark_entry, exam, exam_subject
- `admin`: school, session, term, subject, class, class_subject, staff, device, invite, conflict,
  review_flag, notification, licence, grade_scale, grade_band

**`student`, `enrollment`, `request` are deliberately undecided** (`audience_for` returns
`VALIDATION`). They cross the finance/class boundary: the **pushing device must hold the audience
key**, but an accountant holds only `finance` and a teacher only `class:<own>`, so no single
table→audience rule keeps every pusher able to seal. Options for the owner:
- **student/enrollment → `finance`** (accountants + Principal push them; teachers read the roster
  from the server, so a teacher won't see a *new* admission via Drive during an outage), OR
- **→ the student's `class:<id>`** (teachers see roster changes, but accountants then can't seal
  their own admission edits — would need a finance key on those bundles too).
- **request → the requester's own domain** (a teacher's correction request sealed under
  `class:<own>`, an accountant's under `finance`) — i.e. audience is record/actor-dependent, not
  table-only. Until decided, the two existing op sites keep their explicit audiences (`payment` →
  finance ✓; approve-request → admin) and `audience_for` is not yet wired into the write path.

## What's left in Phase 6 (all gated on the Step-0 decision)
1. **Step 1 OAuth** — desktop PKCE loopback (`reqwest` + `tauri-plugin-opener`), refresh token
   encrypted in the DB; Android = the Spike-B method (needs Java/NDK/device + possibly an
   owner-approved Android dep).
2. **Step 2** the real `DriveApi` over Drive v3 (`reqwest`), folder provisioning + the Settings →
   Google Drive screen (sharing status per staff).
3. **Steps 4–6 engine** — device push (group by `audience_for`, temp→verify→rename→`shared_drive`),
   device pull (20 s ± 3 s, provisional apply, "Shared through school Drive · waiting for school",
   never overwrite own unsent), server import (every 5 min, HLC order, acks, `_done/` archive).
4. **Step 7** device acks; **Step 8** the full failure matrix → specific Needs-attention items.
5. **Step 9** wire the fake Drive into the full `server-off → import → Confirmed` loop in
   `sync_e2e.rs` (dedup LAN+Drive is already idempotent by op_id; revoked-author flagging already in
   `apply.rs`). The offline slice is proven in `drive_e2e.rs`.
6. Wire `audience_for` into the write path once the cross-cutting mapping is chosen.

## Verification (real output, this branch)
- `cargo test --workspace` → **394 pass, 0 fail** (was 363 at P05): vidya lib **119** (+19 drive),
  drive_e2e **5** (new), e2e_flows 4, relay_e2e 5, sync_e2e 8, vidya-core **242** (+7 audience),
  no_floats 1, doc 10. (benchmark `#[ignore]`.)
- `cargo clippy --workspace --all-targets -- -D warnings` → **clean**.
- Crypto gate: `cargo tree -i aws-lc-rs` / `-i aws-lc-sys` → *no packages* (absent); `ring` is the
  sole TLS provider — unchanged.
- No JS/TS changed this phase (`tsc`/`vitest` unaffected).
- **Not run (environment):** the two spikes (no Google accounts/Cloud/device), any Android/APK build
  (no Java on PATH), and the real Drive/OAuth transport (built later, post-spike).

## Questions for the owner
1. **Run the Step-0 spikes** (`phase-6-spikes.md`) and record the verdict: does `drive.file` let
   staff read each other's files in a shared folder? Which Android sign-in works, and its APK delta?
2. **Audience mapping** for `student`/`enrollment`/`request` (above) — which option?
3. Confirm the `keys` non-persistence fix (behaviour-preserving; all tests green).
4. Prior defaults kept (lease 30 d; per-device tokens); the P05 opens (relay host/domain,
   `RELAY_SHARED_KEY` management, `futures-util` companion dep) are still awaiting sign-off.
