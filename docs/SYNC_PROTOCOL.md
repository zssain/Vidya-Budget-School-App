# SYNC_PROTOCOL.md — phones and the office computer

## 1. Principles
- The office computer is the **authority**. A change is final only when the office computer accepts it.
- Phones work offline and queue changes in `outbox`.
- Sync happens only on the local network (school Wi-Fi). No relay, no internet.
- The server re-checks permission and validation for every change using the same services as local commands.
- A phone only ever holds data its user's role may see (PERMISSIONS.md, DTO visibility).

## 2. Hybrid logical clock (vidya-core::hlc)
- Value: `(wall_ms: u64, counter: u32, device: String)`.
- Text form (sortable): `format!("{:013}-{:06}-{}", wall_ms, counter, device)`.
- `now(physical_ms)`: if physical > last.wall → (physical, 0); else (last.wall, last.counter + 1).
- `observe(remote)`: wall = max(last.wall, remote.wall, physical); counter = if walls equal then max(counters) + 1 else 0 for the new max.
- Persist the last value in `meta.hlc_last` inside the same transaction as each write.
- Phones apply `clock_offset_ms` (from `GET /time`) to their physical clock. If the offset is larger than 5 minutes, show a clock warning and create an alert on the server.
- Unit tests: monotonic under a backwards physical clock, ordering across devices, text form sorts the same as the tuple.

## 3. Change envelope (vidya-sync)
```json
{
  "changeId": "dev_01J...:42",
  "deviceId": "dev_01J...",
  "userId": "u_01J...",
  "hlc": "1757923200000-000003-dev_01J...",
  "entity": "receipt",
  "entityId": "01J...",
  "op": "append",
  "fields": { "studentId": "...", "amount": 500, "mode": "Cash", "receiptNo": "T1-0001", "paidOn": "2026-09-15" },
  "baseHlc": null
}
```
| op | Meaning | Used for |
|---|---|---|
| `insert` | New row with all fields | marks rows, attendance day, student (server only) |
| `update_fields` | Only listed fields change | student edits, enrollment edits, settings |
| `append` | Immutable row | receipts, receipt cancellations, TCs |
| `replace_set` | Replace a whole set owned by a parent | attendance marks for one section and date |
| `event` | Audit only, nothing to apply | sign-in, backup, device approval |

Entities: `student`, `enrollment`, `attendance_day`, `marks`, `receipt`, `receipt_cancellation`, `user` (server only), `settings.*` (server only).

## 4. Endpoints
### POST /sync/push
Request: `{ "changes": [Envelope] }` (at most 500, in local order).
Server, per batch in one transaction:
1. Verify the request signature and session (section 7).
2. For each envelope in order:
   - If `changeId` already in `change_log`: mark accepted (idempotent), skip.
   - Build `Actor` from the **server's** user record (role, sections, active). Ignore anything the phone claims.
   - Call `SyncService::apply(actor, envelope)`, which calls the same service method as a local command (for example `FeeService::collect_remote`).
   - Apply clash rules (section 5).
   - Append to `change_log` with a new `seq`.
3. Response: `{ "accepted": ["changeId"], "rejected": [{ "changeId", "kind", "messageKey", "params" }], "serverSeq": 1234 }`.
4. After commit, broadcast `{ "upTo": 1234 }` on the live WebSocket.

### GET /sync/pull?since=<seq>
Returns `{ "changes": [Envelope with seq], "upTo": seq, "more": bool }`, at most 1,000 per call, **filtered for the device user's role**:
- Teacher: only entities in their current sections; never receipts, cancellations, fee fields, users, change log, settings except exams, subjects, grade scale, school name and address.
- Accountant: students, enrollments (with fees), receipts, cancellations, fee plans, school details; never attendance, marks, users.
- Principal phone: as accountant plus attendance and marks for all sections; never users' password hashes.
Fields not allowed for the role are removed from `fields` before sending. If a change is entirely invisible, it is skipped.

### GET /sync/snapshot?page=N
First sync, after `needs_resync`, or when the phone is more than 30 days behind. Returns role-filtered table pages (500 rows each) plus the `seq` the snapshot corresponds to. Phone replaces its local data in one transaction after all pages arrive, then continues with `pull?since=<that seq>`.

### GET /sync/live (WebSocket)
Server sends `{ "upTo": seq }` after each commit. Phone pulls when `upTo` > its `last_pull_seq`. Ping every 25 seconds; phone reconnects with backoff (1, 2, 5, 10, 30 s). Fallback polling every 30 seconds.

## 5. Clash rules
| Situation | Rule |
|---|---|
| Two devices change different fields of the same record | Both kept |
| Two devices change the same field | Higher HLC wins. Loser written to `field_history`. Principal can undo from activity |
| Same section and date attendance saved on two devices | Whole set: higher HLC wins; alert `attendance_replaced` for principal |
| Same mark cell entered on two devices | Same-field rule |
| Receipts | Always appended. Numbers never clash because each device has its own prefix |
| Payments exceed session fee after applying receipts | Accept, then alert `overpayment` for principal and accountant listing the receipts |
| Receipt cancellation | Principal only. Append; if already cancelled, idempotent |
| New admission | Only via `POST /students` while online. Never through push. Phone button disabled offline |
| Teacher change for a section no longer assigned | Rejected (`permission`), phone rolls back and shows it |
| Change for a student who left before the change's HLC | Attendance and marks rejected (`validation.student_left`) |
| Receipt for a student with zero balance at apply time | Accept (money was received) and alert `overpayment` |
| User switched off or locked | All changes rejected (`auth`); phone signs the user out |

## 6. Phone side (vidya-client)
- Local write (client mode): apply to local tables + insert `outbox` row, same transaction.
- Sync cycle: push all unsent outbox rows in order → handle accepted (set `acked_at`) and rejected (roll back locally from the last acknowledged state, keep a `RejectedChangeDto` for the user) → pull until `more` is false → apply pulled changes (skip changes whose `changeId` exists locally) → update `last_pull_seq`.
- Rollback approach: rejected changes are undone by re-fetching the affected entity from the server (`pull` returns the server's current row for rejected entity ids in `rejectedCurrent`).
- Triggers: after every local save (debounced 500 ms), app foreground, WebSocket `upTo`, network change to Wi-Fi, manual "Sync now".
- Outbox rows acknowledged more than 7 days ago are deleted.

## 7. Transport security
- HTTPS with the server's self-signed certificate. The phone pins the SHA-256 certificate fingerprint learned at approval. Any other certificate is refused.
- Before approval, the phone accepts the certificate only for the approval flow and shows the 6-digit match code computed from that certificate.
- Match code: `SHA-256(fingerprint_bytes || device_public_key_bytes)`, take the first 3 bytes as a big-endian integer, `% 1_000_000`, zero-pad to 6 digits, show as "482 913".
- Request signature string: `METHOD\nPATH_WITH_QUERY\nX-Vidya-Time\nX-Vidya-Nonce\nhex(SHA-256(body))`, signed with the device's Ed25519 key.
- Server rejects: unknown or revoked device (410 for revoked), time more than 5 minutes from server time, nonce already seen in the last 10 minutes (`used_request_nonces`), bad signature.
- Only private and link-local source IPs are accepted.
- Rate limits per device: sign-in 10/minute; other routes 120/minute.

## 8. Discovery
- mDNS service type `_vidya._tcp.local.`, instance name `Vidya-<schoolcode>`, port 47631, TXT: `school=<code>`, `fp=<first 16 hex of fingerprint>`, `v=1`.
- UDP fallback on port 47632: phone broadcasts `VIDYA-DISCOVER <schoolcode>`; server replies `VIDYA-HERE <schoolcode> <port> <fp16>` to the sender only. Wrong school code → no reply.
- Phone tries mDNS for 4 seconds, then UDP broadcast 3 times at 1-second intervals, then shows the help screen.
- After approval the phone remembers the last address and tries it first.
