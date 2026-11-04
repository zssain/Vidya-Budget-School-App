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
   `NEEDS_REQUEST` / `REASON_REQUIRED`) and the generic upsert / payment path. A
   generic `update` to a row with a `version` column must carry `base_version`
   (missing → `MALFORMED`); `base_version < current` with a differing field →
   `CONFLICT` + a `conflict` row for `list_conflicts`/`resolve_conflict`.
6. A failing op is remembered as `MALFORMED`; the batch continues.

### Command replay (`kind = "action"`)

The device sends the *command* (complete input); the school PC runs the **same domain
function** as the author in Server mode, so every rule is enforced in one place:

| table | payload | replayed through |
|---|---|---|
| `attendance_sheet` | `{class_id, date, submit, marks[]}` | `logic::attendance_write` |
| `marks_sheet` | `{exam_subject_id, submit, entries[]}` | `logic::upsert_marks` |
| `request` | `RequestCmd {create\|leave\|cancel, id, …}` | `logic::request_command` |
| `request` | `RequestCmd {decide, id, decision, note}` (W1) | `logic::decide_request_with` (`from_device = true`) — see §2a |
| `staff_attendance` | `{cmd: check_in\|check_out, …}` | `staff_check_in/out_logic` (route = transport: LAN push → `lan`, Drive import → `drive`) |
| `staff_attendance` | `{cmd: accept_away_checkin\|reject_away_checkin, id}` (W1) | `accept/reject_away_checkin_logic` — see §2a |
| `message`, `circular_read`, `homework_note`, `report_remark`, `student`, `expense`, `store_sale` | `ClientCmd {record_message\|mark_circular_read\|delete_homework_note\|save_report_remark\|create_student\|record_expense\|record_store_sale, …}` | `logic::run_client_cmd` (store sale → `record_store_sale_with`, §2b) |
| `payment` (`insert`) | complete payment row | `apply_payment` (always accepted §8.6; re-allocated; voucher posted in the same tx) |

Rows a device creates use ids both sides agree on (device-chosen or deterministic:
`sheet-<class>-<date>`, `msheet-<exam_subject>`, `sat-<staff>-<date>`,
`crd-<circular>-<staff>`, `rmk-<exam>-<student>`, `enr-<student>-<session>`, …).

Rows a decision or sale touches keep ids both sides agree on: the request id, the
`staff_attendance` row id, the device-chosen `sale-<uuid>` and its stock moves
`mv-<sale>-<line>`.

### 2a. Principal decisions from a joined device (W1)

`decide_request` (approve / reject / return) and `accept_away_checkin` /
`reject_away_checkin` run on a joined PC/phone signed in as the Principal:

* **On the device:** the usual checks (ApproveRequest / ManageStaffHr, `hr` module,
  still pending locally, off-server policy below), then ONE transaction: the local
  row takes the decision (`request.sync_state='on_device'`), an audit row, and ONE
  `action` op carrying the command. The **effects** (leave record + leave days,
  corrected attendance mark, notification) are NOT written locally — only the school
  PC writes them, and they arrive by pull; nothing shows as "applied" before it is.
* **On the school PC:** `apply_client_op` binds the op to the authenticated, live
  (unrevoked) device and its staff; `apply_op` refuses a suspended/removed author
  (`flagged revoked_author`); the replay then runs the SAME function as the author
  with the author's CURRENT role from the staff table (a teacher / accountant →
  `FORBIDDEN`). The decision UPDATE is guarded `WHERE status='pending'`
  (`'away_pending'` for check-ins) **inside** the decision transaction, before any
  effect: a request decided meanwhile (on the school PC or another device) →
  `ALREADY_DECIDED`, the transaction rolls back, nothing is applied twice. The
  device gets the rejection as an Inbox item (`error.ALREADY_DECIDED`) and the pull
  replaces its optimistic local decision with the canonical one. Audit rows carry
  the Principal and the device (`approve_request` / `reject_request` /
  `return_request` / `accept_away_checkin` / `reject_away_checkin`).
* **Idempotency:** a re-delivered op (LAN retry, Drive copy) returns its stored
  result; a *different* op deciding the same request is `ALREADY_DECIDED`.
* **Off-server policy** (`logic::decision_allowed_off_server`, enforced on the device
  AND on replay): every reject / return syncs; **approving** a `payment_reversal`
  (a money correction that posts a reversal voucher), an `access_change` (staff
  permissions) or a `device_replacement` (device registry) stays on the school PC
  → `SERVER_ONLY`.

### 2b. Store sales from a joined device (W1)

Store is an optional module, **off by default**. Module switches live in
`module_setting`, which is not a synced table, so the school PC publishes them into
`school.settings_json.modules` (`{key: bool}`; on every `set_module` and at
startup, written only when changed — `modules::publish_to_school`); a joined device
gates its commands on that published set (`modules::enabled_set` / `is_enabled`
read it on a client; a school row without it → the local defaults).

* **On the device:** `record_store_sale` (accountant / Principal, store on as last
  published) checks the cart against the device's copy of the stock (a sale beyond
  it is blocked before any money is taken), takes a receipt number from the
  device's OWN series (`S-<series>-NNNN`), writes the sale and its stock moves
  `on_device` (no voucher) and queues ONE `store_sale` / `action` op:
  `ClientCmd::RecordStoreSale {id, today, receipt_no, input}` where every line
  carries the `price_paise` actually charged.
* **On the school PC (`record_store_sale_with`, replay):** module gate (store off →
  `MODULE_OFF`, nothing written) and RecordStoreSale permission first. Then, because
  the money was already taken (§8.6), the sale is **always** recorded — exactly once:
  * idempotent by the device-chosen id: a re-delivered op returns its stored result;
    a different op carrying an already-recorded sale id returns that sale and writes
    nothing (one sale, one voucher, one stock decrement);
  * recorded at the device's prices (`vidya_core::store::replay_sale` checks only the
    shape: lines, `qty > 0`, `price ≥ 0`, total > 0); the Store-income voucher is
    posted in the same transaction;
  * stock is decremented and **may go negative**; any line that took an item below
    zero raises a `store_oversold` review flag (`{items: [{item_id, stock_after}]}`)
    for the Principal — never a refusal;
  * a price that differs from the school PC's, a since-deactivated item, or a
    receipt number already used (then a fresh number from the same series is
    assigned) raises a `store_sale_review` flag (migration 0035 widens
    `review_flag.kind`);
  * an op whose `record_id` is not the sale id → `MALFORMED`; an unknown item →
    refused (`NOT_FOUND`: no such stock exists).
* `stock_adjust` (Principal) accepts any **increase** even if the result is still
  negative, so a purchase that only partly covers an oversell can be recorded.

### School-PC-only commands

66 command gates return `SERVER_ONLY` on a joined device (settings, structure,
staff/devices, money corrections — direct reversal, attendance correction, and
approving a payment reversal from a device —, timetable, exams, calendar,
circulars, custom fields, privacy actions, backups, bulk import, transfers,
guardian edits, store catalogue + stock adjustments, the Principal's manual
staff-attendance entry, and conflict / review-flag resolution, whose tables exist
only on the school PC).

## 3. School PC → devices

* **LAN:** `/v1/sync/snapshot` (join bootstrap) = JSON `{manifest, changes}`, manifest
  `{v:1, snapshot_id, server_epoch, cursor, row_count, tables, digest}` (digest =
  SHA-256(`vidya/snapshot/v1\n<id>\n<epoch>\n<cursor>\n` ‖ sorted rows
  `table\0record_id\0canonical-json\n`)). The device verifies counts + digest + epoch
  before writing, then applies all-or-nothing with `bootstrap_complete`; a mismatch
  is refused and refetched by the sync loop and `/v1/sync/pull` (changes + **tombstones** for
  every synced table — see Deletes below). Clients apply canonical rows with FK enforcement off (rows
  reference the server-only device registry).
* **Change feed (P21):** on the school PC, AFTER INSERT/UPDATE/DELETE triggers on every
  `scope::SYNC_TABLES` table append a `touch`/`delete` row to `op_log`
  (`db::install_change_feed_triggers`; off on joined devices). Pull reads the current
  row and scopes it per device, so ANY school-PC write reaches LAN devices — before,
  only writes made through `write::append_op` did (e.g. assigning a substitute never
  reached a phone). A bootstrap sets the device's pull cursor to `manifest.cursor`, so
  pull continues from the snapshot instead of replaying the whole feed.
* **Deletes (P21 D1):** every school-PC delete on a `SYNC_TABLES` table is a feed
  `delete` row. Pull sends it as a tombstone (table + id only, to every device); within
  one page only a row's final state is sent, so delete→re-insert can't apply out of
  order. The down export carries `deletes` (up to 5000 most recent, excluding rows
  that exist again) because the native client upserts exports; the PWA replaces its
  store from the export and ignores them. A device never deletes a row that still has
  an unsent outbox op (`engine::apply_tombstones`) — the school PC answers that op.
  If the list was cut, `deletes_from` is the newest deletion left out; a device whose
  last applied export (`down_cursor`) is older reconciles: it removes every synced row
  the export doesn't contain, in the same transaction as the export's rows — but only
  with an empty outbox (a sheet op covers rows it doesn't name); otherwise it applies
  the rows + listed deletions, keeps `down_cursor`, and reconciles on a later pass.
* **Duty rosters:** a teacher covering a class through a current/upcoming attendance
  grant (substitution with attendance, approved attendance duty; from yesterday UTC
  on) receives that class's row, its students (address withheld), enrollments and
  only the covered days' sheets/marks — in the snapshot, the down export, and in the
  pull that delivers the grant itself (`scope::duty_grants` / `duty_roster`).
* **Drive (down export):** each Drive pass, for every live device whose data moved,
  `exchange/down/<device>.vdown` = seal(`s2c`, AAD `vidya/down/v1\n<device>`,
  `{v:1, device_id, cursor, server_epoch, changes, manifest}`, manifest as for the LAN
  snapshot; the native client requires and verifies it) — the device's **complete**
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
