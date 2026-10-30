# Phase 21 handoff — repairs (security, multi-device sync, iPhone data path)

Branch `v2/p21-repairs` (from `v2/p20-live-sync` @ `ec7808b`). Not pushed, not merged.
Spec: `~/Downloads/Vidya_Fix_and_Owner_Portal_Implementation_Prompt.md`. Owner decision
(30 Oct 2026): app repairs first; the Razorpay/Supabase owner portal is deferred.

Read first: `docs/implementation/repair-status.md` (every issue, status, evidence),
`docs/implementation/sync-protocol-v2.md` (how devices talk to the school PC now),
`docs/implementation/write-sync-matrix.md` (which writes sync, who sees them).

## What changed (headline)

* **Security:** joined devices need their own PIN and auto-lock (5 min); 54 unguarded
  commands guarded; the school PC binds every op to the authenticated device and
  denies unknown ops; Drive uploads are sealed per device (authorship proven); the
  iPhone Drive join no longer exposes keys to anyone who can read the Drive folder.
* **Sync actually works from devices:** attendance, marks, leave/requests, check-in
  (route-honest), messages/absence alerts, circular reads, remarks, notes (+ delete
  tombstones), payments (own receipt series, voucher on the PC), admissions
  (provisional → official), expenses. Everything else is SERVER_ONLY on a device.
* **iPhone app** gets school data via a sealed per-device Drive export and can take
  attendance, check in, apply for leave, send absence alerts, read circulars.
* **Reliability:** all-or-nothing join bootstrap; refused changes shown in the Inbox;
  backups driven by a durable data revision, at most hourly, partial runs retried.
* Latent bugs found and fixed: colliding receipt/admission series on joined PCs; no
  ledger voucher for synced payments; payments impossible on joined PCs (FK); teacher
  phones couldn't enter marks or send absence alerts (scope gaps); principal /
  accountant pulls failed on other devices' payments; the iPhone app had no data source.

## Verify

```
npm run verify                  # 86 JS tests + all gates
npm run test:e2e:pwa            # 4/4 WebKit
cargo test --workspace --locked # 845 Rust tests
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run check:release-clean
```
`npm run test:e2e` desktop fidelity fails identically at the baseline commit (stale,
gitignored pixel baselines) — regenerate on the canonical machine.

## Not done / owner actions

* K1 Android Keystore — needs a dependency decision (Kotlin plugin vs `jni`) and a device.
* Real-hardware runs: two PCs on the LAN, Android join, iPhone join + Drive loop.
* Deploy the PWA at a stable public origin (no Vercel login) → repo variable
  `PWA_ORIGIN` + Google Web OAuth origin. Decide the website domain.
* PWA ports still missing: marks entry, report remarks, accountant collect-fee.
* Independent review of the join-v2 / device-bundle crypto before claiming production
  assurance. Native Hindi/Telugu review of new strings. Code signing.
* Owner portal (spec Phases 10–13): deferred by the owner.
