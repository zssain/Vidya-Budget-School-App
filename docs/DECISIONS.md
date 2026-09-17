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
| D14 | Backups encrypted with a school backup password (Argon2id + XChaCha20-Poly1305) | The provider and storage administrators cannot read backups | Unencrypted backups, provider-held keys |
| D15 | Superseded by D30: Google Drive backup used only the `drive.file` scope | Replaced by local backup destinations that need no internet | Full Drive scope |
| D16 | Username = first name + @ + school code from the license | Easy to remember, unique per school | Email addresses, numeric IDs |
| D17 | Money as integer rupees | No rounding errors | Floating point, paise |
| D18 | UUID v7 text IDs for rows | Rows can be created offline on any device | Autoincrement IDs |
| D19 | Academic sessions are first-class: enrollments, fee plans, exams, receipts belong to a session | Year change without losing history | Overwrite classes each year |
| D20 | Printable documents are HTML templates; PDF method chosen by spike P5.1 with proof that Hindi conjuncts render correctly | Hindi text needs complex shaping | Choosing a PDF library without proof |
| D21 | Development on a MacBook; Windows built and tested in GitHub Actions; manual Windows testing in a VM and on a real PC before release | Developer has no Windows PC | Requiring a Windows PC |
| D22 | Provider Tool is a separate macOS Tauri app, never published | Protects the signing key | Web admin panel, command-line only |
| D23 | English and Hindi from day one via translation keys | Market need | English only, adding Hindi later by search-and-replace |
| D24 | No automatic internet updates; new installers and Play Store updates only | Offline product, simpler, safer | Built-in updater |
| D27 | React frontend. JSX in plain JavaScript (no TypeScript). Only React and ReactDOM at runtime; no router, state management, form or CSS-in-JS libraries. The router, modal, toast and data hooks are small in-house modules. | Fewer dependencies, a smaller bundle, and a pattern that is easy to review | Additional frontend frameworks and libraries |
| D28 | 30 MB budget. Every shipped download (Windows installer, each macOS `.dmg`, Android APK and AAB) is at most 30 MB. Every installed app is at most 30 MB on disk, excluding school data and the system web engine (WebView2, WKWebView, Android WebView). There is a warning at 25 MB, and CI enforces the limit from P1.1. macOS ships separate Apple Silicon and Intel builds; a universal build is used only if both its download and installed size fit. | Keeps Vidya practical on low-cost computers and phones | Large bundled runtimes and unchecked growth |
| D29 | The licensed office computer is the only server. The LAN server, discovery responders, phone approvals and sync endpoints start only with a `ServerPermit`, which is created when the stored license verifies for this computer's Device ID. Phones are always clients, and mobile builds do not compile server code. There is no cloud or relay: sync works only on the school's Wi-Fi or LAN. | Prevents unlicensed, moved or mobile installations from serving school data | Cloud sync, relays and unlicensed local servers |
| D30 | Local backup destinations replace Google Drive. Encrypted backup files are copied to destinations chosen by the principal: a folder on another internal drive, an external disk, a pen drive, or a folder on the school network. Vidya needs no internet. D15 is **superseded by D30**. | Keeps backups offline and under the school's control | Cloud backup and external authorization |
| D31 | Backups from the office computer. The action `backup.run` ("Back up now" and "Sync backup drives") is allowed for the principal and accountant, but only in sessions on the office computer, and only when automatic backups are switched on (the key is stored, so no password is needed). The principal can switch staff backups off. Configuring destinations, changing the backup password and restoring stay with `backup.manage` (principal). | Lets office staff perform routine backups without granting backup administration | Principal-only routine backup execution |

## Decision log (append new decisions here when the developer approves them)
| # | Date | Decision | Approved by |
|---|---|---|---|
