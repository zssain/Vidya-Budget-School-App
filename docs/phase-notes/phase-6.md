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
spikes, the owner delegated twice — *"choose the best option and tell me what you did"* then
*"confirmed do the best."* I took the option that produces the most real, tested, honest progress
without crossing the gate or inventing Google behaviour: **write the executable spike runbook (the
Step-0 deliverable) AND build every spike-independent, offline-provable piece — including the full
device-push + server-import exchange engine over the `DriveApi` trait, proven end-to-end against a
fake Drive.** The only parts left are the ones that hard-depend on the spike (the real Drive v3
client + OAuth) and the UI-coupled device provisional-pull. I also **made the two open policy calls**
(below), since they are mine to make and needed no Google.

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
| `audience_for(table, class_id)` + `Audience` | `crates/vidya-core/src/audience.rs` | 4 | 8 unit tests |
| `.vop` bundle seal/verify + chunking (≤500 ops/1 MB) | `src-tauri/src/sync/drive/bundle.rs` | 4/5 | 8 unit tests |
| Versioned audience-key store + rotation | `src-tauri/src/sync/drive/keys.rs` | 3 | 5 unit tests |
| `DriveApi` trait + `DriveFile`/`DriveError` | `src-tauri/src/sync/drive/mod.rs` | 2/4/5/6 | (surface) |
| Fake permission-enforcing, fault-injecting Drive | `src-tauri/src/sync/drive/fake.rs` | 9 | 6 unit tests |
| **Exchange engine: `push_outbox` + `import_all` + acks** | `src-tauri/src/sync/drive/exchange.rs` | **4/6/7** | via harness |
| End-to-end exchange harness (offline slice) | `src-tauri/tests/drive_e2e.rs` | 9 | 5 integration tests |
| **Full server-off→import→Confirmed harness** | `src-tauri/tests/sync_e2e.rs` (+5) | **9** | 5 integration tests |

### Exchange engine as built (Steps 4/6/7 mechanics — transport-agnostic)
`exchange::push_outbox` (device) groups the outbox by audience, seals each group into a `.vop`
(`chunk_ops` for the ≤500-op / ≤1 MB caps), uploads temp-name → **verify-by-readback** (re-download +
compare bytes, so no md5 crate is needed for either the fake or the real Drive) → rename, then marks
the domain rows `shared_drive` (only advancing from `draft`/`on_device`, never downgrading
`confirmed`). Retry-safe: the final name is a pure function of the ops, so a re-run finds the file
already present and skips it. `exchange::import_all` (server) discovers `ops-*` folders under
`exchange/`, downloads bundles, opens each with the server's held key **version**
(`keys::key_bytes`), applies **all** ops across all bundles in **global HLC order** through the
existing `apply::apply_op` (idempotent by op_id; revoked author → `flagged` + `review_flag`; excess
payment → flagged; conflicts detected — all unchanged), archives processed bundles to `ops-*/_done/`,
and returns a per-device sealed **ack**. `write_ack`/`read_ack` seal `{last_hlc, results[]}` with the
device session key (Step 6/7). The new `sync_e2e` scenarios prove the DONE-MEANS server side:
server-off phone push → server-on import → **Confirmed** (register changes, audit chain valid);
tampered bundle → **quarantined**, register unchanged; suspended author → **flagged**, not applied;
same op via LAN **and** Drive → applied **once**; ack round-trips to the device.

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

## Policy calls I made (audience of cross-cutting tables) — confirm
`audience_for` maps every table:
- `finance`: fee_head, fee_due, payment, payment_allocation, reversal **+ student, enrollment**
- `class:<id>`: attendance_sheet, attendance_mark, marks_sheet, mark_entry, exam, exam_subject
- `admin`: school, session, term, subject, class, class_subject, staff, device, invite, conflict,
  review_flag, notification, licence, grade_scale, grade_band

**Decision — `student`/`enrollment` → `finance`** (default; changeable). The pusher must hold the
audience key; student/enrollment are written by accountants + the Principal (both hold `finance`),
teachers only *read* the roster. `finance` is the one audience every pusher of these tables holds.
The alternative (`class:<id>`) would let teachers see roster changes via Drive but leave accountants
unable to seal their own admission edits — worse. **Consequence to accept:** during a server outage a
teacher won't see a *new* admission via Drive until the server returns (it reconciles on import).

**Decision — `request` audience = the requester's own domain** (a teacher's correction under
`class:<own>`, an accountant's under `finance`), so it is record/actor-dependent, not table-only;
`audience_for("request", …)` returns `VALIDATION` on purpose and the write site passes it explicitly.
Today only three sites emit ops (`payment` → finance ✓; approve-request + a p04 admin action → admin,
both Principal actions ✓); attendance/marks/student **client** writes don't yet emit outbox ops, so
`audience_for` becomes load-bearing in the write path only when Step 4's client push is wired — the
exchange engine already groups by each op's stored `audience`.

## What's left in Phase 6 (the Google-dependent + UI parts)
1. **Step 1 OAuth** *(gated on the spike + needs Java/NDK/device)* — desktop PKCE loopback (`reqwest`
   + `tauri-plugin-opener`), refresh token encrypted in the DB; Android = the Spike-B method (maybe
   an owner-approved Android dep).
2. **Step 2** *(gated)* — a `DriveApi` impl over the **real** Drive v3 (`reqwest`) that satisfies the
   same trait the engine already uses; folder provisioning + the Settings → Google Drive screen
   (sharing status per staff). `[VERIFY — Spike A]` whether other `drive.file` users can read a
   file's public `properties`; if not, fall back to encoding audience+version in the filename.
3. **Step 5 device pull** *(engine core straightforward; the UI/rules are the work)* — the 20 s ± 3 s
   foreground loop, **provisional** apply within scope marked "Shared through school Drive · waiting
   for school", never overwrite the device's own unsent (flag locally instead), payments shown only
   in "waiting for server" totals. `import_all`'s decrypt/HLC/quarantine logic is reusable; the
   provisional (non-confirming) client apply + the totals/UI are new.
4. **Step 8** the full failure matrix → specific Needs-attention items (the `DriveError` variants are
   already the Step-8 cases; each needs its exact copy + fix action + safe-requeue).
5. Wire `audience_for` into the client write path once attendance/marks/student client writes emit
   outbox ops (Step 4's client half).

**Already built (were "left"):** the device push (Step 4 mechanics), server import + HLC order + acks
+ `_done/` archive (Step 6), device ack read (Step 7), and the full `server-off → import → Confirmed`
harness incl. LAN+Drive dedup, tampered-quarantine and revoked-flagging (Step 9) — all over the
`DriveApi` trait, so the real client drops straight in.

## Verification (real output, this branch)
- `cargo test --workspace` → **400 pass, 0 fail** (was 363 at P05): vidya lib **119** (+19 drive),
  drive_e2e **5** (new), e2e_flows 4, relay_e2e 5, **sync_e2e 13** (+5 Drive exchange), vidya-core
  **243** (+8 audience), no_floats 1, doc 10. (benchmark `#[ignore]`.)
- `cargo clippy --workspace --all-targets -- -D warnings` → **clean**.
- Crypto gate: `cargo tree -i aws-lc-rs` / `-i aws-lc-sys` → *no packages* (absent); `ring` is the
  sole TLS provider — unchanged.
- No JS/TS changed this phase (`tsc`/`vitest` unaffected).
- **Not run (environment):** the two spikes (no Google accounts/Cloud/device), any Android/APK build
  (no Java on PATH), and the real Drive/OAuth transport (built later, post-spike).

## Questions for the owner
1. **Run the Step-0 spikes** (`phase-6-spikes.md`) — the ONE real blocker: does `drive.file` let
   staff read each other's files in a shared folder? Which Android sign-in works, and its APK delta?
   The whole engine is built and green; only the real transport + OAuth wait on this.
2. **Confirm the two policy calls I made** (above): `student`/`enrollment` → `finance`;
   `request` → the requester's own domain. Say the word if you want student/enrollment on the class
   audience instead (with the accountant-sealing caveat).
3. Confirm the `keys` non-persistence fix (behaviour-preserving; all tests green).
4. Prior defaults kept (lease 30 d; per-device tokens); the P05 opens (relay host/domain,
   `RELAY_SHARED_KEY` management, `futures-util` companion dep) are still awaiting sign-off.
