# Repair status — P21 (repairs before the owner portal)

Branch `v2/p21-repairs`, cut from `v2/p20-live-sync` at `ec7808b` (30 Oct 2026). The local,
uncommitted `src-tauri/build-config/dev.json` change (Google OAuth client IDs) was left untouched.
Spec: `~/Downloads/Vidya_Fix_and_Owner_Portal_Implementation_Prompt.md`. **Owner decision
(30 Oct 2026): app repairs first; the Razorpay/Supabase owner portal (spec Phases 10–13) is
deferred** — it reverses the recorded manual-UPI purchase model and the ₹0/month rule.
`Vidya_Codebase_Review.md` (named by the spec) was not available; findings come from the
guide's §24, two code audits run on this branch, and direct code reading.

Status: **FIXED** (code + regression test on this branch) · **PARTIAL** · **OPEN** ·
**BLOCKED** (needs hardware / accounts / an owner decision) · **NOT REPRODUCED**.

## Baseline (at `ec7808b`, before any change)

| Check | Result |
|---|---|
| `npm run verify` (typecheck, gates, 63 vitest) | pass |
| `npm run check:release-clean` | pass |
| `cargo test --workspace --locked` | 813 pass, 0 fail |
| `cargo clippy --workspace --all-targets -D warnings` | **fail** — 2 doc-indent errors, `tests/client_bootstrap.rs:9-10` |
| GitHub CI (latest) | **red** — clippy above + macOS/Windows size-gate jobs panic in `build.rs` (no `release.json` in CI) |
| `npm run build:web`, `test:e2e`, `test:e2e:pwa` | NOT RUN yet |

## Issue matrix

| ID | Symptom | Where | Status | Fix / evidence |
|---|---|---|---|---|
| S1 | Joined phone / second PC opens with no PIN (auto-signed-in as the joined staff) | `state.rs compute`, `lib.rs build_ctx` | **FIXED** `c43db5b` | New `pin_setup` state; `security::device_pin` (encrypted app_kv, Argon2id, persisted lockout); cold start = locked; unlock only as the joined staff. Tests: `device_pin::*`, `state::joined_client_*`, `tests/client_bootstrap.rs`. Upgrade note: a device joined on an earlier build goes to PIN setup on first launch; no production installs exist (all releases are drafts), so Principal re-verification of legacy enrolments was not built. |
| S2 | No auto-lock at all on desktop/Android (§9 says 5 min) | — | **FIXED** `c43db5b` | `RtCtx` wall-clock idle deadline enforced in `require_session` (sleep counts; clock-back fails closed); UI pings `session_activity` on real input only; re-routes on LOCKED / return / every 20 s. Tests: `ctx::tests::*`. The 1/5/15-minute setting is not wired (fixed 5 min). |
| S3 | PWA locked only after 5 min *hidden*, never on idle; dispatcher served data while locked; `create_pin` could replace an existing PIN | `web/lock.ts`, `web/backend.ts` | **FIXED** `c43db5b` | Timestamp idle lock; dispatcher allows only the PIN gate while locked; `createPin` refuses if a PIN exists. Tests: `src/lib/web/lock.test.ts`. |
| S4 | 54 of 200 commands had no session check (fees, payments, Principal dashboard, staff/devices, backups, Drive, restore…); setup commands (`create_pin`, `setup_principal`) ran any time → locked PC's Principal PIN could be overwritten | `commands/mod.rs` | **FIXED** `c43db5b` | Guards: setup-phase only; restore/activate/join only with no school (or Principal); session for reads; role gates for fees / admin. Requests scoped to requester unless Principal. **Remaining:** teacher reads of students on the school PC are not class-scoped (session only). |
| A1 | Server authorised ops from the `staff_id` inside the op, not the authenticated token → any teacher device could act as Principal | `server/net.rs push`, `service::push`, `sealed.rs`, `apply.rs` | **FIXED** `e48d0ec` | `apply_client_op` binds device/staff to the auth; device must be live and belong to that staff. Test `op_claiming_another_staff_is_rejected`. |
| A2 | Unmapped tables (staff, device, request, class, app_kv, …) were raw-upserted with any columns | `apply.rs action_for → apply_upsert` | **FIXED** `e48d0ec` | Deny by default (`UNSUPPORTED_OP`). Test `unmapped_tables_are_denied_by_default`. |
| A3 | One failing op aborted the push / Drive import (Drive sync stalled forever) | `service::push`, `exchange::import_all` | **FIXED** `e48d0ec` | Per-op error → remembered `MALFORMED`, batch continues. Test `a_failing_op_does_not_abort_the_batch`. |
| A4 | Locked (submitted) sheets editable by teachers directly; Principal reason never required | `apply.rs target_for` | **FIXED** `e48d0ec` | `is_locked` from stored sheet → EditSubmitted* → `NEEDS_REQUEST` / `REASON_REQUIRED`. Test `teacher_edit_of_submitted_attendance_needs_request`. |
| A5 | Permission target (class) taken from the payload; update could relabel another class's row | `apply.rs resolve_class_id` | **FIXED** `e48d0ec` | Stored row wins; ownership columns immutable on update; `*_by` must be the author. Test `update_cannot_relabel_a_row_into_own_class`. |
| A6 | Op `server_epoch` never checked | apply path | **FIXED** `e48d0ec` | `EPOCH_OLD`. Test `stale_epoch_and_delete_are_refused`. Protocol-version header still unchecked. |
| A7 | Dedup by op_id only; first writer wins (poisoning); client drops rejected/conflict acks silently | `apply.rs`, `sync/client.rs:313` | **PARTIAL** | Server half FIXED `e48d0ec` (owner + content hash, migration 0032; test `reused_op_id_with_other_content_is_refused`). Client still deletes non-confirmed acks without surfacing → Phase 5/7. |
| A8 | Drive: audience key is the only authentication; revoked devices keep pushing | `exchange::import_all` | **PARTIAL** | Ops bound to the folder's device row (unrevoked) and bundle audience `e48d0ec`. **Open:** folder placement is not cryptographic proof — per-device bundle MAC/signature (sync-protocol-v2); audience-key rotation on revoke is never called in production. |
| A9 | Drive acks routed by untrusted folder name | `exchange.rs`, `live.rs` | **OPEN** | Depends on A8 per-device authentication. |
| A10 | SQL injection via peer bundles in provisional pull | `drive/pull.rs provisional_upsert` | **FIXED** `e48d0ec` | Table allowlist + identifier gate. |
| A11 | `delete` kind upserted; missing base_version skips conflict check; backdated HLC; `/v1/join` unthrottled | apply / net | **PARTIAL** | delete refused `e48d0ec`. Others OPEN (low). |
| C1 | Drive join: invite code uploaded in plaintext; response key = SHA-256(code) → anyone reading `exchange/joins/` decrypts the device token, session key and every audience key | `seal.rs`, `drive/join.rs`, `web/join.ts`, `live.rs` | **FIXED** (P21 join v2) | Web invite carries a 256-bit secret in the URL fragment (migration 0033, cleared on use); request = invite id + request id + HMAC proof (no code/secret); response sealed under HMAC-derived per-request key, AAD binds invite/request/school; v1 refused; answered requests deleted; invite consume atomic (`used_at IS NULL`). Tests: `seal::tests::join_v2_*` (incl. golden vector cross-checked in Python and the real WASM via Node), `tests/join_drive.rs` (Drive reader can't open, forged/altered/replayed/v1 refused, invite not consumed by attacks), `tests/drive_server_tick.rs`, `src/lib/web/join.test.ts`. Not done: independent crypto review; stashed pending invite (incl. secret) sits in plain PWA KV until join completes (K2). |
| C2 | Invite code = 40 bits | `service.rs create_invite` | **MITIGATED** | Code now only used on the pinned-TLS LAN route (online, single use); never a key. `/v1/join` still unthrottled (A11). |
| B1 | PWA couldn't read native Drive acks (encrypted vs plain JSON, different fields) → outbox never drained, everything re-uploaded every 20 s, peer pull stopped | `web/drive/sync.ts`, `web/store.ts` | **FIXED (code)** | `openAck` = native format (s2c from session key, AAD `ACK\n<dev>\n<dev>\n0`, `{last_hlc, results[]}`); confirmed → removed, rejected/conflict/flagged → kept with reason (`refusedOps()`, not dropped); unreadable ack no longer aborts the pass; outbox rows carry `uploaded_at` (no re-upload until 30 min unacked). Tests `src/lib/web/outbox.test.ts`. **Not done:** UI surfacing of refused ops; browser E2E of the Drive loop (`test:e2e:pwa` not run). |
| B2 | PWA uses one key version for all audiences | `web/join.ts`, `sync.ts` | **OPEN** | Store per audience. |
| D1 | PWA ops fall back to empty device/staff and epoch 1; epoch never refreshed | `web/op.ts` | **OPEN (low)** | Throw if not joined; refresh epoch. |
| W1 | **Almost no client write syncs correctly** (see `write-sync-matrix` in the audit): attendance draft/submit, leave, create/cancel request, note delete → no op; marks, payments, students, expenses, messages, guardians, calendar, circulars → op with `{}` payload (fails or confirms with no data); staff check-in partial; homework note native op fails NOT NULL. Only the PWA `save_homework_note` applies correctly. | `commands/logic.rs` (see audit lines) | **OPEN — high** | Spec Phases 5/6. Needs owner decisions on audiences for teacher-originated admin-audience writes (leave, check-in, messages). |
| W2 | `sync_now` doesn't sync (returns status only) | `commands/p04.rs:296` | **OPEN** | |
| I1 | iPhone invite link = Vercel *preview* URL behind Vercel login | `server/invite.rs` | **FIXED (code)** / **BLOCKED (deploy)** | Origin now `build-config pwa_origin` (`config::validate_pwa_origin`: https, bare, not a Vercel preview; dev → localhost:5273), checked again in `build.rs`; release writes it from repo variable `PWA_ORIGIN`; no origin → no web invite shown (no broken link). Tests `config::tests::pwa_origin_validation`, `invite::tests::web_join_link_*`. **Owner:** deploy the PWA at a stable public domain without deployment protection, set `PWA_ORIGIN`, add it to the Web OAuth client origins. |
| G1 | Bootstrap snapshot silently dropped rows (insert errors + unparsable lines ignored); "complete" inferred from a school row | `engine::apply_snapshot`, `transport::snapshot`, `client_sync_tick` | **FIXED** (native LAN) | All-or-nothing transaction; strict NDJSON parse; refuses a snapshot without a school row; `PRAGMA foreign_key_check` names any dangling reference; `bootstrap_complete` set in the same tx; self-heal retries until set; new `bootstrap_pending` state (loading screen, not an empty home). Tests: `client_bootstrap::{every_role_snapshot_applies_completely, a_corrupt_snapshot_is_rolled_back_entirely, …}`, `state::joined_client_with_pin_*`. **Not done:** server manifest (per-table counts/digest, snapshot id, cursor binding for chunked Drive export); PWA has no snapshot route (Drive canonical export — Phase 7). |
| G1a | (found by G1) Teacher snapshot omitted the class_subject rows its timetable slots reference → timetable subjects silently missing on teacher phones | `scope.rs` | **FIXED** | Teacher scope now includes all subject assignments in the teacher's classes (snapshot + pull rule). |
| G1b | (found by G1) Clients enforce FKs, but canonical rows (payment/expense/advance) reference the server-only `device` registry → snapshot rows dropped and **every pulled payment from another device failed the pull** on Principal/Accountant devices | `engine.rs apply_pull/apply_snapshot` | **FIXED** | Canonical application on clients runs with FK enforcement off (server is the integrity authority); snapshot completeness still checked, ignoring only server-only parents (`device`, `invite`). |
| G2 | Unconditional `eprintln!` join logging; raw error chains on the welcome screen; every command failure `console.error`'d with its full detail | `sync/client.rs`, `App.tsx`, `lib/backend.ts` | **FIXED** | Join diagnostics → `tracing` (rotated app log file, all platforms; debug/info/warn; no codes/tokens); raw detail folded behind "Technical details"; console logs only `cmd` + error code. `check:logs` passes. |
| G3 | CI red: clippy | `tests/client_bootstrap.rs` | **FIXED** `c43db5b` | clippy clean. |
| G4 | CI red: desktop size-gate jobs need `release.json` | `ci.yml`, `build.rs` | **FIXED (pending CI run)** | `scripts/write-ci-config.mjs` writes a CI-only config with a throwaway random licence key (no private key exists → no licence verifies; dev key never used); `build.rs` production checks unchanged; Windows CI artifact renamed `…-CI-ONLY-not-a-release`. Verified locally only — needs a push to confirm on GitHub. |
| G5 | Main frontend chunk 837 KB | Vite build | **OPEN (low)** | Code-split desktop routes. |
| K1 | Android DB key stored as a plain file (`FileKeyStore`) | `security/keys.rs`, `lib.rs` | **OPEN** | Spec Phase 9: Android Keystore plugin (needs Android toolchain to verify). |
| K2 | PWA device token / session / audience keys stored as plain KV values | `web/join.ts`, `store.ts` | **OPEN** | Spec Phase 9. |
| K3 | Backup dirty-detection by audit head only | `backup/schedule.rs` | **OPEN** | Spec Phase 9. |

## External / owner blockers (not code)

- Public PWA domain + hosting without deployment protection; Google Web OAuth origin for it.
- Real-hardware runs: two-PC LAN, Android join, iPhone end-to-end, Drive Q-B harness.
- Owner decisions: audiences for teacher leave/check-in/messages (W1); PWA/website domain
  (`neverworks.org` currently serves another site); signing identities; native Hindi/Telugu review.
- Owner portal (spec Phases 10–13): deferred by the owner.
