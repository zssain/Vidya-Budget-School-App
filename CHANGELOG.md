# Changelog

All notable changes to Vidya Budget School are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-09-24

First public release of Vidya Budget School — offline-first school-management
software for small Indian schools (Principal, Accountant and Teacher roles) on
Windows, macOS and Android, in English and Hindi.

### Added

- **Activation and setup.** One-time, perpetual licence activation from an
  activation code (needs the internet once); guided setup wizard for school,
  academic session, terms and classes, and the Principal account. Setup resumes if
  interrupted.
- **Security unlock.** Per-user PIN (4–6 digits) with unlock, user switch and
  auto-lock; a recovery key shown once at setup for restore and PC transfer.
- **Staff and access.** Invite staff by link, 8-character code or QR; single-use
  invitations; suspend and remove staff; per-role permissions (Principal,
  Accountant, Teacher) enforced everywhere; per-device receipt and admission number
  series so numbers never clash offline.
- **Students and admissions.** Students list, profile with enrollment history and
  attendance, new admission (official numbers on the server, provisional numbers
  offline), section transfer that preserves history, and marking a student as left.
- **CSV import and export.** Import students with a dry-run preview before committing;
  scoped, formula-safe CSV exports of students, dues and the day book.
- **Attendance.** Phone attendance entry and a desktop register (day and month
  views); submitted sheets lock; corrections go through an approval request.
- **Exams and marks.** Exams, marks entry (desktop grid and phone list), per-subject
  locking on submit, grade scale, and printable report cards (A4, whole-class batch,
  English and Hindi).
- **Fees.** Fee structure, dues, fee collection, receipts (with amount in words),
  approved reversals, advance credit, and a per-student ledger.
- **Day book and reports.** Day book with mode totals; reports for admissions, fee
  collection, dues, exam results and attendance; and an audit-log viewer.
- **Approvals.** Requests inbox for the Principal covering attendance corrections,
  marks corrections, student-detail edits, payment reversals, access changes and
  device replacements, with approve, reject and return; plus My Requests and Inbox.
- **Sync — three routes, always offline-first.** The school server runs on the LAN
  (with mDNS discovery); phones and PCs also reach it over the internet through the
  Vidya relay; when the server is off, devices exchange encrypted change bundles
  through Google Drive; with no network at all, work is queued on the device and sent
  later. Nothing is ever silently overwritten. Conflicts are flagged for the
  Principal; review flags catch overpayments, possible duplicate payments and changes
  from revoked authors — and real money is never lost.
- **Server fencing.** After a restore or licence transfer the server epoch advances,
  old machines are fenced off and stop acting as the school server, and devices are
  asked to re-join.
- **Backups and restore.** Daily verified backups (06:00 and on quit) kept locally
  and copied to Google Drive, with 30-daily / 12-monthly retention; a USB copy
  workflow; and restore-to-a-new-PC using the recovery key.
- **New session rollover.** Promote students to the next class, carry unpaid balances
  forward as a "Previous balance" due, and generate new-session fees, all in one
  audited step; the previous session becomes read-only.
- **Settings and storage.** School, session and terms, classes and subjects, grade
  scale, appearance/accent, language, printing, security, Google Drive and connection
  settings, a storage screen, and About (showing the audit chain fingerprint).
- **Full Hindi.** Complete English and Hindi throughout the app — interface,
  receipts, report cards and reports — including the Devanagari font swap and Hindi
  logo lockups.
- **Indian formatting.** Rupee amounts with lakh grouping (₹6,84,200), full-form
  dates, April–March sessions shown as "2026–27", and 10-digit mobiles.

### Security

- **Encrypted everywhere.** One encrypted SQLCipher database per device; the database
  key is held in the OS keychain on desktop and wrapped by the Android Keystore on
  Android. All data is encrypted before it ever reaches Google Drive or the relay, so
  Google, the relay operator and the company can never read school data.
- **Append-only audit chain.** Every important change is recorded in a hash-chained,
  append-only audit log that the database itself refuses to edit or delete (as it does
  for payments and reversals). The chain is verified daily and before every backup,
  and its fingerprint is stored with each backup. Documented honestly: someone with
  full control of the PC and keys could rebuild the chain, but backups and Drive copies
  make that detectable.
- **PIN protection.** PINs are hashed with Argon2id and are a local unlock only, never
  a network credential; repeated wrong attempts trigger an increasing lockout.
- **Pinned, sealed networking.** The school server uses a self-signed certificate that
  clients pin by fingerprint; devices authenticate with per-device tokens stored only
  as hashes and compared in constant time; relay traffic is end-to-end sealed
  (ChaCha20-Poly1305) so the relay sees only encrypted envelopes, with replay
  protection.
- **Locked-down Google Drive.** Backups are never shared; each staff member can write
  only to their own folder; per-class keys keep teachers from reading other classes.
- **Recovery key.** A 30-character recovery key derives the backup key (Argon2id) and
  is required to restore backups or move the school to a new PC.
- **Safe logs.** Logs never contain tokens, keys, PINs, OAuth tokens, recovery keys or
  full mobile numbers.
- **Hardening.** Full role-matrix testing across every command and endpoint for every
  role and state, minimal app capabilities and a strict content-security policy, and
  removal of all development-only seed/gallery/fixtures and the dev licence key from
  release builds.

### Performance

- Runs on low-end hardware (a 4 GB RAM Windows PC and a 2 GB RAM Android phone) with
  measured budgets for cold start, dashboard load with 1,500 students, attendance
  taps, search and sync.
- **Size gates** enforced in CI: each installer is within the download and installed
  limits (≤ 40 MB download, ≤ 50 MB installed), with `ring` as the sole TLS provider,
  tree-shaken JavaScript and subset fonts (keeping the Newsreader optical-size axis).
- Reliability work: a panic handler with rotated logs and a calm restart screen, a
  daily integrity and audit-chain check that blocks writes and guides to Recover on
  failure, atomic writes that survive power loss, and a large-scale multi-client sync
  soak test.

### Notes

- **Unsigned by default.** When code-signing secrets are not supplied, the Windows,
  macOS and Android builds are produced **unsigned** and are clearly labelled as such;
  see `docs/INSTALL.md` for the first-run steps on each platform. Supplying the signing
  secrets produces signed (and, on macOS, notarized) builds.
- **Not counted in the app size** (and disclosed): the operating system's web engine
  (WebView2 on Windows, Android System WebView), your school data, backups and logs.
