# Sync protocol v2 (P21) — what the school PC accepts and how devices talk to it

Status: implemented on `v2/p21-repairs`; verified by the Rust/JS suites named below.
Not yet verified on real hardware or a live Google Drive. The cryptographic design
has **not** had an independent review (spec Phase 7 asks for one before claiming
production assurance).

## 1. Identities

| Thing | Who holds it | Purpose |
|---|---|---|
| Device token (32 random bytes, hex) | device; server stores SHA-256 | LAN bearer auth (`/v1/sync/*`) |
| Session key (32 bytes, base64) | device + server (`device.session_key`) | per-device keys: `c2s`, `s2c` = HMAC-expand(session_key, "vidya/relay/{c2s,s2c}/v1") |
| Audience keys | nobody (removed) | no longer issued at join (A8): nothing in either direction is sealed under an audience key, so revoking a device needs no key rotation — its token and session key die with the device row |
| Device PIN | device only (encrypted `app_kv` / PWA kv) | unlocks the UI session; never sent anywhere. On the PWA it also derives (Argon2id, separate salt) the in-memory key that wraps the stored secrets (K2) |

The join token authenticates the **device**, never a human session. A joined device
needs its own PIN (`pin_setup` state) and auto-locks after 5 minutes idle.

## 2. Ops (device → school PC)

Every op: `{op_id, hlc, device_id, staff_id, audience, table, record_id, kind, payload, base_version, server_epoch}`.

Client ops are stamped centrally (`write::stamp_client_op`): the joined device id, a
packed monotonic HLC (persisted in `app_kv.last_hlc`), the newest known epoch.

Inbound routes all go through `sync::apply::apply_client_op(conn, OpAuth{device_id, staff_id, route}, op)`:

1. `op.device_id`/`op.staff_id` must equal the authenticated device and its staff
   (refused FORBIDDEN, **not** remembered).
2. Idempotency: `applied_ops(op_id, device_id, payload_hash)`. A repeat from the same
   device with the same content returns the stored result; any other reuse of the
   op_id → `DUPLICATE_OP`, not remembered.
3. Device must exist, be unrevoked and belong to that staff.
4. `op.server_epoch < school.server_epoch` → `EPOCH_OLD`.
5. `apply_op`: identifier gate; deny by default (only tables/kinds with a server
   rule; `delete` refused); ownership columns of existing rows immutable, `*_by` must
   be the author; module gate; then either a **command replay** (below) or the
   permission check (target resolved from stored rows; submitted sheets locked →
   `NEEDS_REQUEST` / `REASON_REQUIRED`) and the generic upsert / payment path.
6. A failing op is remembered as `MALFORMED`; the batch continues.

### Command replay (`kind = "action"`)

The device sends the *command* (complete input); the school PC runs the **same domain
function** as the author in Server mode, so every rule is enforced in one place:

| table | payload | replayed through |
|---|---|---|
| `attendance_sheet` | `{class_id, date, submit, marks[]}` | `logic::attendance_write` |
| `marks_sheet` | `{exam_subject_id, submit, entries[]}` | `logic::upsert_marks` |
| `request` | `RequestCmd {create\|leave\|cancel, id, …}` | `logic::request_command` |
| `staff_attendance` | `{cmd: check_in\|check_out, …}` | `staff_check_in/out_logic` (route = transport: LAN push → `lan`, Drive import → `drive`) |
| `message`, `circular_read`, `homework_note`, `report_remark`, `student`, `expense` | `ClientCmd {record_message\|mark_circular_read\|delete_homework_note\|save_report_remark\|create_student\|record_expense, …}` | `logic::run_client_cmd` |
| `payment` (`insert`) | complete payment row | `apply_payment` (always accepted §8.6; re-allocated; voucher posted in the same tx) |

Rows a device creates use ids both sides agree on (device-chosen or deterministic:
`sheet-<class>-<date>`, `msheet-<exam_subject>`, `sat-<staff>-<date>`,
`crd-<circular>-<staff>`, `rmk-<exam>-<student>`, `enr-<student>-<session>`, …).

### School-PC-only commands

69 commands return `SERVER_ONLY` on a joined device (settings, structure, approvals,
staff/devices, money corrections, timetable, exams, calendar, circulars, custom
fields, privacy actions, backups, bulk import, transfers, guardian edits, store sales).

## 3. School PC → devices

* **LAN:** `/v1/sync/snapshot` (join bootstrap, applied all-or-nothing with
  `bootstrap_complete`) and `/v1/sync/pull` (changes + **tombstones** for
  `homework_note`). Clients apply canonical rows with FK enforcement off (rows
  reference the server-only device registry).
* **Drive (down export):** each Drive pass, for every live device whose data moved,
  `exchange/down/<device>.vdown` = seal(`s2c`, AAD `vidya/down/v1\n<device>`,
  `{v:1, device_id, cursor, server_epoch, changes}`) — the device's **complete**
  role-scoped data. A revoked or removed device gets no new export and its old
  `.vdown` is deleted on the next pass. The PWA replaces all local stores from it in one IndexedDB
  transaction (keeping rows with pending local edits); native off-LAN applies it.

## 4. Drive uploads (device → school PC)

`exchange/ops-<device>/<hlc>-dev-v2.vop` = seal(`c2s` of that device, AAD
`vidya/vop/v2\n<device>`, JSON ops), Drive property `vidya_format=device-v2`. The
school PC opens it only with the session key of the live device owning the folder →
authorship proven. Legacy audience bundles, foreign keys, moved files, tampering and
revoked devices are quarantined. Acks: `exchange/acks/<device>.json` = seal(`s2c`,
AAD `ACK\n<device>\n<device>\n0`, `{last_hlc, results[]}`); confirmed → removed,
rejected → Inbox (`list_sync_problems`), never silently dropped.

## 5. Join over Drive (iPhone) — v2

Web invite link `https://<pwa_origin>/#d=<b64url {v:2, invite_id, secret, school_name, school_id}>`
(fragment only; `secret` = 256 random bits, stored in the encrypted DB, wiped on use).

* Request `exchange/joins/<request_id>.vjoin` = `{v:2, invite_id, request_id, device_name, platform, proof}`,
  `proof = HMAC(HMAC-expand(secret,"vidya/join/v2/proof"), "vidya/join-req/v2\n<invite>\n<request>\n<device_name>\n<platform>")`.
* Response `<request_id>.resp.vjoin` = seal(`HMAC(HMAC-expand(secret,"vidya/join/v2/resp"), request_id)`,
  AAD `vidya/join-resp/v2\n<invite>\n<request>\n<school>`, JoinResp).
* Proof verified in constant time; invite consumed atomically (`used_at IS NULL`);
  answered requests deleted. v1 requests refused. Golden vector in
  `vidya_core::seal::tests::join_v2_golden_vector` (checked against Python and WASM).

## 6. Compatibility

No production installs exist (all releases are drafts), so v1 Drive bundles, v1 join
requests and the audience-sealed upload path were **removed**, not kept in parallel.
A device joined on an older build is asked for a PIN on first launch; its Drive
uploads must come from a v2 build.
