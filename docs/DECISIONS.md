# DECISIONS.md — decisions already made

Each decision is final unless the developer changes it. Agents must not reopen these. To propose a change, stop and ask.

| # | Decision | Why | Rejected alternatives |
|---|---|---|---|
| D1 | Tauri 2 for Windows, macOS and Android from one codebase | Small apps (uses system WebView), Rust backend, mobile support | Electron (80–150 MB, no Android), Flutter (rewrite UI, larger), React Native (no desktop server story) |
| D2 | Frontend in plain JavaScript modules with Vite, no framework | Prototype is plain JS; small; easy to maintain | React, Vue, Svelte |
| D3 | Rust workspace with separate crates per responsibility (ARCHITECTURE.md) | Testable pure core; one path for every action; phone and PC share rules | Single crate |
| D4 | SQLite via rusqlite with SQLCipher | Reliable single-file DB, encryption at rest | Plain SQLite (not encrypted), sled, Postgres |
| D5 | Office computer is the only server; phones sync only on school Wi-Fi | No cloud, no running costs, data never leaves school | Cloud sync, relay server |
| D6 | No QR codes; phone finds server by school code via mDNS, UDP broadcast fallback | Simpler for staff | QR pairing, typing IP addresses |
| D7 | New phone needs principal approval with a matching 6-digit code | Stops strangers on school Wi-Fi and fake servers | Password only |
| D8 | HTTPS with self-signed certificate pinned on the phone after approval | Encrypted LAN traffic without internet certificates | Plain HTTP, public certificates |
| D9 | Change log + hybrid logical clock + per-field last-writer-wins; receipts append-only | Works offline, predictable clashes, accounting safety | Whole-database copy, CRDT library |
| D10 | Admission numbers only allocated by the office computer; receipt series per device | Admission numbers must be continuous; receipts can be per book | Per-device admission series |
| D11 | Permissions enforced in Rust services; teachers' devices never receive fee data | Security cannot depend on UI | UI-only hiding |
| D12 | Argon2id for login passwords | Current recommendation for password storage | PBKDF2 (used only in the prototype) |
| D13 | Ed25519-signed activation codes, bound to the office computer's device ID, verified offline | One-time payment, no internet | Online activation server, license files without signatures |
| D14 | Backups encrypted with a school backup password (Argon2id + XChaCha20-Poly1305) | Provider and Google cannot read backups | Unencrypted backups, provider-held keys |
| D15 | Google Drive backup uses only the `drive.file` scope | App sees only its own files; lighter Google verification | Full Drive scope |
| D16 | Username = first name + @ + school code from the license | Easy to remember, unique per school | Email addresses, numeric IDs |
| D17 | Money as integer rupees | No rounding errors | Floating point, paise |
| D18 | UUID v7 text IDs for rows | Rows can be created offline on any device | Autoincrement IDs |
| D19 | Academic sessions are first-class: enrollments, fee plans, exams, receipts belong to a session | Year change without losing history | Overwrite classes each year |
| D20 | Printable documents are HTML templates; PDF method chosen by spike P5.1 with proof that Hindi conjuncts render correctly | Hindi text needs complex shaping | Choosing a PDF library without proof |
| D21 | Development on a MacBook; Windows built and tested in GitHub Actions; manual Windows testing in a VM and on a real PC before release | Developer has no Windows PC | Requiring a Windows PC |
| D22 | Provider Tool is a separate macOS Tauri app, never published | Protects the signing key | Web admin panel, command-line only |
| D23 | English and Hindi from day one via translation keys | Market need | English only, adding Hindi later by search-and-replace |
| D24 | No automatic internet updates; new installers and Play Store updates only | Offline product, simpler, safer | Built-in updater |

## Decision log (append new decisions here when the developer approves them)
| # | Date | Decision | Approved by |
|---|---|---|---|
