# Vidya — Build Prompts Book (Edition 2)

All 45 build prompts in one file for reading and printing. **To run a prompt, do not paste from this book:** open Claude Code in your project and type `/run <id>`, which loads the same text from `docs/prompts/` together with the project rules. The one exception is P0.1, which you run from this file because it creates `docs/prompts/` for this edition.

Read `README-FIRST.md` before starting. The agent rules are in `AGENTS.md`; the specification is in `docs/`.

## What Edition 2 changes

| Requirement | How the book delivers it | Where |
|---|---|---|
| React frontend | JSX components with React + ReactDOM only (no router, state or CSS libraries). React escapes text by default; raw HTML injection is banned by lint everywhere | P0.1, P1.1, P1.2, then every screen prompt |
| The licensed PC is the local server | The LAN server, discovery and background sync can only start with a `ServerPermit`, which only `LicenseService` can create when the license is valid for this exact computer. Phones are always clients. Unlicensed, tampered or moved installs never serve | P2.2, P4.2, P7.1, P7.2 |
| Whole app under 30 MB | A hard budget of 30 MB for every download **and** every installed app (school data and the operating system's web engine excluded), checked by `scripts/check-size.mjs` in CI from P1.1 onwards. The Mac ships separate Apple Silicon and Intel builds unless a universal build fits | P0.1, P1.1, P1.3, P2.3, P8.1, P10.1 |
| Sync over Wi-Fi | Phones sync only with the office computer on the same school Wi-Fi or LAN. No cloud, relay, port forwarding or internet service anywhere in Vidya | P7.1–P7.5, P8.3, P8.4 |
| Principal or the person at the PC backs up to a local drive | Encrypted backups sync to backup destinations: a second internal drive, an external disk, a pen drive or a folder on the school network. Anyone signed in **at the office computer** with `backup.run` can press "Back up now"; setting up destinations and restoring stay with the principal. Google Drive is removed | P6.1, P6.2 |

## Global rules for Edition 2 (P0.1 writes these into AGENTS.md)

1. **Size budget.** No shipped file may be larger than 30 MB, and no installed app may take more than 30 MB on disk. A warning is raised at 25 MB. Any prompt that adds a Rust crate or npm package runs `npm run size` (from P1.3) and records the result in `docs/SIZE.md`. If a change crosses the warning line, stop and report.
2. **React rendering.** Use components and JSX text only. `dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `insertAdjacentHTML` and `document.write` are banned everywhere with no exceptions. Event handlers are passed only as React props. No business rules in components.
3. **Server rule.** Only a licensed office computer runs server code. Mobile builds do not compile server crates at all.
4. **Network rule.** Vidya never needs the internet. The only network traffic is between the office computer and approved phones on private addresses.
5. **Backup rule.** Backups are always encrypted files. Plain copies exist only in private temporary folders and are deleted in every code path.

## Contents

**Phase 0 Specification**

- P0.1 — Apply Edition 2 to the specification

**Phase 1 Foundation**

- P1.1 — Workspace, tooling, size gate and CI
- P1.2 — React port of the prototype
- P1.3 — Desktop shell and security settings

**Phase 2 Core and database**

- P2.1 — vidya-core domain rules
- P2.2 — Permission matrix
- P2.3 — Encrypted database and migrations
- P2.4 — Platform secrets, folders and app start
- P2.5 — Services foundation, repositories and change log
- P2.6 — Sign-in and sessions
- P2.7 — Command layer and first real screens

**Phase 3 School features**

- P3.1 — Staff logins and the permission test harness
- P3.2 — Students and admissions
- P3.3 — Fees, receipts and day book
- P3.4 — Attendance
- P3.5 — Marks and report cards
- P3.6 — Settings
- P3.7 — Home screens, reports, activity and Excel export

**Phase 4 Licensing and setup**

- P4.1 — License codes crate
- P4.2 — Device ID, activation, server permit and principal reset
- P4.3 — Provider Tool
- P4.4 — Setup wizard, login slips and recovery sheet
- P4.5 — Student import from Excel

**Phase 5 Documents**

- P5.1 — Printing and Hindi spike (decide how documents are made)
- P5.2 — Documents: receipts, report cards, registers, TC

**Phase 6 Backups and year end**

- P6.1 — Encrypted backups, automatic backups and restore
- P6.2 — Backup drives: sync backups to local and removable drives
- P6.3 — New academic session

**Phase 7 Local network and sync**

- P7.1 — Licensed LAN server and background running
- P7.2 — Discovery, firewall and connection check
- P7.3 — Phone approval, signed requests and device management
- P7.4 — Sync engine core
- P7.5 — Sync endpoints and live updates

**Phase 8 Android**

- P8.1 — Android project and client-only build
- P8.2 — Android secure storage and local database
- P8.3 — Phone discovery, sign-in and approval
- P8.4 — Phone sync, offline work and removal
- P8.5 — Phone screens, PDFs and privacy protections

**Phase 9 Language**

- P9.1 — Full Hindi

**Phase 10 Release**

- P10.1 — Size gate and Windows installer
- P10.2 — Mac signing and notarization
- P10.3 — Android signing and release pipeline
- P10.4 — Security and privacy review
- P10.5 — Load test and real-device testing
- P10.6 — Play Store listing and review access
---

## P0.1 — Apply Edition 2 to the specification

| | |
|---|---|
| Phase | 0 Specification |
| Depends on | nothing (run on the starter kit before P1.1) |
| Size | medium (documents only) |
| Runs on | MacBook |
| Read first | AGENTS.md, src/AGENTS.md, src-tauri/AGENTS.md, crates/AGENTS.md, every file in docs/ |

### Before you start (developer)
1. Do steps 1–4 of P1.1 "Before you start" (private repository, starter kit copied, first commit pushed).
2. Save this book as `docs/PROMPTS_BOOK.md`, replacing any older book, and commit.
3. Start Claude Code in `~/code/vidya` and type: `Run prompt P0.1 from docs/PROMPTS_BOOK.md`.

### Goal
Every document, AGENTS.md file and prompt file describes Edition 2, so later prompts, the permission document test and the API drift check all agree. No code is written in this prompt.

### Tasks

#### Task 1 — Prompt files
1. Split `docs/PROMPTS_BOOK.md` into `docs/prompts/<ID>.md` (for example `P1.2.md`): each file holds one `## P…` section up to the next `---` line.
2. Delete prompt files whose ids are no longer in the book.
3. Read `.claude/commands/run.md` and confirm it loads `docs/prompts/<id>.md`. Change only the path if it differs.
**Check:** 45 prompt files exist, and `P6.2.md` is titled "Backup drives".

#### Task 2 — Decision log (`docs/DECISIONS.md`)
Add these decisions. Keep D25 and D26 reserved for P5.1 and P10.2. If 27–31 are already taken, use the next free numbers and update every reference in the book and prompt files.
- **D27 React frontend.** JSX in plain JavaScript (no TypeScript). Only React and ReactDOM at runtime; no router, state management, form or CSS-in-JS libraries. The router, modal, toast and data hooks are small in-house modules. Reason: fewer dependencies, a smaller bundle, and a pattern that is easy to review.
- **D28 30 MB budget.** Every shipped download (Windows installer, each macOS `.dmg`, Android APK and AAB) is at most 30 MB. Every installed app is at most 30 MB on disk, excluding school data and the system web engine (WebView2, WKWebView, Android WebView). There is a warning at 25 MB, and CI enforces the limit from P1.1. macOS ships separate Apple Silicon and Intel builds; a universal build is used only if both its download and installed size fit.
- **D29 The licensed office computer is the only server.** The LAN server, discovery responders, phone approvals and sync endpoints start only with a `ServerPermit`, which is created when the stored license verifies for this computer's Device ID. Phones are always clients, and mobile builds do not compile server code. There is no cloud or relay: sync works only on the school's Wi-Fi or LAN.
- **D30 Local backup destinations replace Google Drive.** Encrypted backup files are copied to destinations chosen by the principal: a folder on another internal drive, an external disk, a pen drive, or a folder on the school network. Vidya needs no internet. D15 is **superseded by D30**.
- **D31 Backups from the office computer.** The action `backup.run` ("Back up now" and "Sync backup drives") is allowed for the principal and accountant, but only in sessions on the office computer, and only when automatic backups are switched on (the key is stored, so no password is needed). The principal can switch staff backups off. Configuring destinations, changing the backup password and restoring stay with `backup.manage` (principal).

#### Task 3 — AGENTS.md files
- Root `AGENTS.md`: add the five "Global rules for Edition 2" from the top of this book, word for word, under a heading "Edition 2 rules".
- `src/AGENTS.md`: replace any rules about `html`, `render`, `delegate` or `SafeHtml` with the React rules from Task 4. File extensions for components and views are `.jsx`.
- `src-tauri/AGENTS.md`: add "Server code runs only with a ServerPermit (D29). Never add a way to start the server that bypasses LicenseService."
- `crates/AGENTS.md`: add "Every new dependency: check size impact with `cargo bloat` or the size script, and record it in docs/SIZE.md."

#### Task 4 — `docs/UI_GUIDE.md`
Replace the "Safe HTML" section with "React rendering rules":
1. Text is rendered as JSX children or attribute values. React escapes it.
2. Never use `dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `insertAdjacentHTML` or `document.write`. ESLint blocks them with no exception list.
3. Never build URLs for `href` or `src` from user data, except `tel:` links made by `src/core/links.js`, which only accepts 10 digits.
4. Use class names from the CSS files. Use inline `style={{…}}` objects only for computed values (for example progress width). Never write style strings.
5. Components get data from hooks in `src/core/useCommand.js`. They never call `invoke` directly and never compute fees, grades, percentages or permissions.
6. Every visible string comes from `t('key')` (from the `useT()` hook).
7. Printing renders a document component into the print root through `usePrint()`. It never builds HTML strings.
Change every code example in the guide from template strings to JSX. Keep colours, spacing, wording rules, translation rules and print rules unchanged.

#### Task 5 — `docs/ARCHITECTURE.md`
- Frontend section: the React tree (`main.jsx` → `main-desktop.jsx` or `main-mobile.jsx` → `AppProviders` (I18n, Session, Router, Toast, Modal, Print) → Shell → View). `src/api/commands.js` is still the only bridge to Rust.
- Server section: D29. Add the `ServerPermit` path (`LicenseService::server_permit()` → `vidya_server::start(permit, …)`).
- Backup section: D30 and D31. Remove Google Drive, OAuth and HTTP uploads from diagrams and crate descriptions. `vidya-backup` is now "backup file format, local automatic backups, backup destinations, restore".
- New section "Size budget" that points to D28 and `docs/SIZE.md`.

#### Task 6 — `docs/PERMISSIONS.md`
1. Add a row `backup.run` directly after `backup.manage`: principal `yes`, accountant `yes`, teacher `no`.
2. Add a section after the table, exactly like this (the P2.2 test parses it):
   ```
   ## Office computer only
   These actions are refused for requests from phones, whatever the role.
   - `backup.run`
   - `backup.manage`
   - `session.change`
   - `students.import`
   - `settings.edit`
   - `users.manage`
   - `devices.approve`
   - `devices.manage`
   ```

#### Task 7 — `docs/API.md`
- Backup section: remove `drive_connect`, `drive_disconnect`, `drive_list_backups` and any other Drive commands. Add, all **D**: `run_backup_now` (backup.run), `list_removable_drives` (backup.run), `list_backup_destinations` (backup.run), `add_backup_destination` (backup.manage), `remove_backup_destination` (backup.manage), `sync_backup_destinations` (backup.run), `list_destination_backups` (backup.manage), `set_staff_backup_allowed` (backup.manage). Describe inputs and DTOs in the same style as the existing commands (see P6.2 for fields).
- `AppStatusDto`: add `serverAllowed: boolean`.
- LAN `GET /api/v1/health`: response `{ schoolCode, version, fp, licensed: true }`. A server without a permit does not exist, so `licensed` is always true. It is there so phones can refuse anything else.

#### Task 8 — `docs/BACKUP_FORMAT.md`
Remove Google Drive details (appProperties, upload retention). Add a section "Backup destinations":
- Folder layout `<destination root>/Vidya Backups/<schoolcode>/`.
- Files are written as `<name>.vidyabak.partial`, flushed, renamed, then read back and checked against SHA-256.
- `manifest.json` in the same folder: `{ version: 1, schoolCode, files: [{ name, kind, createdAt, size, sha256, schemaVersion, appVersion }] }`. No names of people and nothing decrypted.
- Retention at each destination is the same as for local backups (30 daily, 12 monthly, safety copies 5). Files not listed in the manifest are never deleted.

#### Task 9 — `docs/PLATFORMS.md`
- Add to the `Platform` trait: `fn volume_info(&self, path: &Path) -> Result<VolumeInfo, PlatformError>` with `VolumeInfo { volume_id: String, label: String, removable: bool, network: bool, physical_disk_id: Option<String>, free_bytes: u64, total_bytes: u64 }`, and `fn removable_drives(&self) -> Result<Vec<VolumeInfo + mount path>, PlatformError>` if not already present.
- Remove the Drive refresh-token secret from the secrets list.
- Add a table of size budgets per platform from D28.

#### Task 10 — `docs/DEPENDENCIES.md`
- npm runtime: `react`, `react-dom`. npm dev: `@vitejs/plugin-react`, `eslint-plugin-react`, `eslint-plugin-react-hooks`, `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom`.
- Remove dependencies used only for Google Drive (OAuth, `reqwest` on desktop, a fake HTTP server for Drive tests). If `vidya-client` needs an HTTP client, keep one entry for it with rustls and default features off.
- Add a "Size impact" column. Create `docs/SIZE.md` with a table: date, prompt, change, Windows installer, Windows installed, macOS arm64 dmg, macOS arm64 installed, macOS x64 dmg, Android APK. Leave it empty for now.

#### Task 11 — `docs/DATA_MODEL.md`
Do not add tables (P2.3's test counts them). Add to "Default data created by setup" the app settings `backup_staff_can_run = "1"` and `backup_destinations = "[]"` (JSON array). Remove any Google Drive keys from `meta` or `app_settings` descriptions.

#### Task 12 — Consistency report
Search `docs/` and `*AGENTS.md` for `Google Drive`, `drive_`, `OAuth`, `universal-apple-darwin`, `SafeHtml`, `innerHTML`, `html\``, `delegate(` and `views/…\.js`. Fix everything except the note that D15 is superseded. List what you changed.

### Do not
- Do not write application code or create the npm or Cargo project (that is P1.1).
- Do not change decisions other than D15 and the new D27–D31.

### Done when (agent)
- [ ] 45 files in `docs/prompts/`
- [ ] Consistency report shows nothing left
- [ ] Summary lists every document changed

### Done when (developer)
- [ ] Read D27–D31 in `docs/DECISIONS.md` and approve them
- [ ] Decide whether teachers signed in at the office computer should also have `backup.run`. The default is no; changing it is one cell in PERMISSIONS.md

---

## P1.1 — Workspace, tooling, size gate and CI

| | |
|---|---|
| Phase | 1 Foundation |
| Depends on | P0.1 |
| Size | large |
| Runs on | MacBook; Windows build in GitHub Actions |
| Read first | AGENTS.md, docs/ARCHITECTURE.md, docs/DEPENDENCIES.md, docs/PLATFORMS.md, docs/TESTING_STRATEGY.md, docs/DECISIONS.md (D27, D28) |

### Before you start (developer)
1. Run `./scripts/doctor.sh`. Install everything marked ✗ in "Core tools". Android can wait until P8.1.
2. Create a **private** GitHub repository named `vidya`. Clone it to `~/code/vidya`.
3. Copy the whole starter kit (including the hidden `.claude` folder) into `~/code/vidya`.
4. `git add -A && git commit -m "Starter kit" && git push`.
5. P0.1 must be finished. Open Terminal in `~/code/vidya`, start Claude Code with `claude`, then type `/run P1.1`.

### Goal
An empty but complete project skeleton: npm + Vite + React frontend, Tauri 2 app, Rust workspace with all crates from ARCHITECTURE.md, linting, tests, `npm run verify`, a size gate, and GitHub Actions that test on macOS and build a Windows installer and a macOS disk image, failing if either is over 30 MB. No features yet.

### Tasks

#### Task 1 — Root files
Create:
- `.gitignore`: `node_modules/`, `dist/`, `target/`, `.DS_Store`, `*.vidyabak`, `*.vidyabak.partial`, `.env`, `.env.*`, `*.p12`, `*.pem`, `*.jks`, `*.keystore`, `provider-keys/`, `.test-phone-*`, `keystore.properties`, `coverage/`, `size-report.json`. `src-tauri/gen/schemas/` is **not** ignored (keep it).
- `.editorconfig`: UTF-8, LF, 2 spaces for JS/JSX/JSON/CSS/HTML/MD, 4 spaces for Rust, final newline.
- `rust-toolchain.toml`: `channel = "stable"`, `components = ["rustfmt", "clippy"]`, `targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]`.
- `.nvmrc`: the major version of the installed Node.js (`node --version`).
**Check:** `rustup show` lists the toolchain from the file.

#### Task 2 — npm, Vite and React
1. `npm init -y`, then edit `package.json`: `"name": "vidya"`, `"private": true`, `"type": "module"`, `"version": "0.1.0"`.
2. Install runtime dependencies (latest versions, no typed versions): `react react-dom @tauri-apps/api`. Install dev dependencies: `vite @vitejs/plugin-react vitest jsdom @testing-library/react @testing-library/user-event @testing-library/jest-dom eslint @eslint/js globals eslint-plugin-react eslint-plugin-react-hooks prettier @tauri-apps/cli`.
3. Scripts:
   ```json
   "dev": "vite",
   "build": "vite build",
   "build:mobile": "vite build --mode mobile",
   "preview": "vite preview",
   "tauri": "tauri",
   "test": "vitest run",
   "test:watch": "vitest",
   "lint": "eslint .",
   "format": "prettier --write .",
   "format:check": "prettier --check .",
   "size": "node scripts/check-size.mjs",
   "verify": "bash scripts/verify.sh"
   ```
4. `vite.config.js` following the **Tauri 2 + Vite guide for the installed Tauri version** (read it; do not guess): `plugins: [react()]`, root `src`, `build.outDir` `../dist`, `emptyOutDir: true`, `build.target` as the Tauri guide recommends for each platform's web engine, `build.sourcemap` false in production, dev server port 1420 with `strictPort: true`, `clearScreen: false`, host from `TAURI_DEV_HOST` for mobile, ignore watching `src-tauri`. Vitest: environment `jsdom`, `setupFiles: ['./src/test/setup.js']` (imports `@testing-library/jest-dom/vitest`), include `src/**/*.test.{js,jsx}` and `scripts/**/*.test.js`.
5. `src/index.html` with `<div id="app"></div>`, `<div id="print-root"></div>` and `<script type="module" src="/main.jsx"></script>`.
6. `src/main.jsx`: `if (import.meta.env.MODE === 'mobile') import('./main-mobile.jsx'); else import('./main-desktop.jsx');`. Vite replaces the mode at build time, so the unused branch is removed from the bundle.
7. `src/main-desktop.jsx`: `createRoot(document.getElementById('app')).render(<StrictMode><App /></StrictMode>)`. `App` shows "Vidya" and the app version from `@tauri-apps/api/app` (`getVersion`), falling back to "dev" outside Tauri.
8. `src/main-mobile.jsx`: same, with text "Vidya mobile".
9. `scripts/check-bundle-split.mjs`: after `npm run build` and `npm run build:mobile` into separate folders, fail if the mobile bundle contains the marker string `VIDYA_DESKTOP_ONLY` (put that string in a comment-free constant exported from `main-desktop.jsx`). P8.1 relies on this.
**Check:** `npm run build` produces `dist/index.html`; `npm run build:mobile` builds too; the split check passes.

#### Task 3 — Tauri app crate
1. Read `npm run tauri init -- --help`. Initialise non-interactively with: app name `Vidya`, window title `Vidya`, frontend dist `../dist`, dev URL `http://localhost:1420`, before-dev `npm run dev`, before-build `npm run build`.
2. In `src-tauri/Cargo.toml`: package name `vidya-app`, lib name `vidya_app_lib`, `publish = false`, keep the `crate-type` values Tauri generated for mobile support. Turn off default features of `tauri` that the app does not need, after reading the feature list of the installed version; list which you kept and why.
3. In `tauri.conf.json`: `productName` `Vidya`, `identifier` `in.vidya.school`, version `0.1.0`, main window label `main`, 1280×800, minimum 1000×680, centered.
4. Keep `src-tauri/AGENTS.md` and `CLAUDE.md` from the kit.
**Check:** `npm run tauri dev` opens a window showing "Vidya" and a version.

#### Task 4 — Rust workspace
1. Root `Cargo.toml`:
   ```toml
   [workspace]
   resolver = "2"
   members = ["src-tauri", "crates/*"]

   [workspace.package]
   edition = "2021"
   version = "0.1.0"
   publish = false
   license = "LicenseRef-Proprietary"

   [workspace.dependencies]
   # filled in by later prompts using versions resolved by cargo add

   [profile.release]
   codegen-units = 1
   lto = true
   opt-level = "s"
   panic = "abort"
   strip = true
   ```
   Move any `[profile.*]` Tauri generated in `src-tauri/Cargo.toml` to the root (profiles only work at the workspace root). Try `opt-level = "z"` against `"s"` on the Tauri binary, record both sizes in `docs/SIZE.md`, and keep the smaller one unless it breaks tests.
2. Create library crates with `cargo new --lib`: `crates/vidya-core`, `vidya-db`, `vidya-services`, `vidya-license`, `vidya-sync`, `vidya-server`, `vidya-client`, `vidya-backup`, `vidya-export`, `vidya-testkit`. Each uses `edition.workspace = true`, `version.workspace = true`, `publish.workspace = true`.
3. Add **path dependencies only**, exactly following the arrows in ARCHITECTURE.md section 2. `vidya-testkit` is only a `[dev-dependencies]` entry.
4. Each `lib.rs`: a crate doc comment copied from the crate table in `crates/AGENTS.md`, and one test `fn crate_builds() {}`.
5. Add `vidya-services` as a dependency of `src-tauri` (nothing used yet).
6. Write into `crates/AGENTS.md` under a new "Adding dependencies" heading how to add a shared external dependency: run `cargo add <crate> -p <member>`, read the resolved version in `Cargo.lock`, move it to `[workspace.dependencies]`, use `<crate> = { workspace = true }` in members, use `default-features = false` and enable only the features needed, then run the size check.
**Check:** `cargo test --workspace` passes; `cargo tree -p vidya-core` shows no workspace crates.

#### Task 5 — Lint and format
1. `eslint.config.js` (flat config): `@eslint/js` recommended, `eslint-plugin-react` recommended with the JSX runtime config and React version `detect`, `eslint-plugin-react-hooks` recommended, browser and node globals, ES2022 modules, files `**/*.{js,jsx}`. Rules:
   - `react/no-danger: error`
   - `no-restricted-properties` blocking `innerHTML`, `outerHTML`, `insertAdjacentHTML` and `write` on `document`, message "Render with React components (UI_GUIDE.md)". **No override for any file.**
   - `no-restricted-syntax` blocking `eval`, `new Function`, and JSX attributes named `dangerouslySetInnerHTML`.
   - `react/jsx-no-target-blank: error`, `react/prop-types: off`.
   - Ignore `dist`, `target`, `src-tauri/gen`, `reference`.
2. `.prettierrc.json`: `{ "singleQuote": true, "printWidth": 110 }`. `.prettierignore`: `dist`, `target`, `src-tauri/gen`, `reference`, `docs`, `Cargo.lock`, `package-lock.json`.
3. `rustfmt.toml`: `max_width = 110`.
4. `src/core/lint-guard.test.js`: a Vitest test that runs ESLint programmatically on a fixture string containing `dangerouslySetInnerHTML` and on `el.innerHTML = x`, and asserts both are errors. This keeps the rule from being removed silently.
**Check:** `npm run lint`, `npm run format:check`, `cargo fmt --check` pass.

#### Task 6 — Check scripts
1. `scripts/check-api-drift.mjs`:
   - Read command names from the `generate_handler![...]` list(s) in `src-tauri/src/lib.rs` (strip module paths: `commands::fees::collect_fee` → `collect_fee`).
   - Read exported function names from `src/api/commands.js` and convert camelCase to snake_case.
   - Read backticked command names from the first column of tables in `docs/API.md` (ignore HTTP routes and events tables: only rows whose first cell matches `` `[a-z_]+` ``, optionally followed by `**D**` or `**M**`).
   - Fail if a Rust command is missing from JS or API.md, or a JS command is missing from Rust or API.md. Commands only in API.md are "planned" and allowed; print the count.
   - If `src/api/commands.js` does not exist yet, skip JS with a notice.
   - Export the parsing functions and add `scripts/check-api-drift.test.js` with small fixture strings.
2. `scripts/check-i18n.mjs`: if `src/locales/en.json` and `hi.json` exist, fail when their key sets differ; otherwise print "no locales yet". (P1.2 extends it.)
3. `scripts/check-size.mjs` (the size gate, D28):
   - Inputs: optional paths; with no arguments it looks for known outputs: Windows `target/release/bundle/nsis/*.exe` (download); macOS `target/*/release/bundle/dmg/*.dmg` (download) and the `.app` folder next to it (installed, total of all files); Android `src-tauri/gen/android/app/build/outputs/**/*.apk` and `*.aab`. For the Windows installed size, add the main executable and every file Tauri bundles as resources (read `tauri.conf.json` for the resource list; the VM check in P1.3 confirms the real installed size).
   - Prints a table in MB with two decimals. Exits 1 if anything is over **30.00 MB**, prints a warning if over **25.00 MB**, exits 0 if nothing was found (with a notice).
   - Writes `size-report.json`.
   - `scripts/check-size.test.js` with temporary files of known sizes.
4. `scripts/verify.sh` (bash, `set -euo pipefail`), printing a heading before each step:
   1. `cargo fmt --all -- --check`
   2. `cargo clippy --workspace --all-targets -- -D warnings`
   3. `cargo test --workspace`
   4. `npm run lint`
   5. `npm run format:check`
   6. `npm test`
   7. `node scripts/check-api-drift.mjs`
   8. `node scripts/check-i18n.mjs`
   9. If `VERIFY_DENY=1`: `cargo deny check licenses bans sources`
   10. If `VERIFY_SIZE=1`: `node scripts/check-size.mjs`
   End with `ALL CHECKS PASSED`.
**Check:** `npm run verify` ends with `ALL CHECKS PASSED`.

#### Task 7 — cargo-deny
1. `cargo install cargo-deny --locked` (tell the developer if it needs to be installed manually).
2. `deny.toml`: allow licences MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, Unicode-3.0, Unicode-DFS-2016, CC0-1.0; deny unknown registries and git sources; under bans, warn on duplicate crate versions (duplicates cost size). Check the config format for the installed cargo-deny version (`cargo deny init` produces a template — start from it).
**Check:** `VERIFY_DENY=1 npm run verify` passes, or list the exact licences that fail and stop for approval.

#### Task 8 — GitHub Actions
Create `.github/workflows/ci.yml`, triggered on every push and pull request:
- Job `macos` on `macos-latest`: checkout; set up Node from `.nvmrc` with npm cache; install Rust stable with rustfmt and clippy; Rust cache (use a well-known Rust cache action; read its README for the current usage); `npm ci`; `npm run verify`; build the Apple Silicon disk image `npm run tauri build -- --target aarch64-apple-darwin --bundles dmg` (check the flags with `--help`); `node scripts/check-size.mjs`; upload `size-report.json` as artifact `size-macos`.
- Job `windows` on `windows-latest`: same setup; `npm ci`; `npm run verify`; then build the NSIS installer only (check `npm run tauri build -- --help` for the bundles option); `node scripts/check-size.mjs`; upload the `.exe` from the workspace `target/release/bundle/nsis/` folder as artifact `vidya-windows-installer`, kept 14 days, and `size-report.json` as `size-windows`.
- Job `deny` on `ubuntu-latest`: install cargo-deny, run `cargo deny check`.
- Use pinned major versions of actions and read each action's README before using its inputs.
- The size step must run **before** the upload and fail the job when over budget.
**Check:** push a commit; all three jobs are green; the artifacts are downloadable; the job logs show the size table.

#### Task 9 — Developer README
Write `README.md`: what Vidya is (two lines), prerequisites (link to `scripts/doctor.sh`), how to run (`npm install`, `npm run tauri dev`), how to verify, the 30 MB size budget and `npm run size`, how to get the Windows installer from GitHub Actions (Actions tab → latest run → Artifacts → download → unzip → run in the Windows VM; SmartScreen: More info → Run anyway, because it is unsigned until P10.1), where the docs and prompts are, how to run a prompt (`/run P1.1`).

### Do not
- Do not create any feature code, database or UI beyond the placeholder text.
- Do not add dependencies not listed in DEPENDENCIES.md.
- Do not add a React router, state library, UI kit, icon library or CSS framework.
- Do not commit generated build output.

### Done when (agent)
- [ ] `npm run verify` passes locally
- [ ] `cargo tree` shows dependency directions exactly as ARCHITECTURE.md
- [ ] `DEPENDENCIES.md` installed versions filled in for everything added
- [ ] First row of `docs/SIZE.md` filled from the local macOS build (and the Windows numbers after CI runs)
- [ ] CI workflow file written; you listed what the developer must check in GitHub

### Done when (developer)
- [ ] `npm run tauri dev` opens a Vidya window on the MacBook
- [ ] All GitHub Actions jobs are green after pushing, and each shows sizes under 30 MB
- [ ] The Windows installer artifact installs and opens in the Windows VM

---

## P1.2 — React port of the prototype

| | |
|---|---|
| Phase | 1 Foundation |
| Depends on | P1.1 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, src/AGENTS.md, docs/UI_GUIDE.md ("React rendering rules"), docs/API.md, docs/DECISIONS.md (D27), reference/VidyaSchoolApp_step1.html (all of it) |

### Goal
Move every screen of the prototype into React components in `src/`, looking and behaving exactly the same, with translation keys. Data still comes from an in-memory mock that copies the prototype's logic, behind the same function names the real Rust commands will have.

**Temporary exception to AGENTS.md:** business logic is allowed only inside `src/api/mock/`, because Rust does not exist yet. It is deleted piece by piece from P2.7 to P4.4. Nothing outside `src/api/mock/` may contain business rules.

### Folder layout
```
src/
  main.jsx  main-desktop.jsx  main-mobile.jsx
  app/        DesktopApp.jsx  MobileApp.jsx  AppProviders.jsx
  core/       i18n.jsx  router.jsx  ui.jsx  print.jsx  useCommand.js  format.js  links.js  session.jsx
  api/        commands.js  errors.js  session.js  mock/
  components/ (shared pieces, one component per file)
  views/      (one folder or file per screen, .jsx)
  styles/     tokens.css  base.css  components.css  print.css
  locales/    en.json  hi.json
  test/       setup.js  render.jsx (renders a component inside all providers with a fake user)
```

### Tasks

#### Task 1 — Styles
Split the prototype's `<style>` block, **without changing any values**, into:
- `src/styles/tokens.css` — the `:root` variables
- `src/styles/base.css` — reset, typography, layout (nav, top bar, view, tab bar), utility classes
- `src/styles/components.css` — buttons, inputs, cards, stats, tables, chips, pills, notes, modal, toast, attendance cards, marks inputs, receipt, report card, wizard, slips
- `src/styles/print.css` — the print rules, plus: when printing, hide `#app` and show only `#print-root`
Import them in `main-desktop.jsx` in that order.
Where the prototype uses `style="..."` attributes with fixed values, create a utility or component class instead. Only values computed at runtime may use `style={{…}}`. List the classes you added.
**Check:** `npm run dev` page background, fonts and a sample button match the prototype.

#### Task 2 — `src/core/format.js` and `src/core/links.js`
- `formatRupees(n)` (Indian grouping with `₹`), `formatDate('2026-09-15')` → `15 Sep 2026`, `formatDateTime(iso)`, `formatTime(iso)` (`2:30 pm`), `timeAgo(iso, now)`. Pure functions with the current time passed in. Tests with fixed inputs, including 0, 999, 1,000, 1,00,000 and 1,23,45,678.
- `telHref(mobile)`: returns `tel:+91XXXXXXXXXX` only for exactly 10 digits, else `null`. Test with injection-like input (`javascript:`, spaces, letters).

#### Task 3 — `src/core/i18n.jsx` and locales
1. `I18nProvider`, `useT()` returning `t(key, params)`, `useLanguage()` returning `{ language, setLanguage }`, and a plain `translate(lang, key, params)` for non-component code. A missing key returns the key and warns in development.
2. Go through every visible string in the prototype and create keys in `src/locales/en.json` grouped by area (`nav.*`, `common.*`, `login.*`, `wizard.*`, `home.*`, `students.*`, `attendance.*`, `marks.*`, `fees.*`, `reports.*`, `users.*`, `activity.*`, `backup.*`, `settings.*`, `errors.*`).
3. `hi.json` must have the same keys. Use the Hindi text that exists in the prototype. For other keys, copy the English text for now (P9.1 translates).
4. Extend `scripts/check-i18n.mjs`:
   - Key parity between en and hi (already there).
   - Parse every `src/**/*.jsx` file with a JSX-aware parser that is already installed (ESLint's `espree` with `jsx: true`, through ESLint's API; do not add a new package). Report JSX text children and `title`, `placeholder`, `aria-label`, `alt` attribute strings that contain two or more English words. Allow-list: `{/* i18n-ignore */}` or `// i18n-ignore` on the same line.
   - Every literal key in `t('…')` calls exists in `en.json`.
**Check:** `node scripts/check-i18n.mjs` passes.

#### Task 4 — `src/core/ui.jsx`
- `ToastProvider` and `useToast()` → `toast(message, { kind: 'info'|'ok'|'error' })`.
- `ModalProvider` and `useModal()` → `openModal({ title, subtitle, body, footer, locked, wide })` where `body` and `footer` are React nodes; returns `{ close }`. `closeModal()`.
- `useConfirm()` → `confirm({ title, message, confirmLabel, danger }) → Promise<boolean>`.
- Esc closes unlocked modals; focus moves into the modal (first focusable element) and returns to the opener afterwards; Tab is trapped inside. The modal renders through a portal with a z-index above login and setup screens (the prototype had this bug; keep the fix).
- Tests with React Testing Library: confirm resolves true and false; a locked modal ignores Esc; focus returns to the button that opened it.

#### Task 5 — `src/core/print.jsx`
`PrintProvider` and `usePrint()` → `printElement(reactNode)`: renders the node into `#print-root` with a portal, waits for the next animation frame and for `document.fonts.ready`, calls `window.print()`, then clears the print root. No HTML strings anywhere. P5.1 and P5.2 build on this.

#### Task 6 — `src/api/errors.js`, `src/api/session.js`, `src/api/commands.js`
1. `errors.js`: `class AppError extends Error { constructor({ kind, messageKey, params, message, field }) }` and `toAppError(unknown)` that turns anything thrown into an `AppError` of kind `internal` with a safe message.
2. `session.js`: holds the current token in a module variable (memory only), with `getToken`, `setToken`, `clearToken`.
3. `commands.js`: one exported async function per command in docs/API.md that the prototype's screens need (names in camelCase, parameters as documented). Each function currently calls `mock.<name>(...)` and converts thrown values with `toAppError`.
4. The file begins with a comment: "Every export here maps 1:1 to a Tauri command in docs/API.md. Mock implementations are temporary."
**Check:** `node scripts/check-api-drift.mjs` reports JS commands that exist in API.md and no unknown names.

#### Task 7 — `src/core/useCommand.js` and `src/core/session.jsx`
- `useQuery(fn, deps)` → `{ data, error, loading, reload }`. It ignores results from an older call when deps change, keeps the old data while reloading, and exposes `error` as an `AppError`.
- `useMutation(fn)` → `{ run, pending, error, fieldError(name) }`. `run` resolves with the result or rejects; `fieldError('amount')` returns the message when `error.field === 'amount'`.
- `SessionProvider` and `useCurrentUser()` → `{ user, permissions, can(action), signIn, signOut, refresh }`, reading `currentUser()` from `commands.js`.
- Tests: stale responses are ignored; field errors map correctly.

#### Task 8 — `src/api/mock/`
Port the prototype's data and logic into modules: `db.js` (in-memory state), `sample.js`, `setup.js`, `auth.js`, `students.js`, `attendance.js`, `marks.js`, `fees.js`, `reports.js`, `users.js`, `activity.js`, `backup.js`, `settings.js`, `index.js`. Keep the prototype's rules and messages exactly, but return the **DTO shapes named in docs/API.md** and throw `AppError`-shaped objects (`{ kind, messageKey, params, message, field }`). Keep the prototype's Web Crypto password and backup code for now.

#### Task 9 — Router and app shell
1. `src/core/router.jsx`: `RouterProvider`, `useRouter()` → `{ viewId, params, go(id, params), back() }`. The view list is an array in `src/app/views-desktop.js`: `{ id, titleKey, Component, navIcon, permission }`. Navigation shows only views whose `permission` passes `can()`. Going to a view the user may not open shows the home view with a toast.
2. `src/views/Shell.jsx`: nav, top bar (backup pill for users with `backup.run`, device pill otherwise, language switch), bottom tab bar with More, account modal (change password, sign out).
3. Screens before sign-in: `views/setup/Welcome.jsx`, `views/setup/Wizard.jsx` (7 steps), `views/setup/Credentials.jsx`, `views/Login.jsx`, `views/Password.jsx` (forced and normal).
4. `src/app/DesktopApp.jsx` chooses between these and the shell based on `appStatus()` and the session.

#### Task 10 — Feature views
Port each screen from the prototype into its own component, same layout, same wording (through `t()`), same validation messages (shown from `AppError.message` next to `field`):
`views/Home.jsx` (principal, accountant, teacher variants as three components), `views/students/StudentList.jsx`, `views/students/StudentDetail.jsx`, `views/students/StudentForm.jsx`, `views/Attendance.jsx` (with monthly register print), `views/Marks.jsx`, `views/ReportCard.jsx`, `views/fees/FeeRegister.jsx`, `views/fees/CollectFee.jsx`, `views/fees/Receipt.jsx`, `views/fees/DayBook.jsx`, `views/Reports.jsx`, `views/Users.jsx`, `views/Activity.jsx`, `views/Backup.jsx`, `views/Settings.jsx`.
Rules: views call only `commands.js` through the hooks; forms are controlled components with local state; re-query after successful mutations; printing uses `usePrint()`. Put repeated markup in `src/components/`: `StatCard`, `EmptyState`, `ChipBar`, `Pill`, `FeePill`, `DataTable` (columns as props, keyboard accessible), `ReceiptCopy`, `Slip`, `ReportCardSheet`, `Field` (label, input, error), `Button`.
Keep each component under about 200 lines; split large screens into sub-components in the same folder.

#### Task 11 — Entry points
- `main-desktop.jsx`: imports styles, renders `<AppProviders><DesktopApp/></AppProviders>`.
- `main-mobile.jsx`: renders `MobileApp` with views login, password, home, students, attendance, marks, reportcard and fees only (from `src/app/views-mobile.js`). Not built for Android yet.

#### Task 12 — Tests
- At least one React Testing Library test per view with `commands.js` mocked (`vi.mock`): renders without throwing; shows the error message next to the right field when a command rejects with a validation `AppError`; a student named `Sierra D'Souza <b>x</b>` appears as literal text (assert with `getByText` and that no `<b>` element exists).
- `router.test.jsx`: a teacher's navigation does not include fees, reports, users, backup or settings.
- Interactions use `@testing-library/user-event`, not direct DOM events.

#### Task 13 — Parity checklist and size
1. Create `docs/PARITY.md`: a checklist of every prototype screen and action (at least 60 lines: each button, form, validation message, print action). Mark each item as ported. The developer will tick them after comparing side by side.
2. Record the production `dist/` size (total and gzip size of the JS files) in `docs/SIZE.md`.

### Do not
- `grep -rnE "innerHTML|outerHTML|dangerouslySetInnerHTML|insertAdjacentHTML" src` must return nothing (the lint guard test fixture lives in a string in `lint-guard.test.js`; exclude that one file from the grep).
- No business logic outside `src/api/mock/`.
- No new npm packages.
- Do not redesign or reword anything.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] The grep above returns nothing
- [ ] `docs/PARITY.md` complete

### Done when (developer)
- [ ] `npm run tauri dev`: load the sample school and sign in as sunita, anita and sierra (password `vidya123`); every screen works like the prototype
- [ ] Tick `docs/PARITY.md` by opening the prototype in Chrome side by side

---

## P1.3 — Desktop shell and security settings

| | |
|---|---|
| Phase | 1 Foundation |
| Depends on | P1.2 |
| Size | medium |
| Runs on | MacBook; Windows via CI and VM |
| Read first | AGENTS.md, src-tauri/AGENTS.md, docs/PLATFORMS.md, docs/DEPENDENCIES.md, docs/DECISIONS.md (D28) |

### Goal
A proper desktop app on macOS and Windows: strict security settings, single instance, logging, Mac menus, icons, installer settings, the `Platform` trait skeleton, and a measured size for every desktop build.

### Tasks

#### Task 1 — Content Security Policy
1. Read the Tauri 2 security/CSP documentation for the installed version.
2. Set `app.security.csp` so that: scripts only from the app (`'self'`), no remote origins at all, images from `'self'` and `data:`, styles from `'self'` **without** `'unsafe-inline'`, `connect-src` exactly what Tauri IPC needs for the installed version, `font-src 'self'`, `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`, `form-action 'none'`.
3. React's `style={{…}}` sets CSSOM properties, which CSP does not block; style **attributes** in markup would be blocked. Confirm this in the running app. If Vite or Tauri injects inline styles or scripts in development only, allow them only in the development configuration and say so.
**Check:** `npm run tauri dev` and a release build: open every screen; the devtools console (debug build) shows no CSP violations. List the final CSP string in your summary.

#### Task 2 — Capabilities and command restriction
1. `src-tauri/capabilities/default.json` for window `main`: only the core permissions actually needed plus the plugins added in Task 3. No shell, no filesystem plugin, no HTTP plugin.
2. Find out (installed version docs or source) whether Tauri allows restricting which **app commands** the webview may call through the build-script app manifest. If yes, set it up so only commands listed in `generate_handler!` are allowed and note how new commands are added. If not, record that in KNOWN_ISSUES.
**Check:** the app runs; any removed permission shows as an error only if used.

#### Task 3 — Plugins
Add (with `cargo add` in `src-tauri` and `npm install` for JS parts) and register in `lib.rs`:
- `tauri-plugin-single-instance` (desktop): a second launch focuses the existing window. Register it **first**, as its docs require.
- `tauri-plugin-log`: log to a file in the app log directory and to stdout in development; level `info` in release, `debug` in development; rotate files and keep at most 5 × 2 MB. Write the rule in a comment: never log personal data.
- `tauri-plugin-dialog` and `tauri-plugin-opener` (desktop): registered, not used yet.
Update DEPENDENCIES.md versions, and record the size change in `docs/SIZE.md`.
**Check:** starting the app twice keeps one window; log file created.

#### Task 4 — Menus and keyboard
- macOS: application menu (About Vidya, separator, Hide Vidya, Hide Others, Show All, separator, Quit Vidya), Edit menu (Undo, Redo, Cut, Copy, Paste, Select All) and Window menu (Minimize, Zoom), built with Tauri's menu API using predefined items. Verify each predefined item exists in the installed version.
- Windows: no menu bar.
- Frontend: Esc closes an unlocked modal (already done). In release builds, block the right-click context menu except inside inputs and textareas (a single listener in `main-desktop.jsx`).
**Check:** on Mac, Cmd+C / Cmd+V work in the login username field; Cmd+Q quits.

#### Task 5 — Developer tools
Confirm devtools are available only in debug builds (check Tauri's `devtools` feature and defaults). Make sure the release configuration does not enable them.
**Check:** release build on Mac: right-click Inspect is not available and the keyboard shortcut does nothing.

#### Task 6 — Icons
If the developer has provided `app-icon.png` (1024×1024) in the repo root, run `npm run tauri icon app-icon.png`. Otherwise create a temporary 1024×1024 PNG with a Node script using only built-in modules (a blue `#1F5FA9` rounded square is enough), save as `app-icon.png`, run the icon command, and add "Replace temporary app icon" to KNOWN_ISSUES. Delete generated icon sizes that no bundle uses (check which files `tauri.conf.json` references).
**Check:** icons exist for macOS (`.icns`), Windows (`.ico`) and the PNG sizes in use.

#### Task 7 — Bundle settings
Using the configuration schema in `src-tauri/gen/schemas/` for the exact key names:
- Bundle targets: `nsis` on Windows; `dmg` and `app` on macOS.
- Windows: NSIS per-machine install; WebView2 install mode "download bootstrapper" (keeps the installer small; the offline installer would break the budget); NSIS compression set to the strongest available (LZMA; verify the key); installer languages English (Hindi added in P9.1).
- macOS: minimum system version `11.0` or Tauri's minimum if higher (state which); category `public.app-category.education`; copyright `© Vidya`; an empty `src-tauri/entitlements.plist` prepared but not used until P10.2.
- Short description "School management that works without internet".
**Check:** on the MacBook build both `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, and also `--target universal-apple-darwin` once. Run `npm run size` and record all three in `docs/SIZE.md` (dmg and installed `.app`). Record the CI Windows installer size. In the Windows VM, install and record the real installed folder size. If the universal build fits both limits with at least 8 MB to spare, say so; the default stays separate builds (D28).

#### Task 8 — Platform trait skeleton
1. `src-tauri/src/platform/mod.rs`: the `Platform` trait exactly as in PLATFORMS.md (including `volume_info` and `removable_drives` from P0.1), `PlatformError` (thiserror) with `Unsupported`, `Io`, `Os(String)`, `NotFound`, and `fn current() -> Box<dyn Platform>` choosing by `cfg(target_os)`.
2. `macos.rs`, `windows.rs`, `android.rs` with structs implementing the trait; every method returns `Err(PlatformError::Unsupported)` except `name()`. Add a comment on each method naming the prompt that implements it (P2.4, P4.2, P6.1, P6.2, P7.1, P7.2, P8.2). These methods are not called by anything reachable yet.
3. `fake.rs` behind `#[cfg(test)]`: in-memory implementation for tests.
**Check:** `cargo clippy --workspace --all-targets -- -D warnings` passes on the MacBook and in the Windows CI job.

### Do not
- Do not add a filesystem, shell or HTTP plugin.
- Do not enable devtools in release.
- Do not implement platform methods yet.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Final CSP and capabilities listed in the summary
- [ ] `docs/SIZE.md` has arm64, x64 and universal macOS rows and the Windows installer and installed sizes; all under 30 MB

### Done when (developer)
- [ ] Open the `.dmg` on the MacBook (right-click Vidya.app → Open, because it is unsigned); the app works
- [ ] Copy and paste work; launching twice keeps one window
- [ ] In the Windows VM, the CI installer installs, runs, and uninstalls; note the size of `C:\Program Files\Vidya` for the agent

---

## P2.1 — vidya-core domain rules

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P1.1 |
| Size | medium |
| Runs on | MacBook (pure Rust, also tested in CI) |
| Read first | AGENTS.md, crates/AGENTS.md, docs/DATA_MODEL.md ("Derived values"), docs/SYNC_PROTOCOL.md (section 2), docs/PRODUCT.md, docs/UI_GUIDE.md ("Translation") |

### Goal
All pure business rules in `crates/vidya-core`, fully tested, with no I/O, no clock reads, no randomness and no database. Every other crate will call these functions instead of re-implementing rules.

### Dependencies allowed in this prompt
`serde` (derive), `serde_json`, `thiserror`, `chrono` (serde; `default-features = false` with only the features needed; no `clock` usage inside core: time is always passed in), `proptest` (dev). Add with `cargo add`, move to workspace dependencies as described in crates/AGENTS.md.

### Module layout (create exactly these files)
```
crates/vidya-core/
  src/lib.rs          pub mod declarations only
  src/error.rs        DomainError
  src/i18n.rs         message lookup from locales
  src/roles.rs        Role, Actor, Origin
  src/money.rs
  src/words.rs
  src/dates.rs
  src/validation.rs
  src/fees.rs
  src/marks.rs
  src/attendance.rs
  src/usernames.rs
  src/secrets.rs      temporary password formatting from supplied bytes
  src/hlc.rs
  locales/en.json
  locales/hi.json
  tests/vectors.rs    table-driven tests from this prompt
```

### Tasks

#### Task 1 — Errors and messages
```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
#[error("{message_key}")]
pub struct DomainError {
    pub kind: ErrorKind,                       // Validation, Permission, NotFound, Conflict, Auth, Locked, License, Offline, Internal
    pub message_key: &'static str,
    pub params: std::collections::BTreeMap<String, String>,
    pub field: Option<&'static str>,
}
```
Helper constructors: `DomainError::validation(key).field("amount").param("balance", "₹9,100")`.
`i18n::message(lang: Lang, key: &str, params: &BTreeMap<String,String>) -> String` loads `locales/en.json` and `hi.json` with `include_str!` once (use `std::sync::OnceLock`), replaces `{name}` placeholders, falls back to English, then to the key.
Tests: placeholder replacement; missing Hindi key falls back to English; missing key returns key.

#### Task 2 — Roles and actor
```rust
pub enum Role { Principal, Accountant, Teacher }        // serde "principal" | "accountant" | "teacher"
pub enum Origin { OfficeComputer, Phone }               // where the request comes from (D29, D31)
pub struct Actor { pub user_id: String, pub role: Role, pub section_ids: BTreeSet<String>, pub device_id: String, pub origin: Origin, pub lang: Lang }
pub enum Lang { En, Hi }
```
`origin` is always set by the server from the session or the signed device, never from request data.

#### Task 3 — Money
```rust
pub struct Rupees(pub i64);
pub fn format_inr(amount: Rupees) -> String;             // "₹1,23,45,678", negative "−₹500"
pub fn parse_rupees(input: &str) -> Result<Rupees, DomainError>; // accepts "1,000", "₹ 500", " 750 "; rejects decimals, negatives, empty, > 10,00,00,000
```
Vectors: 0→`₹0`, 999→`₹999`, 1000→`₹1,000`, 100000→`₹1,00,000`, 12345678→`₹1,23,45,678`. Parse errors use key `money.error.invalid` with field set by caller.
Property test: `parse_rupees(format_inr(x) without ₹)` round-trips for 0..=10^9.

#### Task 4 — Amount in words (English, Indian system)
`pub fn amount_in_words_en(n: u64) -> String` and `pub fn receipt_words_en(n: u64) -> String` = `"Rupees {words} Only"`.
Vectors (exact strings):
| n | words |
|---|---|
| 0 | Zero |
| 7 | Seven |
| 15 | Fifteen |
| 20 | Twenty |
| 45 | Forty Five |
| 100 | One Hundred |
| 101 | One Hundred One |
| 999 | Nine Hundred Ninety Nine |
| 1000 | One Thousand |
| 1005 | One Thousand Five |
| 12000 | Twelve Thousand |
| 99999 | Ninety Nine Thousand Nine Hundred Ninety Nine |
| 100000 | One Lakh |
| 250500 | Two Lakh Fifty Thousand Five Hundred |
| 1234567 | Twelve Lakh Thirty Four Thousand Five Hundred Sixty Seven |
| 10000000 | One Crore |
| 123456789 | Twelve Crore Thirty Four Lakh Fifty Six Thousand Seven Hundred Eighty Nine |
Hindi words are **not** part of this prompt (P9.1, with a reviewed table).

#### Task 5 — Dates and sessions
```rust
pub fn parse_date(s: &str) -> Result<NaiveDate, DomainError>;          // YYYY-MM-DD only
pub fn format_date_en(d: NaiveDate) -> String;                         // "15 Sep 2026"
pub fn default_session_name(today: NaiveDate) -> String;               // April or later → "2026-27", Jan–Mar → "2025-26"
pub fn parse_session_name(s: &str) -> Result<(i32, i32), DomainError>; // "2026-27" → (2026, 2027); second part must be first + 1
pub fn session_bounds(name: &str) -> Result<(NaiveDate, NaiveDate), DomainError>; // 1 Apr to 31 Mar
pub fn days_in_month(year: i32, month: u32) -> u32;
```
Tests: leap year February, session change around 31 Mar / 1 Apr, invalid formats.

#### Task 6 — Validation
Each returns `Result<String_or_value, DomainError>` with the field name set:
- `validate_person_name(s, field)`: trimmed, collapse spaces, 2–80 characters, letters from any script, spaces, `.`, `'`, `-`.
- `validate_mobile(s)`: exactly 10 digits, first digit 6–9. Key `validation.mobile`.
- `validate_udise(s)`: empty or 11 digits.
- `validate_school_code(s)`: 3–12 of `[a-z0-9]`.
- `validate_username(s)`: 2–30 of `[a-z0-9]`.
- `validate_new_password(pw, username, same_as_current: bool)`: at least 8 characters; must not contain the username (case-insensitive); must not be the same as current. Keys `password.too_short`, `password.contains_username`, `password.same_as_current`.
- `validate_receipt_prefix(s)`: 2–4 of `[A-Z0-9]`.
- `validate_class_name(s)`: 1–12 of letters, digits, spaces.
- `validate_reason(s, min)`: trimmed length ≥ min.
- `validate_upi_reference(s)`: exactly 12 digits.
- `validate_cheque_reference(s)`: trimmed length ≥ 4.
Tests: at least three valid and three invalid cases per function, including Devanagari names like `सुनीता मिश्रा`.

#### Task 7 — Fees
```rust
pub struct FeePlan { pub tuition: i64, pub exam: i64, pub other: i64 }
pub struct FeeInputs { pub plan: FeePlan, pub terms: u32, pub transport_fee_per_term: i64, pub transport: bool, pub rte: bool, pub concession: i64 }
pub enum FeeState { Rte, Paid, Part, Due }
pub fn term_fee(plan: &FeePlan) -> i64;
pub fn session_due(i: &FeeInputs) -> i64;               // never below 0
pub fn one_term_amount(i: &FeeInputs) -> i64;           // term fee + transport if used
pub fn fee_state(due: i64, paid: i64, rte: bool) -> FeeState;
pub fn balance(due: i64, paid: i64) -> i64;             // max(0, due - paid)
pub enum PayMode { Cash, Upi, Cheque }
pub struct PaymentCheck<'a> { pub amount: i64, pub balance: i64, pub mode: PayMode, pub reference: &'a str, pub upi_reference_already_used: bool, pub rte: bool, pub student_active: bool }
pub fn validate_payment(p: &PaymentCheck) -> Result<(), DomainError>;
pub fn receipt_number(prefix: &str, counter: i64) -> String;   // "PC-0001"; 5+ digits after 9999: "PC-10000"
```
`validate_payment` order and keys: student not active `fees.error.student_not_active`; RTE `fees.error.rte`; amount ≤ 0 `fees.error.amount` (field amount); balance 0 `fees.error.no_balance`; amount > balance `fees.error.over_balance` with param `balance` formatted; UPI reference invalid `fees.error.upi_reference` (field reference); UPI used `fees.error.upi_used`; cheque reference `fees.error.cheque_reference`.
Vectors: plan 2400/300/400, terms 3, transport 900 used → due 12000; with concession 1000 → 11000; concession 50000 → 0; RTE → 0; terms 12 without transport → 37200; one term with transport → 4000.
Property test: `session_due` is never negative; `balance` never negative.

#### Task 8 — Marks and grades
```rust
pub enum MarkValue { Blank, Absent, Score(u16) }
pub fn parse_mark(input: &str, max: u16) -> Result<MarkValue, DomainError>; // "" blank; "ab"/"AB" absent; 0..=max; else marks.error.invalid with param max
pub struct ExamResult { pub got: u32, pub max: u32, pub entered: u32, pub percent_tenths: Option<u32> }
pub fn exam_result(values: &[MarkValue], max_per_subject: u16) -> ExamResult; // Blank skipped; Absent counts max but 0 got
pub struct GradeBand { pub grade: String, pub min_percent: u32 }
pub fn grade_for(percent_tenths: u32, scale: &[GradeBand]) -> String;   // highest band with min_percent*10 <= percent_tenths
pub fn validate_grade_scale(scale: &[GradeBand]) -> Result<(), DomainError>; // distinct grades, strictly decreasing mins, last min 0
```
`percent_tenths = (got * 1000 + max / 2) / max` (integer rounding half up).
Vectors: got 77 of 125 → 616 → C with default scale; 800 → A; 799 → B; 330 → D; 329 → E; 2 of 3 → 667; nothing entered → `None`.

#### Task 9 — Attendance
`AttendanceStatus { P, A, L }`, `next_status(Option<AttendanceStatus>) -> AttendanceStatus` (none→P→A→L→P), `AttendanceCounts { present, absent, leave }`, `percent(present, total) -> Option<u32>` rounded half up, `check_attendance_date(date, today, can_edit_past: bool) -> Result<(), DomainError>` (future always error `attendance.error.future`; past without permission `attendance.error.past_read_only`).

#### Task 10 — Usernames and temporary passwords
```rust
pub fn username_base_from_name(name: &str) -> Option<String>; // first word, lowercase, keep a-z0-9 only; None if empty after filtering
pub fn unique_username(base: &str, taken: &BTreeSet<String>) -> String; // sierra, sierra2, sierra3...
pub fn fallback_username_base(role: Role, index: usize) -> String;      // teacher1, accountant1
pub fn format_temp_password(bytes: [u8; 8]) -> String;                   // 4 letters from "abcdefghjkmnpqrstuvwxyz", "-", 4 digits from "23456789"; letters use bytes 0..4, digits bytes 4..8, modulo alphabet length
pub fn format_session_token(bytes: [u8; 32]) -> String;                  // base64url without padding (implement small encoder or use a listed crate; do not add a new crate)
```
Vectors: `Sierra D'Souza` → `sierra`; `सुनीता मिश्रा` → None; `  Ravi  Kumar` → `ravi`; taken {sierra} → `sierra2`; taken {sierra, sierra2} → `sierra3`; bytes `[0,1,2,3,4,5,6,7]` → `abcd-6789`; bytes `[23,24,25,26,8,9,10,11]` → `abcd-2345`.

#### Task 11 — Hybrid logical clock
Exactly as SYNC_PROTOCOL.md section 2:
```rust
pub struct Hlc { pub wall_ms: u64, pub counter: u32, pub device: String }
impl Hlc {
    pub fn to_text(&self) -> String;                    // "{:013}-{:06}-{device}"
    pub fn parse(s: &str) -> Result<Hlc, DomainError>;
}
pub struct HlcClock { last: Hlc }
impl HlcClock {
    pub fn new(device: String, last: Option<Hlc>) -> Self;
    pub fn tick(&mut self, physical_ms: u64) -> Hlc;
    pub fn observe(&mut self, remote: &Hlc, physical_ms: u64) -> Hlc;
}
```
Tests: tick is strictly increasing even when physical time goes backwards; observe of a future remote moves ahead; text ordering equals tuple ordering (property test); parse rejects malformed text.

#### Task 12 — Locales
Add every key used above to `locales/en.json` with plain English messages matching the prototype's wording where one exists (for example `fees.error.over_balance`: "That is more than the balance of {balance}."). Copy the same keys to `hi.json` with English text for now. A test fails if any key used in code is missing from `en.json` (collect keys with a constant list in `i18n.rs`).

### Do not
- No `chrono::Local::now()`, `Utc::now()`, `SystemTime`, `rand`, file or network access in this crate.
- No floating point anywhere in money or marks.
- No dependency on any other workspace crate.

### Done when (agent)
- [ ] `cargo test -p vidya-core` passes with all vectors above
- [ ] `grep -rn "now()\|SystemTime\|f32\|f64" crates/vidya-core/src` returns nothing
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Read the vectors table in `tests/vectors.rs`; spot-check 3 fee and 3 words results by hand

---

## P2.2 — Permission matrix

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P2.1 |
| Size | small |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/PERMISSIONS.md (entire, including "Office computer only"), docs/DECISIONS.md (D29, D31), crates/vidya-core/src/roles.rs |

### Goal
One source of truth for permissions in `vidya-core`, proven equal to `docs/PERMISSIONS.md` by a test, including the list of actions that phones may never perform, plus the scope helpers every service will use.

### Tasks

#### Task 1 — Action enum
In `crates/vidya-core/src/permissions.rs` create `pub enum Action` with one variant per row of the PERMISSIONS.md table, in the same order. Implement `Action::as_str()` returning the exact text (`"students.view"`), `Action::ALL: &[Action]`, `FromStr`, and `Action::office_computer_only(self) -> bool` (a match with every variant written out).

#### Task 2 — Access table
```rust
pub enum Access { Yes, No, Own }
pub const fn access(role: Role, action: Action) -> Access;   // a match with every combination written out explicitly (no wildcard arms)
```

#### Task 3 — Scope helpers
```rust
pub enum Scope { All, Sections(BTreeSet<String>) }
/// Can the actor do this at all? Returns the data scope the service must filter by.
pub fn scope(actor: &Actor, action: Action) -> Result<Scope, DomainError>;
/// Can the actor do this for this specific section?
pub fn authorize_section(actor: &Actor, action: Action, section_id: &str) -> Result<(), DomainError>;
/// For actions that are not about sections (fees.collect, users.manage ...)
pub fn authorize(actor: &Actor, action: Action) -> Result<(), DomainError>;
pub fn permission_names(role: Role, origin: Origin) -> Vec<&'static str>;   // actions with Yes or Own, minus office-only actions for phones, for the UI
```
Rules (checked in this order in all three functions):
- `office_computer_only()` and `actor.origin == Phone` → Permission error `permission.office_computer_only`.
- `No` → `DomainError` kind Permission, key `permission.denied`.
- `Own` with `scope` → `Scope::Sections(actor.section_ids.clone())`; if the teacher has no sections, return Permission error `permission.no_sections`.
- `Own` with `authorize_section` → allowed only if the section is in `actor.section_ids`.
- `authorize` on an `Own` action is a programming error: return Internal error `permission.needs_section` and add a debug assertion.

#### Task 4 — Test that the code equals the document
`crates/vidya-core/tests/permissions_doc.rs`:
1. `include_str!("../../../docs/PERMISSIONS.md")`.
2. Parse the first table: rows starting with `| ` whose first cell is an action name.
3. Parse the "Office computer only" section: lines starting with `` - ` `` up to the next heading.
4. Assert: the set of action names equals `Action::ALL`; for every row and role, `access()` equals the cell (`yes`/`no`/`own`); the office-only set equals the variants where `office_computer_only()` is true; every office-only name exists in the table.
5. On mismatch print a readable diff: action, role, document value, code value.

#### Task 5 — Behaviour tests
- Teacher with sections {V-A}: `authorize_section(marks.enter, "V-A")` ok, `"V-B"` denied; `scope(students.view)` returns Sections({V-A}); `authorize(fees.collect)` denied.
- Accountant: `scope(students.view)` All; `authorize(fees.cancel_receipt)` denied; `authorize(students.set_concession)` denied; `authorize(backup.run)` ok on the office computer and `permission.office_computer_only` from a phone.
- Principal: every action allowed with `Scope::All` on the office computer; from a phone every office-only action is refused and the rest are allowed.
- `permission_names(Teacher, OfficeComputer)` contains no action starting with `fees.`, `users.`, `settings.`, `backup.`, `reports.`.
- `permission_names(Accountant, Phone)` does not contain `backup.run`.

### Done when (agent)
- [ ] `cargo test -p vidya-core` passes, including the document test
- [ ] Changing one cell in PERMISSIONS.md, or removing a line from the office-only list, makes the test fail with a readable message (try both, then revert)
- [ ] `npm run verify` passes

---

## P2.3 — Encrypted database and migrations

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P2.1 |
| Size | large |
| Runs on | MacBook; Windows compile and tests in CI |
| Read first | AGENTS.md, crates/AGENTS.md, docs/DATA_MODEL.md (entire), docs/DEPENDENCIES.md, docs/SIZE.md, docs/DECISIONS.md (D28) |

### Goal
`crates/vidya-db` opens an SQLCipher-encrypted database with a raw 32-byte key, applies migrations, and enforces the schema exactly as DATA_MODEL.md. It builds on macOS, Windows (CI) and later Android, and stays inside the size budget.

### Dependencies allowed in this prompt
`rusqlite` with feature `bundled-sqlcipher-vendored-openssl` (or `bundled-sqlcipher` per target, see Task 1), `r2d2`, `r2d2_sqlite`, `thiserror`, `serde_json`, `zeroize`, `tempfile` (dev). **Use the dependency-verifier subagent** to confirm that the `r2d2_sqlite` version you add depends on the same `rusqlite` version, and that the feature names exist.

### Tasks

#### Task 1 — Size and build check first
1. Read the build script of the `libsqlite3-sys` version that `rusqlite` resolves to. Find out which crypto backend SQLCipher uses on each target with each feature: vendored OpenSSL, system OpenSSL, or Apple CommonCrypto. Write the findings in `docs/SIZE.md` notes.
2. Choose per target, using target-specific dependencies in `vidya-db/Cargo.toml`: if CommonCrypto is supported on Apple targets without vendored OpenSSL, use it on macOS (smaller, no OpenSSL build). Windows and Android use vendored OpenSSL. If you cannot confirm CommonCrypto from the source, use vendored OpenSSL everywhere and say so.
3. Add the dependencies to `vidya-db`. Write a tiny function `sqlcipher_version()` that opens an in-memory connection, runs `PRAGMA key = "x'00…'"` and returns `PRAGMA cipher_version` (and `PRAGMA cipher_provider`).
4. Call it from a debug-only log line at app start in `src-tauri` so the library is linked.
5. Build the release `.dmg` on the MacBook and compare with the size recorded in `docs/SIZE.md` in P1.3. Push and read the Windows installer size from CI. Add a row to `docs/SIZE.md`.
6. If either grows by more than 4 MB, or any artefact crosses the 25 MB warning line, **stop**, report the numbers and options, and wait for a decision.
7. If the Windows CI build fails because of OpenSSL building, report the exact error and stop.
**Check:** `cipher_version` and `cipher_provider` are not empty on the MacBook and in the Windows CI test output.

#### Task 2 — Migration file identical to the document
1. Create `crates/vidya-db/migrations/0001_init.sql` containing exactly the SQL block under "0001_init.sql" in DATA_MODEL.md.
2. Test `migration_matches_doc`: `include_str!` both files, extract the SQL block from the markdown (between the line `## 0001_init.sql`, the next ```` ```sql ```` line and the closing fence), normalise line endings, assert equal.

#### Task 3 — Opening the database
```rust
pub struct Db { pool: r2d2::Pool<SqliteConnectionManager>, write_lock: Mutex<()> }
pub enum DbError { WrongKey, NotEncrypted, Migration { version: u32, message: String }, Sqlite(rusqlite::Error), Pool(String), Io(std::io::Error) }
impl Db {
    pub fn open(path: &Path, key: &Zeroizing<[u8; 32]>) -> Result<Db, DbError>;
    pub fn open_in_memory_for_tests() -> Result<Db, DbError>;
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, DbError>;
    pub fn write<T>(&self, f: impl FnOnce(&Transaction) -> Result<T, DbError>) -> Result<T, DbError>;
}
```
- Connection setup, in this order, for **every** pooled connection (use the manager's init hook): `PRAGMA key = "x'<64 hex>'"` (build the hex string, then zeroize it); `PRAGMA cipher_memory_security = ON` if supported by the linked SQLCipher (verify); `SELECT count(*) FROM sqlite_master` (fails → `WrongKey`); `PRAGMA foreign_keys = ON`; `PRAGMA busy_timeout = 5000`.
- On open: `PRAGMA journal_mode = WAL` once; run migrations.
- `write` takes `write_lock`, starts `BEGIN IMMEDIATE`, commits on `Ok`, rolls back on `Err` or panic.
- Pool size 4.
- Never log the key or the hex string.

#### Task 4 — Migrations runner
- `migrations: &[(u32, &str)] = &[(1, include_str!("../migrations/0001_init.sql"))]`.
- Read `PRAGMA user_version`; apply each newer migration inside one transaction, then set `user_version`. On error roll back and return `DbError::Migration`.
- If `user_version` is newer than the newest migration known to the app, return `DbError::Migration` with message key `db.error.newer_version`.

#### Task 5 — Seed defaults
`pub fn seed_defaults(tx: &Transaction, now: &str, hlc: &str) -> Result<(), DbError>` inserts the defaults listed at the end of DATA_MODEL.md that do not depend on a school (grade scale, counters `adm_no` and `tc_no`, app settings including `backup_staff_can_run` and `backup_destinations`). Idempotent (`INSERT OR IGNORE`). Called by setup later, not on open.

#### Task 6 — Tests (`crates/vidya-db/tests/`)
1. `encrypted_on_disk`: create a DB in a temp dir, write a row, close; read the first 16 bytes of the file; they must **not** equal `SQLite format 3\0`.
2. `reopen_same_key` works; `reopen_wrong_key` returns `WrongKey`.
3. `migrations_idempotent`: open twice, `user_version` = 1, and the number of rows in `sqlite_master` with type `table` equals the number of `CREATE TABLE` statements in the migration plus `sqlite_sequence` (compute the expected count from the SQL text, and assert it is 36 for the current DATA_MODEL.md).
4. `newer_version_refused`: set `user_version` 99, reopen → Migration error.
5. Constraint tests (each insert must fail): second current session; second principal; username `Bad Name` and `sierra@x`; student mobile `98765`; UDISE `123`; mark with `absent = 0` and `value = NULL`; duplicate active roll in a section.
6. Trigger tests: UPDATE and DELETE on `receipts`, `receipt_cancellations`, `transfer_certificates`, `change_log` fail with the trigger message.
7. `write_rolls_back`: a closure that inserts then returns `Err` leaves no row.
8. `wal_enabled`: `PRAGMA journal_mode` returns `wal`.

### Do not
- Do not add repositories yet (P2.5).
- Do not put the key in any struct field that is `Debug`-printable.

### Done when (agent)
- [ ] Size impact recorded in `docs/SIZE.md` and within limits (or stopped for a decision)
- [ ] `cargo test -p vidya-db` passes on MacBook and in Windows CI
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Windows CI job is green, with the size step under 30 MB

---

## P2.4 — Platform secrets, folders and app start

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P1.3, P2.3 |
| Size | medium |
| Runs on | MacBook; Windows code compiled and tested in CI, checked in VM |
| Read first | AGENTS.md, src-tauri/AGENTS.md, docs/PLATFORMS.md, docs/DEPENDENCIES.md |

### Goal
The desktop app finds its data folder, creates or loads the database key from the operating system's secure storage, opens the encrypted database at start, and shows a clear screen if something goes wrong.

### Dependencies allowed in this prompt
Windows: `windows` (only features for DPAPI, known folders, memory freeing), `zeroize`. macOS: `security-framework`. Verify every function name with the dependency-verifier subagent before writing code. Enable only the `windows` crate features you use (each feature adds build time and can add size).

### Tasks

#### Task 1 — macOS implementation (`platform/macos.rs`)
- `data_dir()`: the Tauri app data directory for identifier `in.vidya.school` + `/data` (get it through the app handle passed at construction; verify it resolves to `~/Library/Application Support/in.vidya.school`). Create with permissions `0700`.
- `backups_dir()`: sibling `/backups`, `0700`.
- `load_or_create_db_key()`: Keychain generic password, service `in.vidya.school.dbkey`, account `database`. If missing, create 32 random bytes from the OS generator and store them. Try to set accessibility "after first unlock, this device only"; if the installed `security-framework` cannot set it, use the default and add a KNOWN_ISSUES item explaining the default is not iCloud-synced unless marked synchronizable.
- `store_secret`, `load_secret`, `delete_secret`: service `in.vidya.school.<name>`.
- Wrap returned keys in `Zeroizing`.
- Note in a comment: data is per Mac user account; the school must use the same Mac account (also add to README).

#### Task 2 — Windows implementation (`platform/windows.rs`)
- `data_dir()`: known folder ProgramData (use the Windows API, not a hard-coded `C:`), + `\Vidya\data`. `backups_dir()`: `\Vidya\backups`.
- `load_or_create_db_key()`: if `data\key.bin` exists, `CryptUnprotectData` it; otherwise create 32 random bytes, `CryptProtectData` with the local-machine flag and optional entropy `b"vidya-db-key-v1"`, write to `key.bin.partial`, flush, rename to `key.bin`. Free any memory the API allocates with the function its documentation names.
- Secrets: DPAPI blobs in `data\secrets\<name>.bin`, same entropy pattern with the name.
- Integration test `#[cfg(windows)]` in `src-tauri/tests/windows_dpapi.rs`: protect then unprotect round trip; tampered blob fails. Runs in CI.
- Add to PROGRESS.md "Check in Windows VM": install the CI build, start Vidya, confirm `C:\ProgramData\Vidya\data\vidya.db` and `key.bin` exist, restart and confirm it opens, copy both files to another VM user profile location and confirm they still open on the same machine.

#### Task 3 — Android placeholder
`platform/android.rs` keeps returning `Unsupported` (P8.2 implements). Make sure desktop builds do not compile it.

#### Task 4 — Start sequence (`src-tauri/src/lib.rs` + `state.rs`)
```rust
pub enum StartupState { Ready(Arc<AppCore>), Failed { kind: StartupFailure, detail_for_log: String } }
pub enum StartupFailure { DataFolder, SecureStorage, WrongKey, Migration, Unknown }
pub struct AppCore { pub db: Db, pub platform: Box<dyn Platform>, pub data_dir: PathBuf }
```
In the Tauri `setup` hook: platform → data dir → key → `Db::open(data_dir/vidya.db, key)` → store `StartupState` in managed state. Do not panic on failure. Run the database opening on a blocking thread; the React app shows an "Opening Vidya…" state until `app_status` answers.

#### Task 5 — `app_status` command (first real command)
- Rust command `app_status` returning `AppStatusDto { ready: bool, failure: Option<String>, has_school: bool, platform: String, version: String }`; `has_school` from `SELECT count(*) FROM school`. (`serverAllowed` and license fields come in P2.7 and P4.2.)
- JS: `appStatus()` in `commands.js` now calls `invoke('app_status')`. If `ready` is false, `DesktopApp` renders `<StartupError kind=… />` (new component `src/views/StartupError.jsx`), a full-screen message per failure kind (keys `startup.error.*`, in en and hi JSON):
  - DataFolder: "Vidya could not create its data folder. Restart the computer and try again."
  - SecureStorage: "Vidya could not open the computer's secure storage."
  - WrongKey: "Vidya's data could not be unlocked on this computer. If the computer was replaced, restore a backup."
  - Migration: "This data was made by a newer version of Vidya. Install the latest version."
  With a "Copy details" button (details from the log message, no personal data).
- Add `app_status` to docs/API.md if the output fields changed (update the documented DTO).
- Until P2.7, if `ready` and not `has_school`, keep using the mock welcome flow.

#### Task 6 — Tests
- `FakePlatform` test: start sequence with a temp dir creates `vidya.db`; second start opens it; replacing the fake key produces `WrongKey`.
- React Testing Library: `StartupError` shows the right message per kind.
- macOS Keychain integration test in `src-tauri/tests/macos_keychain.rs`, run only when `VIDYA_KEYCHAIN_TEST=1` is set (Keychain prompts can block CI). Document how the developer runs it: `VIDYA_KEYCHAIN_TEST=1 cargo test -p vidya-app --test macos_keychain`.

### Do not
- Never write the raw key to disk on macOS, or unprotected on Windows.
- Never include key bytes, hex or blobs in logs or error messages.

### Done when (agent)
- [ ] `npm run verify` passes on MacBook and CI (Windows test included)
- [ ] Windows VM checks added to PROGRESS.md

### Done when (developer)
- [ ] `npm run tauri dev`: `~/Library/Application Support/in.vidya.school/data/vidya.db` appears; `sqlite3` on that file says it is not a database
- [ ] `VIDYA_KEYCHAIN_TEST=1 cargo test -p vidya-app --test macos_keychain` passes (allow Keychain access if asked)
- [ ] Keychain Access app shows the item `in.vidya.school.dbkey`
- [ ] Windows VM checks in PROGRESS.md done

---

## P2.5 — Services foundation, repositories and change log

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P2.2, P2.3 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, crates/AGENTS.md, docs/ARCHITECTURE.md (sections 2, 6, 7), docs/DATA_MODEL.md, docs/API.md ("Error format"), docs/SYNC_PROTOCOL.md (sections 2 and 3), docs/TESTING_STRATEGY.md ("Test data") |

### Goal
The single path every action will use: a `Services` container with injectable clock, IDs and randomness; `ServiceError`; a change-log writer; repositories for the base tables; password hashing; and a deterministic sample school for tests and debug builds.

### Dependencies allowed in this prompt
`uuid` (v7, serde), `rand`, `argon2`, `chrono`, `serde`, `serde_json`, `thiserror`, `zeroize`, `tracing`; dev: `tempfile`, `insta`. Use `default-features = false` where the crate allows it and record size in `docs/SIZE.md`.

### Tasks

#### Task 1 — Injected environment (`vidya-services/src/env.rs`)
```rust
pub trait Clock: Send + Sync { fn now_utc(&self) -> DateTime<Utc>; fn today_local(&self) -> NaiveDate; fn now_ms(&self) -> u64; }
pub trait IdGen: Send + Sync { fn new_id(&self) -> String; }              // UUID v7 text
pub trait Random: Send + Sync { fn fill(&self, buf: &mut [u8]); }
pub struct SystemClock; pub struct UuidV7; pub struct OsRandom;             // production
pub struct FixedClock { .. } pub struct SeqIds { .. } pub struct SeededRandom { .. } // tests, deterministic
```
`today_local` uses the computer's local timezone in production; tests use a fixed date.

#### Task 2 — Errors (`error.rs`)
```rust
#[derive(Debug, thiserror::Error)]
pub struct ServiceError { pub kind: ErrorKind, pub message_key: String, pub params: BTreeMap<String, String>, pub field: Option<String>, #[source] pub source: Option<Box<dyn std::error::Error + Send + Sync>> }
```
- `From<DomainError>`, `From<DbError>` (WrongKey → Internal with key `db.error.unlock`; constraint violations → Internal unless mapped; never expose SQL text in `message_key` or params).
- `fn to_dto(&self, lang: Lang) -> ErrorDto { kind, message_key, params, message, field }` using `vidya_core::i18n::message`.
- Internal errors get a random short reference (`ERR-7K2Q`) in params and are logged with the source; the message says "Something went wrong (ERR-7K2Q). Please try again."

#### Task 3 — Container (`lib.rs`)
```rust
pub enum Mode { Server, Client }
pub struct Services { pub db: Arc<Db>, pub clock: Arc<dyn Clock>, pub ids: Arc<dyn IdGen>, pub random: Arc<dyn Random>, pub mode: Mode, pub device_id: String, hlc: Mutex<HlcClock> }
impl Services {
    pub fn new(db, clock, ids, random, mode, device_id) -> Result<Self, ServiceError>; // loads meta.hlc_last
    pub fn school(&self) -> SchoolService<'_>;  pub fn settings(&self) -> SettingsService<'_>;  // more added in later prompts
}
```
Service structs borrow `&Services`. Each area lives in its own module under `src/services/`.

#### Task 4 — Change log writer (`change_log.rs`)
```rust
pub struct ChangeRecord<'a> { pub kind: &'a str, pub entity: &'a str, pub entity_id: &'a str, pub op: Op, pub summary_key: &'a str, pub params: serde_json::Value, pub payload: serde_json::Value }
pub fn record(services: &Services, tx: &Transaction, actor: Option<&Actor>, rec: ChangeRecord) -> Result<ChangeMeta, ServiceError>; // returns { seq, change_id, hlc }
```
- `change_id` = `<device_id>:<counter>` where counter is `meta.change_counter` incremented in the same transaction.
- `hlc` from `HlcClock::tick(clock.now_ms())`; store `meta.hlc_last` in the same transaction.
- Refuse (Internal error) if `payload` contains any key named `password`, `passwordHash`, `password_hash`, `key`, `secret` at any depth (test this).

#### Task 5 — Repositories (`vidya-db/src/repo/`)
One module per group, plain functions taking `&Connection` (reads) or `&Transaction` (writes), typed row structs, parameterised SQL only:
- `meta.rs`: get, set, `next_counter(tx, name) -> i64` using `UPDATE counters SET value = value + 1 WHERE name = ?1 RETURNING value` (verify the bundled SQLite version supports RETURNING; if not, SELECT then UPDATE inside the transaction) and insert if missing.
- `school.rs`, `sessions.rs` (current session), `classes.rs` (classes + sections), `fee_plans.rs`, `subjects.rs`, `exams.rs`, `grade_scale.rs`, `app_settings.rs`, `users.rs` (insert, get by id, get by username, list, update fields, set sections), `students.rs` + `enrollments.rs` (insert, get, list by session and section, update fields), `receipts.rs` (insert, list by student and session, list by date, sum paid excluding cancellations), `cancellations.rs`, `attendance.rs`, `marks.rs`.
- Every write that changes a syncable row sets `updated_hlc`.
- Repository tests with an in-memory encrypted test DB: insert and read back one row per table.

#### Task 6 — Password hashing (`vidya-services/src/auth/password.rs`)
- Argon2id, memory 19,456 KiB, iterations 2, parallelism 1, random 16-byte salt from `Random`, PHC string output.
- `hash_password(services, &str) -> Result<String>`, `verify_password(hash, &str) -> bool`, `dummy_verify(&str)` using a hash computed once at start so unknown usernames take similar time.
- Zeroize password buffers after use where you own them.
- Tests: verify ok, wrong password false, hashes of the same password differ, PHC string starts with `$argon2id$`.

#### Task 7 — First read services
- `SchoolService::header(actor) -> SchoolHeaderDto` (any signed-in role).
- `SettingsService::get(actor) -> SettingsDto` (settings.view): school, current session, classes with sections, fee plans, subjects, exams, grade scale, app settings.
- Each method: `authorize` or `scope` first, then read.

#### Task 8 — Sample school (`vidya-testkit`)
`SampleSchool::build(services: &Services, seed: u64) -> Result<SampleLogins, ServiceError>` creates, through repositories inside one transaction, the same school as the prototype's sample: Vaani Public School, code `vaani`, session from `default_session_name(today)`, classes Nursery–VIII (Nursery one section, others A and B), fees and subjects from DATA_MODEL defaults, users sunita (principal), sierra (teacher V-A, V-B, name "Sierra D'Souza"), rakesh (teacher VI-A, VI-B, VII-A), anita (accountant), all password `vidya123`, `must_change = 0`; about 136 students with enrollments using a deterministic pseudo-random generator from the seed; receipts; six working days of attendance before today; Unit Test 1 marks. Also inserts a `license` row of type demo for school code `vaani` and device ID `VD-DEMO-DEMO-DEMO`.
Tests: same seed → identical counts and first student names; different seed → different names.
Add a snapshot test (`insta`) of `SettingsDto` for the sample school.

### Do not
- No Tauri types in `vidya-services`.
- No business rules in repositories (only storage).
- No `unwrap()` on database results in non-test code.

### Done when (agent)
- [ ] `cargo test -p vidya-db -p vidya-services -p vidya-testkit` passes
- [ ] A test shows a change-log payload containing `passwordHash` is refused
- [ ] `npm run verify` passes

---

## P2.6 — Sign-in and sessions

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P2.5 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/PERMISSIONS.md (rule 7 and "Office computer only"), docs/API.md ("Auth and account"), reference prototype sign-in, lockout and forced password change behaviour |

### Goal
`AuthService` in `vidya-services`: sign-in with lockout, pending tokens for first sign-in, sessions with idle timeout, password change, and the `Actor` (with its origin) every other service receives.

### Tasks

#### Task 1 — Session store (`auth/sessions.rs`)
```rust
pub struct SessionStore { inner: Mutex<HashMap<TokenHash, SessionEntry>> }
struct SessionEntry { user_id: String, kind: SessionKind /* Full | Pending */, created_ms: u64, last_active_ms: u64, device_id: String, origin: Origin }
```
- Tokens: 32 random bytes → `format_session_token`. Store only `SHA-256(token)` as the map key (so a memory dump of the map does not reveal tokens).
- `create_full`, `create_pending` (pending expires after 10 minutes), `resolve(token, now_ms, timeout_ms) -> Result<(user_id, kind, device_id, origin), ServiceError>` updating `last_active_ms`, `end(token)`, `end_all_for_user(user_id)`, `expire_idle(now_ms, timeout_ms) -> Vec<String>` returning user ids whose sessions ended.
- Timeout from `app_settings.session_timeout_minutes` (default 30); phones will use 15 (P8.4).

#### Task 2 — Sign-in (`auth/service.rs`)
```rust
pub enum SignInResult { Ok { token: String, user: CurrentUserDto }, MustChangePassword { pending_token: String } }
pub fn sign_in(&self, username_input: &str, password: &str, device_id: &str, origin: Origin) -> Result<SignInResult, ServiceError>;
```
Exact behaviour:
1. Normalise: trim, lowercase. If it contains `@`, the part after `@` must equal the licensed school code, else error kind Auth key `auth.error.wrong_school` param `code`.
2. Load user by username. If missing or inactive: run `dummy_verify`, return Auth `auth.error.no_login` ("No active login with that username.").
3. If `locked = 1`: Locked `auth.error.locked` ("This login is locked after 5 wrong passwords. Ask the principal to unlock it.").
4. If `locked_until` is in the future (principal): Locked `auth.error.paused` with param `time`.
5. Verify password. Wrong: increment `failed_count`. At 5: staff → `locked = 1`; principal → `locked_until = now + 5 minutes`; reset count to 0; change-log event (`auth.log.locked` / `auth.log.paused`). Otherwise Auth `auth.error.wrong_password` with param `left`. Log each failed attempt as an event without the password.
6. Correct: `failed_count = 0`, `locked_until = NULL`. If `must_change = 1` → create pending token, return `MustChangePassword` (no `last_login_at` update yet).
7. Otherwise set `last_login_at`, create full session, change-log event `auth.log.signed_in`, return `Ok` with `CurrentUserDto { id, name, username, full_username, role, sections, language, permissions: permission_names(role, origin) }`.
All database updates in one `write` transaction.
`origin` is `OfficeComputer` only when `device_id` equals `meta.device_id` of this database **and** the call comes from the local command layer. The HTTP layer (P7.3) always passes `Phone`. Add a test that a phone-origin sign-in with the office device id still gets `Phone`.

#### Task 3 — First password, change password, sign out
- `set_first_password(pending_token, new_password)`: resolve pending; validate with `validate_new_password(new, username, verify(current_hash, new))`; hash; `must_change = 0`; `password_changed_at`; `last_login_at`; end pending; create full session with the same origin; event `auth.log.first_password`. Returns the same `Ok` result as sign-in.
- `change_password(actor, current, new)`: permission `account.change_own_password`; wrong current → Auth `auth.error.current_wrong`; validate; hash; end all **other** sessions of the user; event.
- `sign_out(token)`: end session; event.
- `set_language(actor, lang)`: update user; event.

#### Task 4 — Actor resolution
`pub fn actor_for_token(&self, token: &str) -> Result<Actor, ServiceError>`:
- Resolve full session (pending → Auth `auth.error.password_change_required`).
- Load user **fresh from the database** every time (role, active, locked, sections). Inactive or locked → end sessions, Auth `auth.error.signed_out`.
- Build `Actor` with current sections from `user_sections` and the origin stored in the session.
Expired idle session → Auth `auth.error.session_expired` ("You were signed out after {minutes} minutes without activity.").

#### Task 5 — Tests (`vidya-services/tests/auth.rs`, using the sample school and `FixedClock` that tests can advance)
1. sunita / vidya123 → Ok with principal permissions (office computer list includes `backup.manage`).
2. `sunita@vaani` works; `sunita@other` → wrong_school.
3. Unknown user and inactive user → no_login.
4. 4 wrong passwords then correct → Ok and count reset.
5. 5 wrong for sierra → locked; correct password still locked.
6. 5 wrong for sunita → paused; after advancing clock 5 minutes → Ok.
7. User with `must_change = 1` → MustChangePassword; pending token rejected by `actor_for_token`; `set_first_password` with password containing the username → error; with valid password → Ok.
8. Idle: advance clock 31 minutes → session_expired.
9. Switching a user inactive in the DB → next `actor_for_token` fails and sessions end.
10. Change-log entries never contain the password or hash (search the payload JSON).
11. Actor from a Phone-origin session: `authorize(backup.run)` fails with `permission.office_computer_only`.

### Done when (agent)
- [ ] All tests above pass
- [ ] Use the security-reviewer subagent on `crates/vidya-services/src/auth/` and fix findings
- [ ] `npm run verify` passes

---

## P2.7 — Command layer and first real screens

| | |
|---|---|
| Phase | 2 Core and database |
| Depends on | P2.4, P2.6 |
| Size | medium |
| Runs on | MacBook; Windows via CI |
| Read first | AGENTS.md, src-tauri/AGENTS.md, src/AGENTS.md, docs/API.md (conventions, error format, app and auth sections, events) |

### Goal
The pattern every later command follows, implemented for app status, sign-in, account and settings reading. The desktop app signs in against the real encrypted database. Feature screens still use the mock until their prompts.

### Tasks

#### Task 1 — App state and error type
- `state.rs`: `AppState { core: StartupState, services: Option<Arc<Services>>, sessions: Arc<SessionStore> }`. Build `Services` in `Mode::Server` with `SystemClock`, `UuidV7`, `OsRandom`, device id from `meta.device_id` (create a UUID v7 on first start and store it).
- `commands/error.rs`: `#[derive(Serialize)] struct AppError { kind, message_key, params, message, field }` implementing `From<ServiceError>` using the actor's language, or English before sign-in. Commands return `Result<T, AppError>`.

#### Task 2 — Command pattern
Write this helper and use it in every command from now on:
```rust
pub async fn with_actor<T: Send + 'static>(
    state: tauri::State<'_, AppState>, token: String,
    f: impl FnOnce(&Services, &Actor) -> Result<T, ServiceError> + Send + 'static,
) -> Result<T, AppError>   // resolves actor, runs f on a blocking thread, maps errors
```
and `without_actor` for no-session commands. No command body longer than about 10 lines. Desktop commands always sign in with `Origin::OfficeComputer`; there is no parameter that lets the webview choose the origin.

#### Task 3 — Commands
Implement in `src-tauri/src/commands/app.rs` and `auth.rs` and register in `generate_handler!`:
`app_status` (extend DTO to `AppStatusDto` in API.md: add `licensed`, `school_name`, `school_code`, and `server_allowed`, which is always `false` until P4.2), `sign_in`, `set_first_password`, `sign_out`, `current_user`, `change_password`, `set_language`, `get_settings`, and `load_sample_school` compiled only with `#[cfg(debug_assertions)]` and refusing if a school already exists.
`sign_in` passes the office computer's own device id.

#### Task 4 — Idle session event
A background task (desktop) every 60 seconds calls `expire_idle`; if the signed-in user's session ended, emit `session-expired`. The frontend listens (in `SessionProvider`, with the listener removed on unmount) and returns to the sign-in screen with the translated message.

#### Task 5 — Frontend wiring
1. `src/api/commands.js`: change `appStatus`, `signIn`, `setFirstPassword`, `signOut`, `currentUser`, `changePassword`, `setLanguage`, `getSettings`, `loadSampleSchool` to call `invoke` (token added from `session.js`). Convert rejected values with `toAppError`.
2. The welcome screen's "Load a sample school" button calls the real `loadSampleSchool` in debug builds and is not rendered in release builds (`import.meta.env.DEV`, so the code is removed from the production bundle).
3. `Login.jsx`, `Password.jsx`, the account modal, the language switch and sign-out use the real commands through `useMutation`. `I18nProvider` takes the language from the user record after sign-in.
4. The mock feature modules must use the real `currentUser()` for role and sections. Add a KNOWN_ISSUES item: "Features other than sign-in and settings reading still use mock data until P3.1–P4.4".
5. `session.js` keeps the token in memory only (never localStorage, sessionStorage or React state that is persisted).

#### Task 6 — API drift
Update docs/API.md for any DTO field changes. `node scripts/check-api-drift.mjs` must pass with these commands present in Rust, JS and docs.

#### Task 7 — Tests
- Rust: command-level tests using the Tauri mock runtime if available in the installed version (verify); otherwise test `with_actor` with a fake state.
- React Testing Library: `Login` shows `AppError.message` for a wrong password; `Password` shows the field error; a `session-expired` event (mocked `listen`) returns to `Login`.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Commands above exist in Rust, JS and API.md

### Done when (developer)
- [ ] `npm run tauri dev` → Load sample school → sign in as sunita / vidya123 works
- [ ] Wrong password 5 times for sierra locks the login; the message matches the prototype
- [ ] Quit and reopen the app: the sample school is still there (real database)
- [ ] Changing language to हिं persists after sign-out and sign-in

---

## P3.1 — Staff logins and the permission test harness

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P2.7 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/PERMISSIONS.md, docs/API.md ("Staff logins"), docs/TESTING_STRATEGY.md, prototype "Staff logins" screen |

### Goal
Principal manages teacher and accountant logins through `UserService`, and a reusable test harness proves every service method obeys the permission table, including the office-computer-only rule. Later prompts add their methods to the same harness.

### Tasks

#### Task 1 — Permission matrix harness (`vidya-services/tests/permissions_matrix.rs`)
- Build the sample school once per test with a fresh in-memory DB.
- Actors: `principal` (sunita), `accountant` (anita), `teacher_own` (sierra acting on V-A), `teacher_other` (sierra acting on VI-A), each with `Origin::OfficeComputer`, plus `principal_phone` and `accountant_phone` with `Origin::Phone`. "No session" is checked at command level later.
- A macro `case!(name, action: Action, section: Option<&str>, |services, actor| { call })` that runs the call for each actor and asserts: allowed when `access()` is Yes, or Own with teacher_own, **and** the action is not office-only for a phone actor; otherwise the error kind is Permission (with key `permission.office_computer_only` for the phone case). Allowed calls may fail with validation errors (count as allowed).
- A final test asserts every `Action` in `Action::ALL` has at least one case, **except** actions listed in a `NOT_YET_IMPLEMENTED` constant with the prompt that will add them. Each later prompt must remove its actions from that list.

#### Task 2 — UserService (`services/users.rs`)
| Method | Rules |
|---|---|
| `list(actor) -> Vec<UserDto>` | users.view. DTO: id, name, username, full_username, role, mobile, section names, status (`active`, `must_change`, `locked`, `switched_off`), last_login_at. Never the hash |
| `create(actor, CreateUserInput) -> CredentialSlipDto` | users.manage. Role only teacher or accountant (principal → Permission `users.error.principal_once`). Name via `validate_person_name`. Mobile empty or valid. Teacher needs ≥1 existing active section (`users.error.no_sections`). Active users < license max_users (`users.error.limit` param `max`). Username: if `username` given, `validate_username` and must be free (`users.error.username_taken`); else `username_base_from_name` or `fallback_username_base`, then `unique_username`. Temporary password from 8 random bytes → `format_temp_password`. `must_change = 1`. Change-log event (`users.log.created`, no password). Returns slip with the temporary password (the only time it is returned) |
| `update(actor, UpdateUserInput) -> UserDto` | users.manage. Name, mobile, sections (teachers only). Principal can update own name and mobile but has no sections |
| `reset_password(actor, user_id) -> CredentialSlipDto` | users.manage. Not for principal (`users.error.principal_self`). New temporary password, `must_change = 1`, `locked = 0`, `failed_count = 0`. End that user's sessions |
| `unlock(actor, user_id) -> UserDto` | users.manage. `locked = 0`, `failed_count = 0` |
| `set_active(actor, user_id, active) -> UserDto` | users.manage. Principal cannot be switched off. Switching on respects max_users. Switching off ends sessions |
All writes: one transaction and a change-log entry with `kind = "user"`.

#### Task 3 — Commands and frontend
- Commands: `list_users`, `create_user`, `update_user`, `reset_user_password`, `unlock_user`, `set_user_active`.
- `views/Users.jsx` uses real commands through `useQuery`/`useMutation`; the slip modal prints the `Slip` component with `usePrint()`. Remove `src/api/mock/users.js` and its uses.
- Add matrix cases for every method, and for `account.change_own_password` and `account.set_own_language` (from P2.6).

#### Task 4 — Tests
- Creating "Sierra Khan" when sierra exists → `sierra2`.
- Creating a principal role → Permission error.
- 41st active login with max 40 → limit error.
- Reset password: old password fails at sign-in; temporary works and requires change.
- Switching off sierra ends her session (actor_for_token fails).
- Temporary password never appears in change_log.
- `principal_phone` calling `create` → `permission.office_computer_only`.

### Done when (agent)
- [ ] Harness in place and passing; `NOT_YET_IMPLEMENTED` lists remaining actions with prompt ids
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] As sunita: add teacher "Meera Joshi" with class III-A, print the slip, sign out, sign in as meera with the temporary password, set a new one
- [ ] Reset Meera's password; old one stops working

---

## P3.2 — Students and admissions

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.1 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md (students, enrollments), docs/PERMISSIONS.md (DTO visibility), docs/API.md ("Students"), prototype Students screens and form |

### Goal
Real student records with admission numbers, enrollments in the current session, role-shaped views, left-school handling and duplicate warnings.

### Tasks

#### Task 1 — DTOs by role (`dto/students.rs`)
Three separate structs so fields cannot leak:
- `StudentTeacherDto`: id, adm_no, name, gender, dob, father, mother, mobile, locality, class_name, section_name, section_id, roll, status.
- `StudentOfficeDto`: everything in the teacher DTO plus category, rte, transport, aadhaar_collected, apaar_created, admitted_on, left_on, left_reason, concession, fee summary (`due`, `paid`, `balance`, `state`).
- `StudentListItemDto` enum `#[serde(tag = "shape")]` with `Teacher(..)` and `Office(..)` list variants; same for detail.
A test: a function that takes `StudentTeacherDto` and serialises it must not contain the keys `rte`, `concession`, `due`, `paid`, `balance`, `category` (assert on the JSON).

#### Task 2 — StudentService (`services/students.rs`)
| Method | Rules |
|---|---|
| `list(actor, filter) -> Vec<StudentListItemDto>` | `scope(students.view)`. Filter: q (name, adm_no, father, mobile, roll exact), section_id, status (default active). Teachers: sections in scope only; a section_id outside scope → Permission. Sorted by class sort order, section, roll. Max 500 rows with `truncated` flag in a wrapper DTO |
| `get(actor, student_id) -> StudentDetailDto` | Section of the student's current enrollment must be in scope. Office detail includes receipts (id, receipt_no, paid_on, amount, mode, cancelled) |
| `add(actor, StudentInput) -> StudentDetailDto` | students.add; `Mode::Client` → Offline error `students.error.online_only`. Validate: name, gender, class+section exist and active, father, mobile, dob not future, category. `concession` present and not 0 from non-principal → Permission. Duplicate check: active student with same name and father (case-insensitive) → Conflict `students.possible_duplicate` with params name, class, adm_no, unless `confirm_duplicate`. In one transaction: `next_counter("adm_no")` → `ADM/0001` (4 digits, grows beyond 9999), insert student, enrollment in current session with roll = max roll in section + 1 (among all statuses, so rolls are not reused in the session), change log `students.log.admitted` |
| `update(actor, UpdateStudentInput) -> StudentDetailDto` | students.edit; concession change needs students.set_concession. Class or section change gives the next roll in the new section. Only changed fields written; change log with field names (not values of mobile) |
| `mark_left(actor, student_id, left_on, reason)` | students.mark_left. Date not future, reason 2+ characters. Sets enrollment status `left`. Undo by `update` status back to active (students.edit), which assigns the next free roll if the old one is taken |

#### Task 3 — Commands and frontend
- Commands: `list_students`, `get_student`, `add_student`, `update_student`, `mark_student_left`. (`export_students_xlsx` comes in P3.7, import in P4.5.)
- `views/students/*.jsx` use real commands. The list renders the office or teacher columns from the DTO `shape`, never from the role. `StudentForm` handles `students.possible_duplicate` by awaiting `confirm()` and resending with `confirmDuplicate: true`; the concession input renders only when `can('students.set_concession')`.
- Remove student parts from `src/api/mock/` (keep fee mock until P3.3 but make it read students from the real commands).

#### Task 4 — Tests
- Admission numbers continue after restart (reopen DB) and are never reused.
- Roll numbers: add three to V-A → 1, 2, 3; move roll 2 to V-B → next V-B roll; add another to V-A → 4.
- Teacher list for V-A contains only V-A/V-B students and no fee keys in JSON.
- Accountant setting concession → Permission; principal → ok.
- Client mode add → Offline error.
- Duplicate flow: first call Conflict, second with confirm → ok.
- React Testing Library: duplicate confirm flow resends with the flag.
- Matrix cases for all methods; remove `students.view`, `students.view_fees`, `students.view_contact`, `students.add`, `students.edit`, `students.set_concession`, `students.mark_left` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] As anita: add a student; admission number appears; possible duplicate warning works
- [ ] As sierra: Students shows only V-A and V-B, with no fee column
- [ ] As sunita: give a concession; as anita the concession field is not shown

---

## P3.3 — Fees, receipts and day book

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.2 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md ("Derived values", receipts), docs/PERMISSIONS.md, docs/API.md ("Fees"), crates/vidya-core/src/fees.rs, prototype Fees screens, receipt and day book |

### Goal
Correct fee collection with append-only receipts, per-device receipt series, cancellations by the principal, fee register and day book, all matching the prototype's behaviour.

### Tasks

#### Task 1 — Fee calculation helper (`services/fees/account.rs`)
`fn fee_account(conn, session_id, student_id) -> FeeAccount { due, paid, balance, state, term_fee, one_term, receipts }` using `vidya-core::fees` and the enrollment (rte, transport, concession), fee plan for the class in the session, session terms and transport fee. `paid` excludes cancelled receipts. Also `previous_session_dues` from the previous session's enrollment if any.

#### Task 2 — FeeService (`services/fees/mod.rs`)
| Method | Rules |
|---|---|
| `register(actor, FeeFilter) -> FeeRegisterDto` | fees.view. Rows for active enrollments in the current session: student, class-section, due, paid, balance, state, previous dues. Filter by q, section, state (`dues` = due or part, `paid`, `rte`, `all`). Totals: due, collected (min(paid, due) per student), pending, counts unpaid and part, collected today and receipts today |
| `account(actor, student_id) -> FeeAccountDto` | fees.view |
| `collect(actor, CollectInput) -> ReceiptDto` | fees.collect. In one write transaction: load account; `validate_payment` with `upi_reference_already_used` from `receipts` joined with no cancellation for mode UPI; device code from `meta.device_code` (office computer) or the approved phone prefix (client mode, P8.4); `next_counter("receipt:<prefix>")`; `receipt_number`; insert receipt with `balance_after = balance − amount`, `paid_on = today_local`, `hlc`; change log `fees.log.collected` (op append) with amount, mode, receipt_no, student name |
| `get_receipt(actor, receipt_id) -> ReceiptDto` | fees.view. DTO: receipt fields, student name, adm_no, father, class-section, roll, school header, amount in words from `receipt_words_en`, cancelled info, `received_by_name` |
| `cancel(actor, receipt_id, reason) -> ReceiptDto` | fees.cancel_receipt. Reason ≥ 4 characters (`fees.error.cancel_reason`). Already cancelled → Conflict `fees.error.already_cancelled`. Insert cancellation, change log |
| `day_book(actor, date) -> DayBookDto` | fees.daybook. Date not future. All receipts that date (cancelled included and flagged), totals per mode for non-cancelled, total, count, cancelled count and amount |

#### Task 3 — Alerts for overpayment (server mode)
After inserting a receipt, if the student's paid exceeds due, insert an `alerts` row kind `overpayment` (this can happen after concession changes or sync). Add `list_alerts` and `resolve_alert` service methods (alerts.view; accountants see only overpayment alerts).

#### Task 4 — Commands and frontend
- Commands: `fee_register`, `get_fee_account`, `collect_fee`, `get_receipt`, `cancel_receipt`, `day_book`, `list_alerts`, `resolve_alert`.
- Views `views/fees/*.jsx` and the student detail fee card use real commands. Receipt print uses the `ReceiptCopy` component in the prototype's two-copy layout through `usePrint()` (P5.2 upgrades printing). In `CollectFee`, the "Full balance" and "One term" buttons fill the amount from `balance` and `one_term` in the account DTO, not from JavaScript maths.
- Remove the fee mock.

#### Task 5 — Tests
- Vectors from P2.1 through the service with real enrollments: bus + 3 terms, concession, RTE (collect refused), 12 terms.
- Overpay refused; zero balance refused; UPI reference 12 digits; same UPI reference refused while the first receipt is active, allowed after it is cancelled.
- Receipt numbers: PC-0001, PC-0002; after changing device code to T1: T1-0001; restart continues.
- Direct `UPDATE receipts` in a test fails (trigger).
- Cancel: principal only; balance restored; day book shows cancelled separately.
- Day book totals per mode.
- React Testing Library: the over-balance error appears under the amount field.
- Matrix cases; remove `fees.view`, `fees.collect`, `fees.cancel_receipt`, `fees.daybook`, `alerts.view` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] As anita: collect ₹500 cash, UPI with a 12-digit ID, and a cheque; receipts PC-0001…PC-0003 print with amounts in words
- [ ] Over-balance amount shows the prototype's message
- [ ] As sunita: cancel a receipt with a reason; the day book shows it struck through and excluded from totals

---

## P3.4 — Attendance

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.2 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md (attendance tables), docs/PERMISSIONS.md (rules 3 and 4), docs/API.md ("Attendance"), prototype Attendance screen and monthly register |

### Goal
Attendance per section and date with the prototype's tap flow, teachers limited to today and their sections, principal corrections, and the monthly register.

### Tasks

#### Task 1 — AttendanceService (`services/attendance.rs`)
| Method | Rules |
|---|---|
| `sheet(actor, section_id, date) -> AttendanceSheetDto` | `authorize_section(attendance.view)`. Active students of the section in the current session ordered by roll; saved marks; saved_by name and saved_at; `read_only_reason` computed with `check_attendance_date(date, today, can(attendance.edit_past))` and permission for marking (null if editable) |
| `save(actor, SaveAttendanceInput) -> AttendanceSheetDto` | Date today → `authorize_section(attendance.mark_today)`; past → `attendance.edit_past`; future → Validation. Every active student in the section must have a status (`attendance.error.not_marked` with count and first three names); student ids not in the section → Validation. In one transaction: upsert `attendance_days` (same id if exists), replace all `attendance_marks` for that day, change log op `replace_set` with key `attendance.log.saved` or `attendance.log.corrected` and counts |
| `register(actor, section_id, month) -> RegisterDto` | `authorize_section(attendance.print_register)`. Days in month, per student statuses per day, present count per student |
| `today_summary(actor) -> TodaySummaryDto` | Used by home screens: per section in scope, marked or not, counts |

#### Task 2 — Commands and frontend
- Commands: `get_attendance`, `save_attendance`, `attendance_register`.
- `views/Attendance.jsx` keeps the draft map (which card is P/A/L before saving) in component state with `useReducer`; "Mark all present" and tap cycling (using the same order as `next_status`: none → P → A → L → P) are UI state, allowed in JS; saving sends the full map. The draft resets when the section or date changes. The read-only note shows `read_only_reason`. The monthly register prints from `RegisterDto`.
- Remove the attendance mock.

#### Task 3 — Tests
- Teacher saves today for own section; yesterday → edit_past denied; other section → Permission.
- Principal corrects yesterday → `corrected` log.
- Future date refused for everyone.
- Missing one student → error naming them.
- Student who left is not in the sheet.
- Register for a month with two saved days returns correct present counts.
- React Testing Library: tapping a card cycles P → A → L; "Mark all present" fills every card.
- Matrix cases; remove `attendance.*` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] As sierra: take today's attendance for V-A; yesterday shows the read-only note
- [ ] As sunita: correct yesterday for V-A; activity shows "Corrected attendance"
- [ ] Print the monthly register

---

## P3.5 — Marks and report cards

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.4 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md (marks, exams, grade_scale, "Derived values"), crates/vidya-core/src/marks.rs, docs/API.md ("Marks and report cards"), prototype Marks screen and report card |

### Goal
Marks entry per exam and section with cell-level validation, results and grades computed in Rust, and report card data for one student or a whole section.

### Tasks

#### Task 1 — MarksService (`services/marks.rs`)
| Method | Rules |
|---|---|
| `sheet(actor, exam_id, section_id) -> MarksSheetDto` | `authorize_section(marks.view)`. Exam must belong to current session and be active. Columns: active subjects of the class ordered by sort_order. Rows: active students by roll with values (`number`, `"AB"` or null) and computed total, max, percent (one decimal as text, e.g. `"61.6"`), grade. `last_saved_by`, `last_saved_at` from the newest mark row. `editable` = can(marks.enter) for the section |
| `save(actor, SaveMarksInput) -> MarksSheetDto` | `authorize_section(marks.enter)`. Parse every entry with `parse_mark(max)`. Collect **all** cell errors and return one Validation error `marks.error.cells` with params listing up to 10 cells ("Aman Yadav – Hindi") and a machine-readable `cells` param (JSON array of `{studentId, subjectId}`) so the UI can highlight them. Unknown student or subject for the section → Validation. In one transaction: blank → delete row; value/AB → upsert row with `entered_by`, `entered_at`, `updated_hlc`. Change log one entry for the save (`marks.log.saved`, exam, section, count) plus per-cell payload for sync |
| `report_card(actor, student_id) -> ReportCardDto` | `authorize_section(reportcard.view)` on the student's section. School header, student basics, all active exams of the session as columns with per-subject values, totals, percent and grade per exam, attendance present/total/percent for the session, blank remarks |
| `class_report_cards(actor, section_id) -> Vec<ReportCardDto>` | `authorize_section(reportcard.print)` |
| grade scale | read from `grade_scale` (P3.6 makes it editable) |

#### Task 2 — Commands and frontend
- Commands: `get_marks_sheet`, `save_marks`, `get_report_card`, `get_class_report_cards`.
- `views/Marks.jsx`: inputs hold draft values in component state; totals and grades shown in the table come from the last server response (the UI shows "—" for rows changed since the last save; **no grade maths in JS**). On a validation error, highlight the cells from the `cells` param. Use controlled inputs keyed by `studentId:subjectId` and memoise rows (`React.memo`) so typing in a 45×8 table stays smooth. `views/ReportCard.jsx` renders the `ReportCardSheet` component; printing uses the prototype's layout for now (P5.2 upgrades).
- Remove the marks mock.

#### Task 3 — Tests
- Value above max → error lists the cell; nothing saved from that request.
- AB counts in max with 0 got; blank excluded.
- Changing a value to blank deletes the row.
- Report card percent and grade vectors: 77/125 → "61.6", C.
- Teacher other section → Permission.
- React Testing Library: cells from the `cells` param get the error class.
- Matrix cases; remove `marks.*`, `reportcard.*` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] As sierra: enter Half Yearly marks for V-A, try 150 out of 100 (cell turns red), fix and save
- [ ] Open a report card and print the whole class

---

## P3.6 — Settings

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.5 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md, docs/API.md ("Settings and session"), prototype Settings screen |

### Goal
The principal edits school details, classes and sections, fee plans, subjects, exams, grade scale, app settings and the receipt prefix, with safety checks that protect existing records.

### Tasks

#### Task 1 — SettingsService write methods
All need `settings.edit` (office computer only), one transaction, change log `kind = "settings"`, and return the fresh `SettingsDto`.
| Method | Rules |
|---|---|
| `save_school(SchoolInput)` | name required; UDISE empty or 11 digits; board from list (State Board, CBSE, ICSE, U.P. Board, Other); logo optional PNG or JPEG under 200 KB (check magic bytes), stored as PNG bytes |
| `save_classes(ClassesInput)` | Add classes (name validated, unique, case-insensitive) with 1–6 sections named A, B, C…; reorder classes; change section count. Removing a section: refused if it has active enrollments (`settings.error.section_has_students` with count) or assigned teachers (`settings.error.section_has_teacher` with names); otherwise set `active = 0` (never delete, history keeps references). Removing a class: same checks for all sections |
| `save_fee_plan(FeePlanInput)` | Current session; each amount whole rupees ≥ 0 via `parse_rupees`; terms in {1,2,3,4,12}; transport fee ≥ 0. Changing terms when receipts exist in the session → Conflict `settings.error.terms_locked` unless `confirm = true`, and the confirmation message explains that balances will change |
| `save_subjects(SubjectsInput)` | Per class list; names unique per class; at least one; removed subjects set `active = 0` (marks kept); re-adding an inactive name reactivates it |
| `save_exams(ExamsInput)` | Current session; name unique; max 1–500; lowering max below an existing mark → Validation listing up to 10 students and subjects; removing an exam with marks → `active = 0`; at least one active exam |
| `save_grade_scale(GradeScaleInput)` | `validate_grade_scale` |
| `save_app_settings(AppSettingsInput)` | session timeout 5–120 minutes, receipt paper, print language, tray, start at login, keep awake, school hours (start and end HH:MM). Backup settings are **not** here (P6.1, P6.2) |
| `save_device_code(code)` | `validate_receipt_prefix`; must not equal an approved phone's prefix |

The logo is resized by Rust to at most 256×256 before storing, so printouts and backups stay small.

#### Task 2 — Commands and frontend
- Commands: `save_school`, `save_classes`, `save_fee_plan`, `save_subjects`, `save_exams`, `save_grade_scale`, `save_app_settings`, `save_device_code`.
- `views/Settings.jsx` with one sub-component per section (`SchoolSettings.jsx`, `ClassSettings.jsx`, …), each with its own Save button, its own `useMutation` and inline errors; `confirm()` dialog for the terms change. The logo is chosen with the dialog plugin and sent as a path; Rust reads and checks the file.
- Remove the settings mock.

#### Task 3 — Tests
Every refusal rule above has a test. React Testing Library: the terms-change confirmation resends with `confirm: true`. Matrix cases; remove `settings.view`, `settings.edit` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Add class IX with 2 sections and fees; try removing V-B (refused because of students)
- [ ] Change grade scale and see report card grades change

---

## P3.7 — Home screens, reports, activity and Excel export

| | |
|---|---|
| Phase | 3 School features |
| Depends on | P3.3, P3.4, P3.5, P3.6 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/API.md ("Home", "Reports, activity, alerts", export commands), docs/DEPENDENCIES.md (rust_xlsxwriter), prototype Home screens, Reports, Activity |

### Goal
Real data on all three home screens, the principal's reports and activity log, and genuine `.xlsx` exports saved where the user chooses.

### Tasks

#### Task 1 — Home services
- `home_principal(actor)` (reports.view): date text, sections marked today vs total, students active, present/absent/leave today, fee totals, unmarked sections list with teacher names (first 8 + count), today's collection and receipt count, latest 6 non-auth activity items, backup reminder flag (`last_backup_at` null or older than 7 days), open alerts count.
- `home_accountant(actor)` (fees.view): collected today by mode, receipts today, pending and collected for the session with percentages, top 8 dues, today's receipts (last 10), and `backup_reminder` if the user has `backup.run` and no backup was saved today.
- `home_teacher(actor)` (attendance.view scope): per own section: student count, attendance status today with present count, marks progress per exam (students with at least one mark / total).
Percentages as whole numbers computed in Rust.

#### Task 2 — Reports and activity
- `reports_summary(actor)` (reports.view): enrolment with boys/girls/other, RTE count, Aadhaar collected percent and pending count, attendance percent for the last 30 days and number of marks, class-wise rows (students, boys, girls, RTE, due, collected, collection percent), category counts and percents, fee state counts.
- `list_activity(actor, filter)` (activity.view): newest first, paged by `before_seq`, `limit` ≤ 100; each item: seq, time, who (name or "System"), device, kind, message translated from `summary_key` + params. Filter by kind and user.
- `undo_field_change` will be implemented in P7.4; do not add it yet.

#### Task 3 — Excel exports (`crates/vidya-export`)
Use `rust_xlsxwriter` with default features off (verify API and features with dependency-verifier; record size in `docs/SIZE.md`). Shared helpers: bold header row with background, frozen top row, auto-filter, column widths, money columns as numbers with Indian format string `[$₹]#,##,##0` (verify Excel accepts it; if not, use `#,##0` and note), dates as real dates.
| Command | Sheet columns |
|---|---|
| `export_students_xlsx` (students.export) | Admission no, Name, Class, Section, Roll, Gender, Date of birth, Father, Mother, Mobile, Category, RTE, Bus, Locality, Aadhaar collected, APAAR, Status, Fee due, Paid, Balance |
| `export_dues_xlsx` (fees.export) | Admission no, Name, Class-section, Roll, Father, Mobile, Due, Paid, Balance, Previous session dues |
| `export_daybook_xlsx` (fees.export) | Receipt, Time, Student, Class, Mode, Reference, Received by, Amount, Cancelled; totals rows |
| `export_class_summary_xlsx` (reports.export) | Class rows from reports_summary |
The frontend opens a Save dialog (`@tauri-apps/plugin-dialog`, suggested name `students-vaani-2026-09-15.xlsx`) and passes the chosen path. Rust writes to `<path>.partial` then renames. Hindi names must be preserved (UTF-8).

#### Task 4 — Commands and frontend
- Commands: `home_principal`, `home_accountant`, `home_teacher`, `reports_summary`, `list_activity`, `export_students_xlsx`, `export_dues_xlsx`, `export_daybook_xlsx`, `export_class_summary_xlsx`.
- Replace remaining mocks in `Home.jsx`, `Reports.jsx` and `Activity.jsx`. Activity uses a "Load more" button with `before_seq`.
- Home screens re-query when the window regains focus.

#### Task 5 — Tests
- Home DTO snapshot tests (`insta`) for each role with the sample school and a fixed clock.
- Export tests: write to a temp file, reopen with `calamine` (dev-dependency here), check header row and one data row including a Devanagari name.
- React Testing Library: each home variant renders its DTO.
- Matrix cases; remove `reports.*`, `activity.view`, `students.export`, `fees.export` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] Each role's home screen shows real numbers matching the fee register and attendance
- [ ] Open each exported `.xlsx` in Numbers or Excel; add a student with a Hindi name first and confirm it appears correctly

---

## P4.1 — License codes crate

| | |
|---|---|
| Phase | 4 Licensing and setup |
| Depends on | P2.1 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, crates/AGENTS.md, docs/LICENSE_FORMAT.md (entire), docs/DECISIONS.md (D13, D29) |

### Goal
`crates/vidya-license` encodes, decodes and verifies activation and reset codes exactly as LICENSE_FORMAT.md. Signing exists only behind feature `signing`, which the school app never enables.

### Dependencies allowed in this prompt
`ed25519-dalek` (default features off; only what verification needs, plus signing features behind `signing`), `sha2`, `base32`, `thiserror`, `zeroize`, `rand` (only with feature `signing`), `chrono` (date conversion only). Use the dependency-verifier subagent to confirm the base32 crate's Crockford option and its exact alphabet handling; if it does not match LICENSE_FORMAT.md section 2 character mapping, implement Crockford base32 yourself in `crockford.rs` with tests and do not use the crate.

### Tasks

#### Task 1 — Module layout
```
crates/vidya-license/src/
  lib.rs          public API only
  crockford.rs    encode/decode + input normalisation (O→0, I/L→1, reject U)
  device.rs       device number from platform values, display form, parse
  payload.rs      ActivationPayload, ResetPayload, binary encode/decode with range checks
  code.rs         text form with check characters, parse with error order
  verify.rs       verify(public_key, text) -> Result<Verified, LicenseError>
  sign.rs         #[cfg(feature = "signing")] sign functions and key generation
  keys.rs         PRODUCTION_PUBLIC_KEY: [u8; 32] (placeholder all zeros until P4.3 exports the real key; verification with an all-zero key must always fail with LicenseError::NoPublicKey)
```

#### Task 2 — Public API
```rust
pub fn device_number(platform: &str, value1: &str, value2: &str) -> u64;       // 60-bit
pub fn device_display(n: u64) -> String;                                          // "VD-XXXX-XXXX-XXXX"
pub fn parse_device_display(s: &str) -> Result<u64, LicenseError>;
pub struct ActivationPayload { pub school_code: String, pub device: u64, pub issued: NaiveDate, pub max_users: u16, pub max_devices: u8, pub license_type: u8, pub flags: u16 }
pub struct ResetPayload { pub school_code: String, pub device: u64, pub issued: NaiveDate, pub expires: NaiveDate, pub nonce: [u8; 8] }
pub enum Verified { Activation(ActivationPayload), Reset(ResetPayload) }
pub fn verify_code(public_key: &[u8; 32], text: &str) -> Result<(Verified, Vec<u8> /*payload*/, [u8; 64] /*sig*/), LicenseError>;
#[cfg(feature = "signing")] pub fn sign_activation(secret: &SigningKey, p: &ActivationPayload) -> String;
#[cfg(feature = "signing")] pub fn sign_reset(secret: &SigningKey, p: &ResetPayload) -> String;
pub enum LicenseError { Typing, Invalid, WrongType, OtherDevice, Expired, Used, NoPublicKey, Format(String) }
impl LicenseError { pub fn message_key(&self) -> &'static str }   // keys from LICENSE_FORMAT.md section 4
```
Device and expiry checks are done by callers with helper `check_activation(v, this_device)` and `check_reset(v, this_device, today, used_nonce: bool)`.

#### Task 3 — Test vectors
1. Generate a test key pair **once** in a test helper from a fixed 32-byte seed (for example bytes 1..=32) named `TEST_ONLY_SEED`. It lives only in `tests/`.
2. `tests/vectors.rs`: build activation for `vaani`, device from `device_number("macos", "ABC", "XYZ")`, issued 2026-09-15, 40 users, 5 devices; sign; verify ok and fields equal. Commit the resulting code text as a constant and assert re-signing produces the same text (Ed25519 is deterministic).
3. Tamper tests: change each character position one at a time (loop) → `Typing` or `Invalid`, never Ok.
4. Bad check characters only → `Typing`.
5. Other device → `OtherDevice`; activation code on reset screen → `WrongType`.
6. Reset code: expired date → `Expired`; used nonce → `Used`.
7. Device display round trip, lowercase input, `O`/`I`/`L` mapping, `U` rejected.
8. `keys.rs` production key is not equal to the test public key (assert) and all-zero key returns `NoPublicKey`.
9. Property test: random payloads encode/decode round trip.

#### Task 4 — Feature isolation
- `src-tauri` depends on `vidya-license` **without** `signing`. Add a CI step (both OSes) that inspects `cargo tree -p vidya-app -e features` output and fails if `vidya-license` has the `signing` feature enabled (check the exact output format first and write the matching command).

### Done when (agent)
- [ ] `cargo test -p vidya-license --all-features` and `cargo test -p vidya-license` pass
- [ ] The CI feature check is in place
- [ ] Security-reviewer subagent run on the crate; findings fixed
- [ ] `npm run verify` passes

---

## P4.2 — Device ID, activation, server permit and principal reset

| | |
|---|---|
| Phase | 4 Licensing and setup |
| Depends on | P4.1, P2.7 |
| Size | medium |
| Runs on | MacBook; Windows device values via CI and VM |
| Read first | AGENTS.md, docs/LICENSE_FORMAT.md, docs/PLATFORMS.md, docs/DECISIONS.md (D29), docs/API.md ("App and setup", `principal_reset_with_code`) |

### Goal
The office computer shows its Device ID, accepts a signed activation code before setup, refuses to run on a different computer, re-verifies the license at every start, issues the `ServerPermit` that is the only way to start server features, and lets the principal reset a forgotten password with a one-time code.

### Tasks

#### Task 1 — Device values
- macOS (`platform/macos.rs::device_values`): read `IOPlatformUUID` and `IOPlatformSerialNumber` from the `IOPlatformExpertDevice` IOKit registry entry through IOKit bindings (no shell commands). Verify each function with dependency-verifier; if bindings are missing, declare the minimal `extern "C"` functions with `#[link(name = "IOKit", kind = "framework")]` and CoreFoundation string conversion, and document it.
- Windows (`platform/windows.rs::device_values`): `MachineGuid` from the registry and `UUID` from `Win32_ComputerSystemProduct`. Prefer a way that does not pull a large WMI dependency (for example reading the SMBIOS system UUID through `GetSystemFirmwareTable`); compare size impact and choose the smaller one that works. If the second value is unavailable, use only MachineGuid with value2 `"-"` and log a warning (no personal data).
- Unit test on each OS that the values are non-empty and stable across two calls (Windows test runs in CI).
- PROGRESS.md "Check in Windows VM": Device ID shown in the VM stays the same after restart; note that a VM's Device ID differs from the real PC's.

#### Task 2 — LicenseService (`services/license.rs`)
| Method | Rules |
|---|---|
| `device_id(platform)` | `device_display(device_number(platform.name(), v1, v2))` |
| `activate(code_text)` | Only when `school` has no row and `license` has no row (else Conflict `license.error.already_activated`). `verify_code(PRODUCTION_PUBLIC_KEY)` → must be Activation for this device. Store in `license` (code, payload, signature base64, device display, limits, issued, activated_at). Change log event `license.log.activated` |
| `check_at_start(platform) -> LicenseState` | No license → `NotActivated`. Stored signature re-verified with the compiled public key, payload fields equal the columns, device number equals this computer → `Valid`; device mismatch → `OtherComputer`; signature or field mismatch → `Tampered` |
| `server_permit(platform, state) -> Option<ServerPermit>` | Returns a permit only when the state is `Valid` and a school exists. See Task 3 |
| `principal_reset(code_text, new_password)` | Reset code for this device and school code, not expired (today), nonce not in `used_reset_nonces`; validate password against the principal's username; hash; `must_change = 0`, `locked = 0`, `locked_until = NULL`, `failed_count = 0`; insert nonce; end principal sessions; change log event |
| `get(actor)` | license.view → `LicenseDto` (school code, device ID, limits, issued, activated; never the raw payload) |

For debug builds only: an environment variable `VIDYA_DEV_PUBLIC_KEY_HEX` may replace the public key so the developer can test with the test key before P4.3. Compile this out of release builds (`#[cfg(debug_assertions)]`) and log a warning when used.

#### Task 3 — ServerPermit (D29)
```rust
// in vidya-services::license
pub struct ServerPermit { school_code: String, device_display: String, max_devices: u8, _private: () }
impl ServerPermit { pub fn school_code(&self) -> &str; pub fn device_display(&self) -> &str; pub fn max_devices(&self) -> u8; }
```
- The fields are private and there is no public constructor, no `Default`, no `Clone`, no `Deserialize`. The only constructor is `LicenseService::server_permit`.
- A compile-fail doc test (`compile_fail`) shows `ServerPermit { .. }` cannot be built outside the module.
- `AppState` stores `Option<Arc<ServerPermit>>` after the start check. `app_status.server_allowed` = permit present.
- In debug builds with the sample school's demo license, `server_permit` returns a permit only when `VIDYA_DEV_ALLOW_DEMO_SERVER=1` is set, so the developer can test sync before activating a real code. This path is compiled out of release builds.
- P7.1's `vidya_server::start` will take `&ServerPermit` as a required argument. Write that in a comment on the struct.

#### Task 4 — App start and lock screens
- `app_status` gains `license_state`. Start flow in `DesktopApp.jsx`:
  - `NotActivated` and no school → `views/setup/Activation.jsx` (replaces the DEMO- step): shows the Device ID with a Copy button, a textarea for the code, an "Activate" button, errors from `AppError.message`, and a "Restore from a backup file" link (hidden until P6.1).
  - `OtherComputer` → `views/Locked.jsx` variant: "This copy of Vidya was activated on a different computer. Contact your Vidya provider." Show the Device ID with Copy, "Enter a new activation code" (replacement code, allowed in this state: verify it is an Activation code for this device and the **same school code**, then update the license row), and "Save a backup file" (enabled after P6.1). The server never starts in this state.
  - `Tampered` → `Locked.jsx` variant: "Vidya's license information is damaged. Contact your Vidya provider." with the Device ID. The server never starts.
  - Sample school (debug) keeps its demo license row and skips this check only in debug builds.
- Sign-in screen: link "Principal forgot password?" → modal with Device ID, reset code, new password twice.

#### Task 5 — Commands
`get_device_id`, `activate`, `get_license`, `principal_reset_with_code`, plus `replace_activation` (add to API.md: **D**, no session, only in `OtherComputer` state).

#### Task 6 — Tests (services, with test key injected through a constructor parameter, not the env variable)
- Valid activation stored; second activation refused.
- Code for another device → `OtherDevice` message.
- Editing `license.max_users` in the DB → `Tampered` at start, and `server_permit` returns `None`.
- Changing the platform device values → `OtherComputer` and no permit; replacement code for same school works and then a permit is issued; different school refused.
- Valid license without a school → no permit.
- Reset code works once; second use refused; expired refused.
- Matrix case for `get_license`; remove `license.view` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Security-reviewer subagent run on license service, `ServerPermit` and screens

### Done when (developer)
- [ ] Fresh data folder (rename the old one): the Activation screen shows a Device ID; the value is the same after restarting
- [ ] Windows VM check from PROGRESS.md done

---

## P4.3 — Provider Tool

| | |
|---|---|
| Phase | 4 Licensing and setup |
| Depends on | P4.2 |
| Size | large |
| Runs on | MacBook only (never CI) |
| Read first | AGENTS.md, provider/AGENTS.md, docs/LICENSE_FORMAT.md, docs/DECISIONS.md (D13, D22), docs/UI_GUIDE.md |

### Before you start (developer)
- Switch on FileVault on the MacBook (System Settings → Privacy & Security → FileVault).
- Have a pen drive ready.
- Think of a long passphrase (5+ random words). Write it on paper stored safely. Never type it into a chat.

### Goal
A separate macOS Tauri app for the seller: creates and protects the signing key, keeps an encrypted register of schools and issued codes, issues activation, replacement and principal reset codes, and exports the public key for the school app. It is never shipped to schools, so the 30 MB budget does not apply to it.

### Tasks

#### Task 1 — Project
- `provider/` as its own Tauri 2 app: identifier `in.vidya.provider`, product name "Vidya Provider Tool", no separate `package.json`; add root npm scripts (`npm run provider:dev`, `npm run provider:build` running Tauri with `--config provider/src-tauri/tauri.conf.json` or from the provider folder — check the CLI for the correct way).
- Add `provider/src-tauri` to the Cargo workspace members.
- Frontend `provider/src/` is React, with its own `main-provider.jsx` and Vite config, and imports `src/styles/`, `src/core/` and `src/components/` from the main app (no copies).
- macOS only. The CI workflows must exclude it from builds (`cargo test --workspace --exclude vidya-provider` on Windows or gate with `cfg(target_os = "macos")`). The school app's Vite build must not include any file from `provider/` (add a check to `scripts/check-bundle-split.mjs`).

#### Task 2 — Key vault (`provider/src-tauri/src/vault.rs`)
- First run: ask for a passphrase twice (at least 16 characters, show a strength hint), generate an Ed25519 key with the OS random generator, derive a 32-byte key with Argon2id (memory 256 MiB, iterations 3, random salt), encrypt the 32-byte secret with XChaCha20-Poly1305, and write `vault.json` `{ version, salt, nonce, ciphertext, public_key_hex, created_at }` to `~/Library/Application Support/in.vidya.provider/`.
- Every start: ask for the passphrase, decrypt into `Zeroizing` memory, keep only while the app runs; lock after 15 minutes idle (ask again).
- Backup: "Save key backup to pen drive" writes `vidya-provider-key-backup-YYYY-MM-DD.json` (same encrypted content) to a chosen folder, reads it back, verifies it decrypts with the passphrase. Issuing codes is disabled until at least one verified backup is recorded.
- Restore: "Restore key from backup" on first run.
- Screen text: "If you lose this Mac and the pen drive backup, you can never issue codes for existing Vidya versions again. Keep the pen drive somewhere safe, away from this Mac."

#### Task 3 — Register database
- SQLite through `vidya-db`'s open function with its own key derived from the unlocked signing secret (HKDF-style: SHA-256("vidya-provider-register-v1" || secret)); file `register.db`.
- Tables (migration in `provider/src-tauri/migrations/0001.sql`): `schools(id, school_code UNIQUE, name, contact_person, phone, city, price_paid, payment_reference, paid_on, notes, created_at)`, `codes(id, school_id, kind CHECK IN ('activation','replacement','reset'), device_id, max_users, max_devices, issued_on, expires_on, reason, code_text, created_at)`. `codes` append-only with triggers.
- Register export to Excel using `vidya-export` helpers.
- Register backup: included in the pen drive backup as an encrypted copy.

#### Task 4 — Screens (React)
1. Unlock / first-run.
2. Schools list with search; Add school form (school code 3–12 `[a-z0-9]`, unique; payment fields required: the tool refuses to issue an activation code for a school with no payment reference).
3. School detail: codes history; buttons Issue activation code, Issue replacement code, Issue principal reset code.
4. Issue activation: paste Device ID (validated with `parse_device_display`), max users (default 40), max devices (default 5). Shows the code in a large monospace box with Copy and "Copy message" (a short WhatsApp-ready text: "Vidya activation code for <school>: <code>").
5. Replacement: old Device ID (prefilled from last code), new Device ID, reason (required). Warn if 2+ replacements in the last 12 months. The message to the school explains: "Only the computer with the new Device ID will run as your school's office computer. Switch Vidya off on the old computer."
6. Reset code: Device ID (prefilled), valid 3 days, random nonce.
7. Settings: export public key, key backup, change passphrase (re-encrypt vault and prompt for a new pen drive backup).

#### Task 5 — Public key export
"Export public key for Vidya app" writes `crates/vidya-license/src/keys.rs` content to a chosen file: `pub const PRODUCTION_PUBLIC_KEY: [u8; 32] = [..];` with a comment containing the date and a SHA-256 fingerprint. The developer copies it into the repo and commits. **The private key is never exported in plain form.**

#### Task 6 — Tests
- Vault round trip; wrong passphrase fails; tampered file fails.
- Codes issued by the tool verify with `vidya-license` using the tool's public key.
- Activation refused without payment reference and without a verified backup.

### Do not
- Never write the secret key, passphrase or derived keys to disk unencrypted, logs, clipboard or error messages.
- Never add the provider app to release workflows.

### Done when (agent)
- [ ] `npm run verify` passes (provider tests included on macOS)
- [ ] Security-reviewer subagent run on `provider/`; findings fixed
- [ ] `git grep -n "BEGIN PRIVATE\|secret_key\|TEST_ONLY_SEED" -- ':!crates/vidya-license/tests'` shows nothing unexpected

### Done when (developer)
- [ ] Create the vault, save the key backup to the pen drive, confirm "verified"
- [ ] Export the public key into `crates/vidya-license/src/keys.rs`, commit
- [ ] Add a test school, issue an activation code for the Device ID shown by your Vidya app, and activate it
- [ ] Issue a reset code and reset the principal password once; second use refused

---

## P4.4 — Setup wizard, login slips and recovery sheet

| | |
|---|---|
| Phase | 4 Licensing and setup |
| Depends on | P4.2, P3.7 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/API.md ("App and setup"), docs/DATA_MODEL.md ("Default data created by setup"), docs/DECISIONS.md (D30, D31), prototype setup wizard, credentials and printouts |

### Goal
The real first-run experience after activation, running only while no school exists, resumable, creating everything in one transaction, then printing login slips and a recovery sheet. The last JS mock is removed.

### Tasks

#### Task 1 — SetupService (`services/setup.rs`)
Guard for every method: license row exists and `school` has no row; otherwise Conflict `setup.error.not_available`. No actor (no session exists yet).
| Method | Rules |
|---|---|
| `get_state()` | Parse `meta.wizard_state_json` (versioned struct) or return step 1 with defaults (session from `default_session_name(today)`) |
| `save_step(step, data)` | Validate that step only, merge into state, store. Steps: `school` (name, address, UDISE, board, phone, session, terms), `principal` (name, mobile, password + confirm validated with `validate_new_password` using the principal username), `classes` (list of classes with section counts and fee plan per class, transport fee), `staff` (teachers and accountants: name, mobile, sections for teachers, optional username override), `backup_password` (≥ 8 characters, confirm, not equal to principal password; plus a checkbox "Let staff at this computer run backups", default on, D31), `review`. The principal password and backup password are **not** stored in plain text in wizard state: store an Argon2id hash for the principal password and keep the backup password only as a derived verifier; the principal enters both passwords again at "Create school" if the app was restarted (explain on screen) |
| `create_school(principal_password, backup_password)` | Re-validate everything. One transaction: `seed_defaults`, school, session, classes, sections, fee plans, subjects defaults per class, exams defaults, principal user (`username_base_from_name` or `principal`, must_change 0), staff with temporary passwords (must_change 1), user_sections, `meta.device_code = "PC"`, counters, backup verifier, `backup_staff_can_run`, delete wizard state, change log `setup.log.created`. After commit, derive the automatic backup key from the backup password and store it with `platform.store_secret("backup-key")` so automatic backups can start as soon as P6.1 exists (if storing fails, show the principal a warning; the school is still created). Returns `principal_username` and `Vec<CredentialSlipDto>` (only time temporary passwords exist in plain form) |

#### Task 2 — Commands and screens
- Commands: `wizard_get_state`, `wizard_save_step`, `wizard_create_school`.
- `views/setup/Wizard.jsx` with one component per step and the step bar, as in the prototype, now after Activation. The school code shown is read-only from the license: "Usernames will look like sierra@vaani".
- Staff step: usernames are suggested by Rust in the `wizard_save_step` response (`suggested_usernames`). The principal can edit them before continuing; edited names are validated again in Rust.
- After creation: `Credentials.jsx` with the slips table, "Print login slips" (A4, 8 per page, English and Hindi instructions) and "Print recovery sheet" (school name and code, principal username, Device ID, activation code, setup date, blank line "Backup password (write by hand): ____", provider contact from build-time env `VIDYA_PROVIDER_CONTACT` with a default placeholder listed in KNOWN_ISSUES), both printed as components through `usePrint()`. Warning that temporary passwords will not be shown again.
- Principal home checklist until done (stored in `meta.checklist_json`): print login slips, print recovery sheet, save first backup (enabled after P6.1), add a backup drive (enabled after P6.2), add students.

#### Task 3 — Remove the mock
Delete `src/api/mock/` entirely. `commands.js` must have no mock import. Update KNOWN_ISSUES (remove the mock item). `npm run verify` must pass with the mock gone. Record the new `dist/` size in `docs/SIZE.md`.

#### Task 4 — Tests
- Wizard unavailable after a school exists (every method).
- Closing mid-way: state persists and resumes at the same step.
- `create_school` failure in the middle (inject a failing repository via a test hook) leaves no school row.
- Staff with Devanagari names get `teacher1`, `teacher2`; duplicate first names get numbers.
- Change log and wizard state never contain plain passwords.
- The backup key secret is stored after creation (fake platform).

### Done when (agent)
- [ ] `grep -rn "mock" src/api` returns nothing
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Rename the data folder, activate with a Provider Tool code, complete the wizard, print slips and recovery sheet
- [ ] Quit halfway through the wizard; reopening resumes at the same step
- [ ] Sign in as a new teacher with the slip password; forced password change works

---

## P4.5 — Student import from Excel

| | |
|---|---|
| Phase | 4 Licensing and setup |
| Depends on | P4.4 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/API.md (`import_students_preview`, `import_students_commit`), docs/DEPENDENCIES.md (calamine), crates/vidya-services/src/services/students.rs |

### Goal
Schools can bring existing student lists from Excel: download a template, fill it, preview row-by-row errors, import all rows in one transaction.

### Tasks

#### Task 1 — Template
`export_import_template_xlsx` (**D**, students.import; add to API.md): sheet "Students" with columns Name*, Gender*, Class*, Section*, Date of birth (DD-MM-YYYY), Father*, Mother, Mobile*, Category*, RTE (Yes/No), Bus (Yes/No), Locality, Aadhaar collected (Yes/No), APAAR (Yes/No), Admission number (optional, for existing numbers); a second sheet "Help" explaining each column in English and Hindi, with allowed values for Gender, Category and the school's classes and sections; data validation lists on Gender, Category, Class and Yes/No columns (verify rust_xlsxwriter supports data validation; if not, only the Help sheet).

#### Task 2 — Preview (`services/import.rs`)
`preview(actor, path) -> ImportPreviewDto`:
- Read the first sheet with `calamine` (only the `.xlsx` reader features; record size impact in `docs/SIZE.md`); find columns by header name (case-insensitive, trimmed, ignoring `*`), so column order may differ.
- Accept dates as Excel dates or text `DD-MM-YYYY` / `DD/MM/YYYY`; Yes/No also `Y`, `N`, `हाँ`, `नहीं`; mobile numbers stored as numbers.
- Validate each row with the same core validators as `StudentService::add` (without writing). Errors per row: row number, column, message.
- Admission numbers: if given, format `ADM/<digits>` or digits only; must be unique within the file and not already used. If any row gives one, all rows must.
- Duplicates within the file (same name + father) and against existing active students → warnings (not errors).
- Maximum 3,000 rows.
- Store the parsed rows in memory with a preview id (30 minutes).

#### Task 3 — Commit
`commit(actor, preview_id)`: refuse if the preview had errors. One transaction: for each row call the same internal insert used by `StudentService::add` (refactor so both share one function); when admission numbers are given, set counter `adm_no` to the highest imported number if larger. One change-log entry per student plus a summary entry. Returns count.

#### Task 4 — Screen
`views/students/ImportStudents.jsx`, reached from Students → "Import from Excel": steps Download template → Choose file → Preview table (errors in red with row numbers, warnings in orange, counts; render at most 200 rows at a time with paging so a 3,000-row preview stays fast) → Import button disabled while errors exist → Done with count.

#### Task 5 — Tests
- File with 200 valid rows and 2 invalid rows → 2 errors; commit refused; nothing written.
- Columns in a different order work.
- Existing admission numbers continue the counter correctly.
- Hindi names preserved.
- Matrix case (office computer only for phones); remove `students.import` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Fill the template with 20 real-looking rows in Excel or Numbers (export as .xlsx), import, and check the students list

---

## P5.1 — Printing and Hindi spike (decide how documents are made)

| | |
|---|---|
| Phase | 5 Documents |
| Depends on | P3.7 |
| Size | medium (research and proof, little product code) |
| Runs on | MacBook; Windows via CI artifact and VM; Android later |
| Read first | AGENTS.md, docs/DECISIONS.md (D20, D28), docs/UI_GUIDE.md ("Print", "React rendering rules"), docs/PLATFORMS.md, src/core/print.jsx |

### Why this prompt exists
Hindi (Devanagari) needs complex text shaping: letters join into conjuncts like क्ष, त्र, श्र and vowel signs move (कि). Many PDF libraries place characters one by one and print broken Hindi. Choosing a method without proof is how projects fail late. This prompt tests the options, including their size cost, and records a decision.

### Goal
A written decision in `docs/DECISIONS.md` (as D25, pending the developer's approval) with evidence, and a tiny working proof for the chosen method.

### Test document
`src/print/SpikeDocument.jsx`: an A4 page component with the school header, a table, the text `क्षितिज श्रीवास्तव, कक्षा पाँच, विद्यालय, प्रिंसिपल, ट्रांसपोर्ट शुल्क ₹1,00,000`, English text, a ₹ sign, and bold and regular weights, printed through `usePrint()` from a debug-only "Print test page" button. Bundle **Noto Sans** and **Noto Sans Devanagari** as WOFF2 files in `src/assets/fonts/` (download from Google Fonts' official repository or the Noto project on GitHub; record the source URL and licence (SIL Open Font License) in DEPENDENCIES.md). Use only the Regular and Bold weights, and record the font sizes in `docs/SIZE.md`; subsetting comes in P10.1 if needed.

### Options to test
| Option | Desktop | Android |
|---|---|---|
| A. WebView print dialog | `window.print()` on the print root in the app window; check it works in Tauri on macOS (WKWebView) and Windows (WebView2) | Android WebView does not support `window.print()`; check |
| B. WebView to PDF file | macOS: WKWebView PDF creation API; Windows: WebView2 print-to-PDF API; reached through Tauri's `with_webview` platform handle (verify availability in the installed Tauri version) | Android: `WebView.createPrintDocumentAdapter` with `PrintManager` (opens system print UI with "Save as PDF") through a Kotlin plugin |
| C. Rust PDF library with shaping | A crate that embeds fonts **and** shapes Devanagari (check whether candidate crates perform OpenType shaping; list evidence from their docs or source) | same |

### Tasks
1. For each option, build the smallest proof that prints or saves the spike document on the MacBook. For Windows options, add a temporary debug command and ask the developer to test the CI build in the VM.
2. Compare the output **visually** by rendering the PDF to PNG (on the MacBook use `sips` or `qlmanage` to convert, and place the images in `docs/spike/`). Show the developer the images for क्ष, श्र, कि and the ₹ sign.
3. Measure the size added to each installer and to the APK for each option. Option C must also count the font files it embeds and any shaping library.
4. Write `docs/spike/PRINTING.md`: options, evidence (image paths), size, platform support table, risks, recommendation.
5. Recommended default unless evidence says otherwise: **B for saving PDFs and sharing on Android, A for direct printing on desktop**, both from the same React document components, because the system web engine shapes Hindi correctly and adds almost nothing to the app size. Option C only if proven with images and it fits the budget.
6. Add D25 to the decision log table as "proposed" and stop for the developer's approval.

### Do not
- Do not build the real documents yet (P5.2).
- Do not adopt a PDF crate without images proving correct Hindi.

### Done when (agent)
- [ ] `docs/spike/PRINTING.md` with images and size numbers for each tested option
- [ ] D25 proposed; waiting for approval

### Done when (developer)
- [ ] Look at the images: Hindi conjuncts are correct for the recommended option
- [ ] Print the spike page from the MacBook to a real printer or PDF, and from the Windows VM
- [ ] Approve D25 (tell the agent to mark it approved)

---

## P5.2 — Documents: receipts, report cards, registers, TC

| | |
|---|---|
| Phase | 5 Documents |
| Depends on | P5.1 (D25 approved), P4.4 |
| Size | large |
| Runs on | MacBook; Windows via VM |
| Read first | AGENTS.md, docs/spike/PRINTING.md, docs/DECISIONS.md (D25), docs/UI_GUIDE.md, docs/PERMISSIONS.md (students.issue_tc), prototype receipt and report card |

### Before you start (developer)
Put photos of real receipts, report cards, attendance registers and a transfer certificate from local schools in `reference/documents/`. Blur student details.

### Goal
Every printed document as a React component, printed and saved as PDF using the method in D25, with Hindi printing correctly, matching the layouts schools already use.

### Tasks

#### Task 1 — Document system (`src/print/`)
- One component per document: `<ReceiptDoc dto options />`, `<ReportCardDoc … />` and so on. Shared `DocHeader.jsx` (logo, name, address, UDISE, phone) and `DocFooter.jsx` ("Printed on <date> by <name>", small).
- `print.css` page rules: `@page` sizes A4 portrait, A4 landscape, A5, and 80 mm roll (width 80 mm, auto height); margins; `break-inside: avoid` for rows and copies. Each document sets its page size with a class on its root element and a matching named `@page` rule (verify named pages work in both WebView2 and WKWebView; if not, set the page size just before printing and restore it after).
- Language option en, hi or both (labels from locale files through `translate(lang, key)`; values unchanged).
- `usePrint()` gains `printDocument(docType, dto, options)` and `savePdf(docType, dto, options, suggestedName)`, implementing D25 for desktop. The Android path is added in P8.5.
- The logo is a `data:` URL from the DTO (allowed by the CSP).

#### Task 2 — Documents
| Document | Data from | Details |
|---|---|---|
| Fee receipt | `get_receipt` | Paper from settings: A4 with school and parent copies and a dashed cut line; A5 single; 80 mm thermal (compact, no logo, 32–42 characters per line equivalent). Cancelled receipts print "CANCELLED" diagonally with reason |
| Report card | `get_report_card`, `get_class_report_cards` | A4, logo, student details, exams as columns, totals, percent, grade, attendance, remarks lines, class teacher and principal signature lines; whole section as one print job, one card per page |
| Monthly attendance register | `attendance_register` | A4 landscape, days as columns (P/A/L), totals, section and month in header |
| Day book | `day_book` | A4, totals by mode, cancelled separately |
| Fee dues list | `fee_register` filter dues + section | A4, parent mobile, balance, a blank "Called on" column |
| Login slips, recovery sheet | setup and users | Move from P4.4 into this system |
| Transfer certificate | new `issue_tc` | See Task 3 |

Adjust layouts to match the photos in `reference/documents/` where they exist; keep a note of what was matched in `docs/spike/DOCUMENTS.md`.

#### Task 3 — Transfer certificates (`services/tc.rs`)
`issue(actor, TcInput)` (students.issue_tc): student must be marked left (or the dialog marks them left in the same transaction with date and reason); fields commonly required by state boards: TC number (`next_counter("tc_no")` formatted `TC/0001`), admission number and date, name, father, mother, nationality (default Indian), category, date of birth in figures and words (English words from a new core function `date_in_words_en`, with tests), class last studied, whether failed, subjects studied, fees paid up to (month), date of leaving, reason, conduct, remarks, date of issue. Insert `transfer_certificates` (append-only, `data_json` holds the printed values), change log. Re-printing reads the stored data (never regenerates changed values). Command `issue_tc` and `get_tc` (add `get_tc` to API.md).

#### Task 4 — Settings
Receipt paper, print language and logo upload (already in P3.6) connected to printing. "Test print" button printing the spike document.

#### Task 5 — Tests
- React Testing Library: each document renders with a sample DTO and shows names as literal text; snapshot tests (Vitest `toMatchSnapshot` on the rendered container) for receipt A4, A5 and thermal.
- Rust: TC numbering, append-only, `date_in_words_en("2015-08-07")` → "Seventh August Two Thousand Fifteen" (write 5 vectors).
- Matrix case; remove `students.issue_tc`, `attendance.print_register`, `reportcard.print` from NOT_YET_IMPLEMENTED if still present.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] PDFs for each document saved to `docs/spike/samples/` from the sample school, converted to PNG for review
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] Print each document on an A4 laser printer from the MacBook and from the Windows VM
- [ ] A receipt on an 80 mm thermal printer (if available)
- [ ] Hindi names print correctly
- [ ] Compare layouts with the school photos

---

## P6.1 — Encrypted backups, automatic backups and restore

| | |
|---|---|
| Phase | 6 Backups and year end |
| Depends on | P4.4 |
| Size | large |
| Runs on | MacBook; Windows via CI and VM |
| Read first | AGENTS.md, docs/BACKUP_FORMAT.md (entire), docs/PLATFORMS.md, docs/PERMISSIONS.md (`backup.run`, `backup.manage`, office computer only), docs/DECISIONS.md (D30, D31), docs/API.md ("Backup"), prototype backup screen and `.vidyabak` v1 format |

### Goal
Encrypted backup files in the v2 format on the office computer: automatic daily and monthly copies with retention, "Back up now" for the principal or the staff member at the office computer, saving a backup file anywhere, restore with a safety copy, backup test, and import of the prototype's v1 files. Copying to backup drives comes in P6.2.

### Dependencies allowed in this prompt
`zstd` (default features off), `chacha20poly1305`, `argon2`, `sha2`, `zeroize`; for v1 import only: check whether an approved crate provides PBKDF2-SHA256 and AES-256-GCM. If none is listed, stop and ask for approval of the smallest well-maintained options (for example the RustCrypto `pbkdf2` and `aes-gcm` crates), explaining that they are only needed for importing prototype files. Record size impact in `docs/SIZE.md`; if `zstd` adds more than 1 MB, compare with a smaller compressor already in the dependency tree and report.

### Tasks

#### Task 1 — Format library (`crates/vidya-backup/src/format.rs`)
```rust
pub struct BackupHeader { .. }                          // exactly the JSON in BACKUP_FORMAT.md
pub fn write_backup(out: &Path, header: &BackupHeader, plain_sqlite: &Path, key: &Zeroizing<[u8;32]>) -> Result<BackupWritten { size, sha256 }, BackupError>;
pub fn read_header(path: &Path) -> Result<BackupHeader, BackupError>;
pub fn decrypt_to(path: &Path, key_or_password: KeySource, out_plain_sqlite: &Path) -> Result<BackupHeader, BackupError>;
pub fn derive_key(password: &str, salt: &[u8], params: &KdfParams) -> Zeroizing<[u8;32]>;
pub fn sha256_file(path: &Path) -> Result<[u8; 32], BackupError>;
```
Errors: `WrongPasswordOrDamaged` (AEAD failure — one message for both, "The backup password is wrong or the file is damaged."), `NotABackup`, `NewerVersion`, `Io`.
Streaming is not required; files are small. Files are always written as `.partial`, flushed (`sync_all`) and renamed.

#### Task 2 — Making a consistent plain copy
- Find out (with evidence from SQLCipher documentation) the supported way to produce a **plaintext** copy of an encrypted database: `ATTACH DATABASE '<tmp>' AS plaintext KEY ''` + `SELECT sqlcipher_export('plaintext')` is the documented SQLCipher approach; confirm it works with the bundled version and write a test.
- Temporary file in a private temp folder inside the data directory (`data/tmp`, created `0700`), deleted after use in all code paths (use a guard type with `Drop`). On start, delete anything left in `data/tmp`.
- `PRAGMA integrity_check` on the copy must return `ok`.

#### Task 3 — BackupService (`services/backup.rs`, desktop only)
All methods are office computer only (P2.2 refuses phones).
| Method | Permission | Rules |
|---|---|---|
| `status(actor)` | backup.run | Last backup time and kind, auto enabled, key stored, staff allowed, next scheduled, days since the last copy that left this disk (from P6.2; null for now), list of local files (name, kind, size, created) |
| `run_now(actor)` | backup.run | Needs the stored backup key (else Conflict `backup.error.auto_off`: "Automatic backups are switched off. Ask the principal to switch them on."). Non-principal and `backup_staff_can_run = 0` → Permission `backup.error.staff_not_allowed`. Makes a `manual` backup in `backups_dir` with the stored key, verifies it, applies retention, logs, triggers the destination sync hook (a no-op until P6.2). Returns the backup entry |
| `save_file(actor, password, path)` | backup.manage | Verify password against `backupCheck`; plain copy → write v2 file to `path`; verify by decrypting the header and comparing SHA-256 after read-back; `meta.last_backup_at`; `backups_log`; change log event |
| `enable_auto(actor, password)` | backup.manage | Verify; derive key with a fresh salt; store key + salt through `platform.store_secret("backup-key")`; setting on |
| `disable_auto(actor)` | backup.manage | Delete stored key; setting off; warning dialog in the UI |
| `set_staff_allowed(actor, allowed)` | backup.manage | `backup_staff_can_run` |
| `run_auto(kind)` | internal | Uses stored key. Writes to `backups_dir`; applies retention (BACKUP_FORMAT.md); logs; calls the destination sync hook |
| `change_password(actor, current, new)` | backup.manage | Updates `backupCheck` and the stored auto key; old files still need old password (message says so) |
| `test_latest(actor)` | backup.run | Decrypt newest local backup to temp, integrity check, compare row counts with header counts, delete temp, log `verify` |
| `inspect(path, password)` | backup.manage, or no school (restore on first run) | Returns school name and code, created date, app and schema version, counts. Stores decrypted temp file under a preview id (30 minutes) |
| `restore(preview_id)` | backup.manage, or no school | School code must match the license's school code. Schema newer → refuse. Safety backup of current data first (if a school exists). Create a new encrypted database from the plain copy with the current key (`ATTACH ... KEY` + `sqlcipher_export`), run migrations on it, swap files atomically (rename current to `vidya.db.before-restore`, rename new into place), mark all devices `needs_resync = 1`, change log event, then ask the app to restart |

#### Task 4 — Scheduler (`src-tauri/src/background/backup.rs`)
- Runs only while the app has a valid license (a `ServerPermit` exists) or a school exists with the demo license in debug builds.
- On start and every 15 minutes: if auto is on and no daily backup today and local time ≥ 18:00, run `run_auto(daily)`; on app quit, if no daily backup today, run it before exit (with a small "Saving today's backup…" window state, max 30 seconds).
- The first backup of a month also counts as monthly.
- Before migrations of a new app version and before restore: `safety` backup (hook into the start sequence: if `user_version` < newest migration and a school exists, back up first using the stored auto key; if no auto key, block the upgrade with a screen asking the principal for the backup password).
- Home reminder (principal) when auto backups are off.

#### Task 5 — v1 prototype import
`import_v1(path, password)` in first-run restore: decrypt with PBKDF2-SHA256 (iterations from file) + AES-256-GCM, parse the prototype JSON, create a school through the same internal functions as setup (school code must match the license), give all staff new temporary passwords (`must_change = 1`) and show slips; principal must set a new password and a backup password. Tests with a `.vidyabak` generated by `reference/VidyaSchoolApp_step1.html` (the developer creates one with password `backup123` and puts it in `crates/vidya-backup/tests/fixtures/`).

#### Task 6 — Screens and commands
- Commands from API.md "Backup" that belong to this prompt: `backup_status`, `run_backup_now`, `save_backup_file`, `enable_auto_backup`, `disable_auto_backup`, `set_staff_backup_allowed`, `change_backup_password`, `test_backup`, `inspect_backup`, `restore_backup`, `import_v1_backup` (use the exact names in API.md; add any missing ones there).
- `views/Backup.jsx`:
  - Everyone with `backup.run`: status cards, a large **"Back up now"** button, "Test your backup", the list of recent backups.
  - Principal only (`can('backup.manage')`): "Save a backup file…", "Automatic backups" toggle with password, "Let staff at this computer run backups" toggle, "Change backup password", "Restore" (file picker → password → preview → confirm with typed school code → restart).
- The top-bar backup pill shows for users with `backup.run`: green "Backed up today", orange "No backup today" (tapping opens Backup).
- The first-run Activation screen shows "Restore from a backup file". The locked "other computer" screen enables "Save a backup file" (needs backup password).

#### Task 7 — Tests
- Round trip: every table identical (compare `SELECT *` ordered by primary key for all tables).
- Wrong password and one flipped byte → the same `WrongPasswordOrDamaged` error; no partial restore.
- Different school code refused.
- Retention keeps exactly 30 daily and 12 monthly with generated dates.
- Temp plaintext files are gone after success and after failure.
- Accountant at the office computer: `run_now` works with the stored key; with staff backups switched off → refused; `restore` → Permission. Accountant from a phone → `permission.office_computer_only`.
- Restore made on Windows (CI artifact from a test) opens on macOS test and vice versa: generate a backup in the Windows CI job, upload as artifact, and add a macOS job step that downloads and restores it (and the reverse).
- React Testing Library: an accountant sees "Back up now" but not "Restore".
- Matrix cases; remove `backup.manage` and `backup.run` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes; cross-platform restore CI steps pass
- [ ] Security-reviewer subagent run; findings fixed
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] After 6 pm a daily file appears in `~/Library/Application Support/in.vidya.school/backups`
- [ ] Sign in as anita on the office computer and press "Back up now": a manual backup appears
- [ ] "Test your backup" succeeds
- [ ] Restore a saved backup file on the Windows VM activated for the same school (replacement code)
- [ ] Import your prototype `.vidyabak` file on a fresh install

---

## P6.2 — Backup drives: sync backups to local and removable drives

| | |
|---|---|
| Phase | 6 Backups and year end |
| Depends on | P6.1 |
| Size | medium |
| Runs on | MacBook with a pen drive or external disk; Windows VM with a USB drive passed through |
| Read first | AGENTS.md, docs/BACKUP_FORMAT.md ("Backup destinations"), docs/PLATFORMS.md (`volume_info`, `removable_drives`), docs/DECISIONS.md (D30, D31), docs/API.md (backup destination commands) |

### Goal
The principal chooses where copies of the encrypted backups go: a folder on another internal drive (for example `D:\`), an external disk, a pen drive or a folder on the school network. Vidya keeps each destination in sync automatically after every backup and whenever a registered drive is plugged in. The principal or the staff member at the office computer can press "Sync backup drives" at any time, and gets a clear "Safe to remove" message. No internet is involved.

### Tasks

#### Task 1 — Platform: volumes (`platform/macos.rs`, `platform/windows.rs`)
- `volume_info(path)`:
  - Windows: `GetVolumePathNameW` → `GetVolumeNameForVolumeMountPointW` for a stable volume GUID; `GetDriveTypeW` for removable/network; label and serial with `GetVolumeInformationW`; free space with `GetDiskFreeSpaceExW`; the physical disk number through `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS` for same-disk detection. Verify each function and feature name with dependency-verifier.
  - macOS: `statfs` for mount point, file system type and network flag; volume UUID, label, removable/ejectable and the BSD whole-disk name through DiskArbitration (`DADiskCreateFromBSDName`, `DADiskCopyDescription`; verify bindings or declare the minimal `extern "C"` functions and document them). No shell commands.
- `removable_drives()`: mounted volumes that are removable or ejectable (macOS: exclude the boot volume and network volumes; Windows: drive type removable, plus USB-attached fixed drives if the bus type can be read with `IOCTL_STORAGE_QUERY_PROPERTY`), each with mount path, label and free space.
- Tests: `volume_info` on the data directory returns a non-empty id and is not removable (both OSes, Windows in CI); the list call does not fail when nothing is attached.

#### Task 2 — Destinations (`vidya-backup/src/destinations.rs` + `services/backup_destinations.rs`)
Stored as JSON in `app_settings.backup_destinations`:
```json
[{ "id": "uuid", "kind": "folder|removable|network", "label": "Pen drive SANDISK", "path": "E:\\", "volumeId": "…", "addedBy": "user id", "addedAt": "…", "lastSyncedAt": null, "lastError": null, "active": true }]
```
| Method | Permission | Rules |
|---|---|---|
| `list(actor)` | backup.run | Destinations with state: `connected` (path exists and the volume id matches), `not_connected` (removable drive not plugged in, or the letter or mount path now belongs to a different volume), `error`; last synced time; number of backups present; free space; `same_disk_warning` |
| `add(actor, path)` | backup.manage | Path must exist and be writable (write and delete a small test file). Kind from `volume_info`. Refused if inside the Vidya data folder (`backup.error.inside_data`). If the destination is on the same physical disk as the data folder: allowed only with `confirm = true`, and the dialog says "This folder is on the same disk as Vidya's data. If this disk fails, both are lost. Use a pen drive or external disk as well." Maximum 5 destinations. Creates `Vidya Backups/<schoolcode>/`. Change log event |
| `remove(actor, id)` | backup.manage | Removes it from the list only; files on the drive are left alone (dialog says so) |
| `sync_all(actor)` | backup.run | For every connected, active destination, run `sync_one`. Returns per-destination results |
| `sync_one(destination)` | internal | Read `manifest.json` (missing or damaged → start a new one; never delete unknown files). For each local backup file not present with the same SHA-256: copy to `.partial`, flush, rename, read back and compare SHA-256; update manifest (written as `.partial` then renamed). Apply retention to files listed in the manifest only. Check free space first (need file size + 10%); if short, try retention first, then fail with `backup.error.drive_full`. Update `lastSyncedAt` or `lastError` |
| `list_backups(actor, id)` | backup.manage | Backups at a destination from its manifest, each checked for existence, for the restore flow |

- After `run_auto` and `run_now` (P6.1 hook): sync all connected destinations in the background and emit `backup-changed`.
- Destination syncs never run at the same time as a backup being written (one mutex for all backup work).
- `meta.last_offsite_backup_at` is updated when a destination that is removable, a network folder, or on a different physical disk syncs successfully. The principal home reminder (7 days) and `backup_status` use it.

#### Task 3 — Drive plugged in
- A background watcher checks every 10 seconds (cheap: list removable drives and compare) for a registered removable destination becoming connected. When it appears: emit `backup-drive-connected`; if a user with `backup.run` is signed in, the UI shows a banner "Backup drive connected. Copying 3 backups…" and syncs automatically; when finished: "Backups copied. You can remove the drive safely." (on macOS, add an "Eject" button through DiskArbitration `DADiskUnmount` plus eject if verified; on Windows, tell the user to use "Safely Remove Hardware" if programmatic ejection is not available without admin rights, and say which you implemented).
- If nobody is signed in, the sync still runs (it needs no password because the key is not used: files are already encrypted) and the result shows on the Backup screen later.
- Unregistered pen drive plugged in: nothing happens automatically. The Backup screen shows "Use this drive for backups" to users with `backup.manage`, and "Copy backups to this drive once" to users with `backup.run` (a one-time copy of the newest backup with verification, not a registered destination).

#### Task 4 — Restore from a destination
In the restore flow (P6.1) add "From a backup drive": pick a destination or any removable drive, list backups for this school code from the manifest (or by reading headers of `*.vidyabak` files if there is no manifest), then continue with `inspect` and `restore`. On first run (no school) the same list is available from the Activation screen's "Restore from a backup file".

#### Task 5 — Commands and screen
- Commands: `list_removable_drives`, `list_backup_destinations`, `add_backup_destination`, `remove_backup_destination`, `sync_backup_destinations`, `list_destination_backups`, `copy_latest_backup_once` (add to API.md: **D**, backup.run).
- `views/Backup.jsx` gains a "Backup drives" card: each destination with a connected/not connected badge, last copied time ("3 days ago" in orange after 7 days), free space, and the same-disk warning. Buttons: "Sync backup drives" (backup.run), "Add backup drive or folder…" and "Remove" (backup.manage). "Add" offers the list of removable drives and a folder picker (dialog plugin) for internal or network folders.
- The top-bar pill shows "Copy backups to a drive" in orange when `last_offsite_backup_at` is older than 7 days.

#### Task 6 — Tests
- Sync copies only missing files; a second sync copies nothing.
- A corrupted copy on the destination (flip a byte) is detected by SHA-256 on the next sync and replaced.
- Retention at the destination deletes only manifest-listed files; an unrelated file in the folder survives.
- Destination full → `drive_full` and no `.partial` left behind.
- A drive letter reused by a different volume → `not_connected`, nothing written.
- Same-disk destination needs confirmation.
- Accountant: `sync_all` ok, `add` → Permission; phone → `permission.office_computer_only`.
- Watcher logic with a fake platform: drive appears → sync runs once.
- React Testing Library: an accountant sees "Sync backup drives" but not "Add backup drive".

### Do not
- Never write anything except encrypted `.vidyabak` files and `manifest.json` to a destination.
- Never delete files that Vidya did not list in the manifest.
- Never add internet or cloud upload code.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Security-reviewer subagent run on destinations and the watcher
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] As sunita, add a pen drive as a backup drive; the backups appear in `Vidya Backups/<school code>/`
- [ ] Unplug it, make a backup with "Back up now", plug it back in: the banner copies the new backup and says it is safe to remove
- [ ] Sign in as anita at the office computer and press "Sync backup drives"; she cannot add or remove drives
- [ ] Open a copied file in a text editor: only unreadable data
- [ ] In the Windows VM, add `D:\` (or a second virtual disk) and a USB drive; restore from the USB drive on a fresh install

---

## P6.3 — New academic session

| | |
|---|---|
| Phase | 6 Backups and year end |
| Depends on | P3.7, P6.1 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/DATA_MODEL.md (academic_sessions, enrollments, fee_plans, exams), docs/PRODUCT.md (glossary: session) |

### Goal
A guided, safe year-end change: promote students, keep last year's records viewable, carry forward fee plans, subjects and exams, and show previous session dues.

### Tasks

#### Task 1 — SessionService (`services/session_change.rs`)
| Method | Rules |
|---|---|
| `preview(actor, new_session_name) -> PromotionPreviewDto` | session.change (office computer only). Name valid (`parse_session_name`), follows the current session (e.g. 2026-27 → 2027-28), not existing. Requires a verified backup saved in the last 60 minutes (`meta.last_backup_at`, set by P6.1 for manual, automatic or saved-file backups) → otherwise Conflict `session.error.backup_first`. For every active enrollment: proposed class = next class by sort order; students in the last active class → `passed_out`; same section name if it exists in the next class, else first section. Summary counts per class. The preview is stored in memory with an id valid for 30 minutes |
| `commit(actor, preview_id, repeaters: Vec<student_id>, left: Vec<student_id>) -> NewSessionDto` | One transaction: create the new session (`is_current` moves to it; starts 1 April, ends 31 March), copy fee plans, transport fee and terms, copy active exams (same names and max marks) for the new session, mark old enrollments `promoted` / `passed_out` / `left`, create new enrollments (repeaters stay in the same class) with rolls assigned alphabetically by name within each section, keep rte, transport and reset concession to 0, change log `session.log.changed` with counts |
| `list_sessions(actor)` | settings.view: id, name, current |

#### Task 2 — Viewing old sessions
- Add optional `session_id` to `list_students`, `get_student`, `fee_register`, `get_fee_account`, `get_attendance`, `get_marks_sheet`, `get_report_card`, `reports_summary`, `day_book` (default current). Old sessions are **read-only**: every write method refuses a non-current session with `session.error.read_only`.
- `get_fee_account` returns `previous_session_dues`; collecting fees always goes to the current session. Add a "Previous dues" column to the fee register and to exports.
- Update docs/API.md inputs.

#### Task 3 — Frontend
- Settings → "Start new session" guided screen (`views/settings/NewSession.jsx`): step 1 name and backup check (a "Back up now" button that calls `run_backup_now` and then re-checks), step 2 preview table per class with checkboxes for repeaters and students who left, step 3 confirm dialog with counts, step 4 done.
- A session selector in the top bar for the principal (and accountant for fees) showing "Viewing 2026-27 (read-only)" when not current. The selected session lives in a `SessionYearProvider` context and is passed to the query hooks.

#### Task 4 — Tests
- Promotion of the sample school: V-A students → VI-A; VIII → passed_out; a repeater stays in V; rolls restart at 1.
- Old session receipts and attendance still readable; writes refused.
- Previous session dues appear for a student who did not pay fully.
- Without a recent backup → refused.
- Matrix case; remove `session.change` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] On a copy of the sample school: press "Back up now", start session 2027-28, mark one repeater; check a promoted student's class, previous dues, and that 2026-27 is viewable but read-only

---

## P7.1 — Licensed LAN server and background running

| | |
|---|---|
| Phase | 7 Local network and sync |
| Depends on | P4.4, P6.1 |
| Size | large |
| Runs on | MacBook; Windows via VM (bridged network) |
| Read first | AGENTS.md, docs/ARCHITECTURE.md (section 4), docs/DECISIONS.md (D29), docs/SYNC_PROTOCOL.md (section 7), docs/API.md ("LAN HTTP API"), docs/PLATFORMS.md |

### Goal
The licensed office computer, and only that computer, runs an HTTPS server on the school's local network with its own certificate, restricted to private addresses and rate-limited, and keeps running in the tray or menu bar and awake during school hours.

### Dependencies allowed in this prompt
`axum` (default features off; `http1`, `json`, `ws`, `tokio` only), `axum-server` (tls-rustls), `rustls` (with the smallest crypto provider that supports the chosen certificate key type; compare `ring` and `aws-lc-rs` for size and build on Windows and Android, and record in `docs/SIZE.md`), `rcgen`, `tokio` (only the features used), `tower` (only if axum needs it for layers; verify), `tauri` `tray-icon` feature, `tauri-plugin-autostart`. Use dependency-verifier to make sure rustls versions and crypto providers line up between axum-server and the HTTP client used later by `vidya-client` (one provider in the whole tree).

### Tasks

#### Task 1 — Server identity (`vidya-server/src/identity.rs`)
- On first server start: generate an ECDSA P-256 or Ed25519 key with `rcgen` (pick the one rustls accepts for server certificates in the installed versions; verify), self-signed certificate valid 20 years, subject alternative names: `vidya.local` plus `localhost`. Phones do not validate names; they pin the fingerprint.
- Store in `server_identity`; fingerprint = SHA-256 of the DER certificate, hex.
- `fingerprint_short()` = first 16 hex characters.

#### Task 2 — Server (`vidya-server/src/lib.rs`)
```rust
pub struct ServerConfig { pub port: u16 /* 47631 */, pub services: Arc<Services>, pub identity: ServerIdentity }
pub struct ServerHandle { pub fn stop(self); pub fn status(&self) -> ServerStatus; pub fn broadcaster(&self) -> Broadcaster }
pub async fn start(permit: Arc<ServerPermit>, config: ServerConfig) -> Result<ServerHandle, ServerError>;
```
- `start` requires the `ServerPermit` from P4.2 (D29). The school code comes from the permit, not from the config. Before binding, `start` re-reads the license row and asks `LicenseService` to confirm the permit is still valid for this computer; if not, return `server.error.not_licensed`.
- Every 30 minutes the server re-checks the license the same way; if it is no longer valid (license row edited or data copied to another computer while running), stop the server, stop discovery, and emit `server-stopped` with reason `license`.
- Listen on `0.0.0.0:<port>` (IPv4). If the port is busy, return a clear error `server.error.port_busy`; the app shows it on the connection check screen with a setting to change the port.
- Middleware (in this order): source IP filter (only 10/8, 172.16/12, 192.168/16, 169.254/16, 127/8; else close connection, count as blocked), request size limit 5 MB (sync push), per-device rate limit (token bucket in memory: sign-in routes 10/min per source IP, other routes 120/min per device id), request id for logs, error mapping to the JSON error format with HTTP statuses from API.md.
- Routes now: `GET /api/v1/health` → `{ schoolCode, version, fp, licensed: true }`; `GET /api/v1/time` → `{ nowMs }`. Other routes come in P7.3 and P7.5.
- Logs: method, path, status, duration, device id; never bodies.
- Local network only: no UPnP, NAT-PMP, port forwarding, relay, tunnel or outgoing internet connection anywhere in the server crate. Add a test that `cargo tree -p vidya-server` contains no HTTP client crate.

#### Task 3 — App integration (`src-tauri/src/background/server.rs`)
- Start only when `AppState` has a `ServerPermit` (valid license, school exists); stop on quit. Restart when the port setting changes.
- Unlicensed states never start it: `NotActivated`, `OtherComputer`, `Tampered`. Add a test with the fake platform for each.
- Command `server_status` (devices.view) → `{ running, port, blockedCount, reasonNotRunning }` where the reason is one of `not_licensed`, `no_school`, `port_busy`, `stopped`.

#### Task 4 — Tray and menu bar
- Tray icon (Windows) / menu bar icon (macOS) with menu: Open Vidya, Phones can connect: yes/no (disabled item showing status), Quit Vidya.
- Closing the main window hides it when `keep_running_in_tray` is on (default). Show a one-time notification: "Vidya is still running so phones can sync. Quit from the tray icon."
- Quit asks for confirmation if phones synced in the last 10 minutes.
- Reuse the app icon files for the tray icon (a separate small monochrome PNG for the macOS template icon is fine); do not add icon libraries.

#### Task 5 — Start at login and keep awake
- `tauri-plugin-autostart` for start at login (setting `start_at_login`, default on). On macOS verify how the plugin registers (login item vs launch agent) and mention it in the summary.
- Keep awake: implement `Platform::set_keep_awake` — Windows `SetThreadExecutionState` with continuous + system required while on; macOS IOKit power assertion preventing idle system sleep (create once, release when off). A background task checks school hours every minute and turns it on/off. Display may still sleep.
- Settings screen gets these toggles (already stored in app_settings from P3.6).

#### Task 6 — Tests
- Integration test: start server on `127.0.0.1` random port with a test identity and a permit created through `LicenseService` with the test key; `GET /health` over HTTPS with a client that trusts that certificate returns the school code and `licensed: true`.
- License edited in the DB while running → the periodic check (called directly in the test) stops the server.
- IP filter unit tests (private, public, IPv6 mapped).
- Rate limiter: 11th sign-in in a minute → 429.
- Identity survives restart (same fingerprint).

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Size row added to `docs/SIZE.md` (the server stack is the biggest addition so far; stop and report if any artefact crosses 25 MB)
- [ ] PROGRESS.md "Check in Windows VM" item: open `https://<vm-ip>:47631/api/v1/health` from the MacBook browser (accept the warning) and see the school code; the tray icon works; the VM does not sleep during school hours

### Done when (developer)
- [ ] From your phone's browser on the same Wi-Fi, `https://<macbook-ip>:47631/api/v1/health` shows the school code (accept the certificate warning in the browser; the app will pin it)
- [ ] On a fresh data folder before activation, the same address does not answer
- [ ] Closing the window leaves Vidya in the menu bar; Quit works
- [ ] Windows VM checks done

---

## P7.2 — Discovery, firewall and connection check

| | |
|---|---|
| Phase | 7 Local network and sync |
| Depends on | P7.1 |
| Size | medium |
| Runs on | MacBook; Windows via VM |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (section 8), docs/PLATFORMS.md (incoming connections, discovery), docs/DECISIONS.md (D29) |

### Goal
Phones can find the licensed office computer by school code without QR codes, the operating system allows the connections, a second computer answering for the same school is detected, and the principal has a screen that explains what is wrong when phones cannot connect.

### Dependencies allowed in this prompt
`mdns-sd` (check its size and dependencies; if it pulls a large tree, report and propose using only the UDP responder), `tokio` (UDP).

### Tasks

#### Task 1 — mDNS advertise (`vidya-server/src/discovery/mdns.rs`)
Service type `_vidya._tcp.local.`, instance `Vidya-<schoolcode>`, port, TXT `school`, `fp`, `v=1`, host name from the computer name sanitised. Register on all IPv4 interfaces that are up and private. Re-register when network interfaces change (poll every 30 seconds or use the crate's interface change support if present; verify). Unregister on stop. Starting discovery also requires the `ServerPermit`.

#### Task 2 — UDP responder (`discovery/udp.rs`)
Bind `0.0.0.0:47632`. Accept only datagrams ≤ 128 bytes of the form `VIDYA-DISCOVER <schoolcode>` from private source IPs; reply to the sender `VIDYA-HERE <schoolcode> <port> <fp16>`. Wrong code, bad format, public IP → no reply. Rate limit 20 replies per source per minute.

#### Task 3 — Discovery client for tests and phones (`vidya-client/src/discovery.rs`)
`discover(school_code, timeout) -> Vec<Found { address, port, fp16, method }>`: mDNS browse up to 4 seconds, then 3 UDP broadcasts one second apart to `255.255.255.255:47632` and to each interface's directed broadcast address. Results are grouped by `fp16`.
- If the phone already has a pinned fingerprint, it uses only the answer with that fingerprint.
- If it has none and more than one distinct fingerprint answers for the same school code, return `DiscoveryError::MultipleServers` with the addresses (the phone shows "Two office computers answered for this school. Ask the principal to switch Vidya off on the old computer." in P8.3).
Used by the phone in P8.3 and by a developer tool now: `cargo run -p vidya-client --example discover -- vaani` prints what it finds.

#### Task 4 — Windows firewall
- NSIS installer hook (Tauri NSIS hooks: check the installed version's documented hook mechanism) that adds inbound rules for the installed `Vidya.exe`: TCP 47631 and UDP 47632, profiles Private and Domain (and Public, per D-level decision: **ask the developer**; default recommendation: Private and Domain only, with the connection check telling the principal to set the school Wi-Fi to Private), remote address LocalSubnet. Uninstall hook removes them. Use `netsh advfirewall firewall` commands (verify syntax on Microsoft Learn).
- VM check steps in PROGRESS.md: rule visible in Windows Defender Firewall with Advanced Security after install and gone after uninstall.

#### Task 5 — macOS permissions
- Read Apple's current documentation for local network privacy and incoming connections. Add the required Info.plist keys through Tauri's macOS config (verify how Tauri 2 merges a custom Info.plist): local network usage description ("Vidya connects to staff phones on your school Wi-Fi."), Bonjour services `_vidya._tcp`.
- Before starting the server for the first time, show a Vidya screen: "Your Mac will ask to allow connections. Choose Allow so staff phones can connect."

#### Task 6 — Connection check (`connection_check`, devices.view)
`ConnectionCheckDto`: licensed and server permit present; server running and port; local IPv4 addresses with interface names; Wi-Fi network name if the OS gives it without extra permissions (else omit); mDNS registered yes/no; UDP responder yes/no; self-test result (the app runs `discover` against itself on each interface and reports found or not); **other servers**: any answer for this school code with a different fingerprint (address shown; message "Another computer is answering as this school's office computer at 192.168.1.23. Switch Vidya off there."); Windows: firewall rules present (query via `netsh`), network profile Private/Public (via Windows API; verify a non-PowerShell method if possible); macOS: application firewall on/off and whether Vidya is allowed (read-only query; verify method), local network permission hint. Buttons: "Open network settings" (opener with the OS settings URL; verify URLs), "Copy report".
Plain explanations shown when something fails: not activated (the server needs a valid license), guest networks and "client isolation" or "AP isolation" block phones; mobile hotspots from phones work; set Windows network to Private.
`views/ConnectionCheck.jsx` renders the DTO with a green tick or orange cross per line.

#### Task 7 — Tests
- UDP responder: right code replies, wrong code silent, oversize ignored.
- `discover` with two fake responders with different fingerprints → `MultipleServers`; with a pin → only the pinned one.
- mDNS: advertise then browse on localhost within the test (mark as integration; skip gracefully if multicast unavailable in CI, and report).
- `discover` example compiles.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] VM check items added
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] With Vidya running on the MacBook, run `cargo run -p vidya-client --example discover -- <your school code>` from the MacBook: found
- [ ] Install a free mDNS/Bonjour browser app on the Android phone; `_vidya._tcp` is listed
- [ ] Windows VM (bridged): discover from the MacBook finds the VM; the connection check screen shows firewall rules and profile correctly
- [ ] Run the sample school on both the MacBook (debug, `VIDYA_DEV_ALLOW_DEMO_SERVER=1`) and the VM: the connection check on each reports the other computer
- [ ] Decide the Public profile question the agent asked

---

## P7.3 — Phone approval, signed requests and device management

| | |
|---|---|
| Phase | 7 Local network and sync |
| Depends on | P7.2 |
| Size | large |
| Runs on | MacBook; test client script; Windows via VM |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (section 7), docs/API.md (LAN HTTP API, "Devices and LAN"), docs/DATA_MODEL.md (devices, pending_approvals, used_request_nonces), docs/PERMISSIONS.md (devices.*, office computer only) |

### Goal
A new phone signs in with username and password, the principal sees a matching 6-digit code on the office computer and allows it, the phone then holds a session and signs every request with its own device key. The principal can see and remove devices. Every request from a phone is treated as `Origin::Phone`. Tested with a command-line test client until the Android app exists.

### Tasks

#### Task 1 — Match code (`vidya-sync/src/match_code.rs`)
`pub fn match_code(cert_fingerprint: &[u8; 32], device_public_key: &[u8; 32]) -> String` exactly as SYNC_PROTOCOL.md section 7 (returns `"482913"`; `format_match_code` returns `"482 913"`). Test with fixed bytes; write the expected value into the test after computing it once, and add a second independent computation in the test (manual SHA-256 → first 3 bytes → modulo) so the vector is not copied from the implementation blindly.

#### Task 2 — Request signing (`vidya-sync/src/signing.rs`)
- `signing_string(method, path_with_query, time_ms, nonce, body) -> Vec<u8>`: exactly `METHOD\nPATH\nTIME\nNONCE\nhex(sha256(body))`.
- `sign_request(key, ...) -> String` (base64) and `verify_request(public_key, ..., signature) -> bool`.
- Nonce: 16 random bytes, base64url.
Tests: sign then verify; changed path, body, time or nonce fails.

#### Task 3 — DeviceService (`services/devices.rs`)
| Method | Rules |
|---|---|
| `remote_sign_in(input) -> RemoteSignInResult` | Input: username, password, device_public_key (32 bytes base64), device_name (≤ 40 chars), platform. Same password and lockout logic as `AuthService::sign_in` (refactor to share) with `Origin::Phone`. If device key belongs to an approved, non-revoked device of **this** user → full session bound to that device id (or `MustChangePassword`). If the key belongs to another user → Permission `devices.error.other_user`. If revoked → status Gone. Unknown key → create or reuse a pending approval (one per user+key, expires 10 minutes), compute match code with the server fingerprint, emit `approval-requested`, return `WaitingForApproval { approval_id, match_code }` and **no session** |
| `approval_status(approval_id, device_public_key)` | Pending → waiting; allowed → a full session for that device (requires the same key); refused/expired → error `devices.error.refused` / `devices.error.expired` |
| `list_pending(actor)` | devices.approve (office computer only) |
| `decide(actor, approval_id, allow, receipt_prefix)` | devices.approve (office computer only). Allow: approved non-revoked devices < `ServerPermit::max_devices()` (`devices.error.limit`); receipt prefix required for accountant and principal phones (suggest T1, T2… for teachers too, used later if the role changes), validated and unique, not equal to `meta.device_code`; insert `devices`; change log. Refuse: mark decided |
| `list(actor)` | devices.view: name, platform, user, receipt prefix, approved, last seen, status |
| `revoke(actor, device_id)` | devices.manage (office computer only): set revoked, end its sessions, change log |
| `verify_signed_request(headers, method, path, body) -> Result<DeviceContext>` | Device exists and not revoked (revoked → Gone); time within ±5 minutes of server clock; nonce unused in the last 10 minutes (insert into `used_request_nonces`, purge older rows every 10 minutes); signature valid; session token belongs to this device; returns device id and `Actor` with `origin = Phone` |

#### Task 4 — Routes (`vidya-server/src/routes/auth.rs`)
`POST /auth/sign-in`, `GET /auth/approval/{id}` (requires header `X-Vidya-Device-Key`), `POST /auth/first-password`, `POST /auth/sign-out`. An axum extractor `SignedDevice` runs `verify_signed_request` for every route except health, time, sign-in and approval polling. HTTP 410 for revoked devices with body kind `auth` key `devices.error.removed`. There is no header, field or route that lets a phone claim `OfficeComputer` origin; add a test that sends such a field and still gets a Phone actor.

#### Task 5 — Office computer screens and commands
- Commands: `list_pending_approvals`, `decide_approval`, `list_devices`, `revoke_device`.
- When `approval-requested` arrives: system notification (tauri-plugin-notification; record size impact) and an `ApprovalBanner` component on every screen for the principal: "Sierra D'Souza wants to sign in on Redmi Note 12. Check the code on her phone is 482 913." with Allow and Refuse. Allow opens a small dialog for the receipt prefix. If no principal is signed in, the banner appears after the principal signs in; approvals expire after 10 minutes and the phone shows it.
- `views/Devices.jsx` (principal, nav item "Phones"): list with Remove (confirm dialog), connection check link.

#### Task 6 — Test client (`crates/vidya-client/examples/test_phone.rs`)
Command-line tool the developer can run on the MacBook: `cargo run -p vidya-client --example test_phone -- --school vaani --user sierra --password <pw>`. It creates or loads a device key from `./.test-phone-key` (git-ignored), discovers the server (P7.2), accepts the certificate for the approval flow, prints the match code, polls until approved, pins the fingerprint in `./.test-phone-pin`, then calls a signed `GET /api/v1/time`. Later prompts extend it.
The HTTPS client here uses a custom rustls certificate verifier that accepts only the pinned fingerprint (or, before approval, the fingerprint from discovery `fp16` prefix check, then stores the full one), and refuses a server whose `/health` does not return `licensed: true`. Verify the rustls API for custom verifiers in the installed version.

#### Task 7 — Tests
- Unknown device → waiting, no token in response.
- Approve → polling returns session; signed request works.
- Refused and expired paths.
- max_devices limit from the permit.
- Replay: same nonce twice → 401; time 6 minutes off → 401; changed body → 401.
- Revoked → 410 on the next request.
- A teacher's session token used with another device's key → 401.
- Wrong password counts toward lockout exactly like local sign-in.
- A principal's phone calling an office-only action through any route → Permission `permission.office_computer_only`.
- Matrix cases; remove `devices.*` from NOT_YET_IMPLEMENTED.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Security-reviewer subagent run on `devices.rs`, `signing.rs`, auth routes, and the test client verifier

### Done when (developer)
- [ ] Run the test client from the MacBook against the Mac app: the same 6-digit code shows in Terminal and on the Vidya banner; Allow works
- [ ] Remove the device in Vidya; the test client's next request says the phone was removed
- [ ] Repeat with the Windows VM as the server

---

## P7.4 — Sync engine core

| | |
|---|---|
| Phase | 7 Local network and sync |
| Depends on | P7.3 |
| Size | large |
| Runs on | MacBook (pure logic and simulation, no network) |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (entire), docs/DATA_MODEL.md (change_log, field_clocks, field_history, outbox, alerts), docs/PERMISSIONS.md |

### Goal
The rules for applying changes on the office computer and on phones, the role filter for outgoing data, and a simulation that proves many devices converge. No HTTP yet (P7.5).

### Tasks

#### Task 1 — Envelope types (`vidya-sync/src/envelope.rs`)
`Envelope`, `Op`, `EntityKind`, `PushRequest`, `PushResponse`, `Rejected`, `PullResponse`, `SnapshotPage` exactly as SYNC_PROTOCOL.md sections 3 and 4, serde camelCase. JSON round-trip tests.

#### Task 2 — Services emit envelopes
Refactor every write service method so the change-log payload **is** the envelope `fields` (the minimal data needed to replay the change), not a free-form description. Add `op` and `entity` per method. List in `docs/SYNC_PROTOCOL.md` a new table "Service method → entity, op, fields" (update the doc; this is allowed and required here). Office-computer-only methods (backup, destinations, settings, users, devices, session change, import) produce change-log entries but are marked `phone_pushable = false` in the table.

#### Task 3 — Apply on the server (`services/sync_apply.rs`)
`SyncService::apply(actor_from_server_user, envelope) -> Result<AppliedMeta, Rejected>` inside the caller's transaction:
- The actor always has `origin = Phone` (built in P7.3); office-only changes are therefore refused by the normal permission check.
- Idempotency: `change_id` exists in `change_log` → return already-applied.
- `observe` the envelope HLC.
- Dispatch by entity and op to the **same internal functions** the local commands use (with permission and validation), passing the envelope's HLC and the phone's device id:
  - `update_fields`: for each field compare with `field_clocks`; newer wins and updates the clock; older loses and writes `field_history` (losing value, winning value). Different fields are independent.
  - `replace_set` attendance: compare with `attendance_days.updated_hlc`; newer replaces all marks; older is recorded in `field_history` with the whole set and creates alert `attendance_replaced`.
  - marks `insert`/`update_fields` per cell with the same-field rule.
  - `append` receipt: validate as `FeeService::collect` except the balance rule: a receipt whose amount is above the balance at apply time is **accepted** (money was received) and creates alert `overpayment`. Receipt number must start with the device's approved prefix and must not already exist; the device's counter on the server moves to at least that number.
  - `append` cancellation: principal only; already cancelled → idempotent success.
  - student `insert` via push → Rejected (`students.error.online_only`).
- Any permission or validation failure → `Rejected { change_id, kind, message_key, params }`.
- Every accepted change appends to `change_log` with the original `change_id`, `hlc`, `device_id`, `user_id`.

#### Task 4 — Undo from activity
`undo_field_change(actor, history_id)` (activity.view + settings.edit): writes the losing value back as a new local change with a new HLC, marks `undone_at`. Command `undo_field_change` (**D**). `Activity.jsx` shows "Changed on two devices" items with an Undo button.

#### Task 5 — Outgoing filter (`services/sync_filter.rs`)
`fn visible(actor, change_row) -> Option<Envelope>`: implements SYNC_PROTOCOL.md "GET /sync/pull" rules by entity and removes forbidden fields (use an explicit allow-list of fields per entity per role; unknown fields are removed). `fn snapshot_tables(actor) -> Vec<TableSpec>` with SQL filters per role.
Tests: a table-driven test for every entity × role asserting exact field sets; a teacher never receives `receipt`, `receipt_cancellation`, `user`, fee plan rows, or enrollment fields `rte`, `transport`, `concession`. No role ever receives backup settings, backup destinations, license rows, server identity private key or device keys of other phones.

#### Task 6 — Client side (`services` in Mode::Client)
- Local writes in client mode: apply locally with the same functions, then insert the envelope into `outbox` in the same transaction; do not require `server` rules that need live data (admission numbers).
- `apply_pulled(envelope)`: skip if `change_id` exists locally; apply with the same field clock rules (the server already resolved clashes, so the pulled value normally wins; phones still keep clocks to order late arrivals).
- `handle_rejected(rejected, server_current_rows)`: replace affected local rows with the server's current rows, mark the outbox row rejected with reason, keep for display.
- Receipt numbers on phones: `receipt:<prefix>` counter locally.

#### Task 7 — Simulation harness (`vidya-testkit/src/sim.rs` + `vidya-sync/tests/convergence.rs`)
- One server `Services` (Mode::Server) and three client `Services` (Mode::Client) with separate in-memory DBs and fixed clocks, sharing the sample school through a snapshot built by `snapshot_tables`.
- Operations generator (seeded): teacher attendance saves and marks edits for own sections, accountant receipts and student field edits, principal concession and cancellations on the office computer, clock skew, going offline/online, sync in random order.
- `sync(client, server)`: push outbox → server apply in one transaction → client handle results → pull visible changes → client apply.
- Property: after everyone syncs twice with no new operations, for every client, all data the client's role may see equals the server's data (compare via the snapshot filter); receipts sum equal; no duplicate receipt numbers; server `change_log` has no duplicate `change_id`.
- Run 200 seeds in normal `cargo test`, and 1,000 with `VIDYA_SIM_RUNS=1000` (documented).
- One named test per clash rule in SYNC_PROTOCOL.md section 5.

### Done when (agent)
- [ ] `cargo test -p vidya-sync -p vidya-services` passes; `VIDYA_SIM_RUNS=1000` passes (report run time)
- [ ] SYNC_PROTOCOL.md method table added
- [ ] Security-reviewer subagent run on apply and filter code
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Run `VIDYA_SIM_RUNS=1000 cargo test -p vidya-sync --test convergence` once on the MacBook: passes

---

## P7.5 — Sync endpoints and live updates

| | |
|---|---|
| Phase | 7 Local network and sync |
| Depends on | P7.4 |
| Size | medium |
| Runs on | MacBook with test client; Windows VM as server |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (sections 4, 6, 7), docs/API.md (LAN HTTP API) |

### Goal
The licensed office computer exposes snapshot, push, pull, live WebSocket and online admission endpoints over the school Wi-Fi; the shared `vidya-client` sync loop drives them; the test client proves it end to end over real Wi-Fi.

### Tasks

#### Task 1 — Server routes (`vidya-server/src/routes/sync.rs`, `students.rs`)
- `GET /sync/snapshot?page=N`: role-filtered pages of 500 rows per table from `snapshot_tables`, consistent across pages: page 0 opens a read transaction, records `upTo = max(change_log.seq)`, and materialises the rows into a temporary in-memory structure keyed by snapshot id (expires 10 minutes); later pages read from it with `snapshotId`. Responses are compressed with the compressor already in the tree (from P6.1) if the client sends the matching `Accept-Encoding`; do not add a new compression crate.
- `POST /sync/push`: ≤ 500 changes; one write transaction per request; build the actor from the server's user record for the device's session (never from the body); per change call `SyncService::apply`; commit; broadcast `upTo`; update `devices.last_seen_at`.
- `GET /sync/pull?since=N`: up to 1,000 visible changes after N, `more` flag, `rejectedCurrent` rows for ids requested in `?refresh=entity:id,...`; update `devices.last_pull_seq`. If `devices.needs_resync = 1` → HTTP 409 kind conflict key `sync.resync_required` (clear the flag after the next completed snapshot).
- `GET /sync/live`: WebSocket after signed upgrade request; sends `{ upTo }` on each broadcast; ping 25 s; closes when the device is revoked or the server stops for license reasons.
- `POST /students`: calls `StudentService::add` in Server mode with the phone's actor; returns the student DTO for that role.
- Local commands also broadcast after writes, so phones hear about office changes.
- Emit `data-changed` to the office window after applying pushed changes; `useQuery` hooks on visible screens reload on that event (add an optional `refreshOn: ['data-changed']` to `useQuery`).

#### Task 2 — Client sync loop (`vidya-client/src/sync.rs`)
```rust
pub struct SyncClient { .. }   // server address, pinned fingerprint, device key, session token, Services (client mode)
pub async fn run_cycle(&self) -> Result<SyncReport, SyncError>;   // push all outbox (batches of 500) → handle results → pull until !more → apply
pub async fn initial_snapshot(&self) -> Result<(), SyncError>;    // all pages into a temp DB file, then swap into place in one step
pub fn spawn_live(&self, on_up_to: impl Fn(u64)) -> JoinHandle<()>; // reconnect backoff 1,2,5,10,30 s
```
- `GET /time` at the start of each cycle: store `clock_offset_ms`; warn if > 5 minutes.
- Only private addresses are ever contacted; the client refuses to connect to a public IP even if one is configured.
- Errors mapped: network unreachable → `NotOnSchoolWifi`; 401 → `SignInAgain`; 410 → `Removed`; 409 resync → run snapshot; health without `licensed: true` or a fingerprint mismatch → `NotYourSchool`.
- The same pinned HTTPS client as the test client. Use one small HTTP client stack shared with the test client (hyper-based or reqwest with default features off and rustls with the provider chosen in P7.1); record its size on Android later in P8.1.

#### Task 3 — Test client extension
`test_phone` subcommands: `snapshot`, `mark-attendance --section V-A`, `collect --student <adm> --amount 500` (for an accountant login), `sync`, `live` (prints `upTo` events). It uses a local temp DB with `vidya-services` client mode, so it exercises the real client code.

#### Task 4 — Tests
- Integration test in one process: server on 127.0.0.1 with sample school, two clients (teacher, accountant): snapshot sizes per role; teacher pushes attendance and accountant sees nothing of it; accountant pushes a receipt and the office day book includes it; live WebSocket delivers `upTo` within 1 second.
- Interrupted push (drop connection after server commit, before response): retry is idempotent.
- `needs_resync` path.
- Snapshot consistency: writes during paging do not produce partial state.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] Size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] Mac app running as server. From the MacBook Terminal: `test_phone` as sierra marks attendance for V-A; it appears in Vidya within seconds
- [ ] `test_phone` as anita collects ₹500; the office day book shows a T1- receipt
- [ ] Turn Wi-Fi off, run `mark-attendance` (queued), turn it on, run `sync`: it arrives
- [ ] Repeat the first two checks with the Windows VM as server

---

## P8.1 — Android project and client-only build

| | |
|---|---|
| Phase | 8 Android |
| Depends on | P7.5 |
| Size | medium |
| Runs on | MacBook with a real Android phone over USB |
| Read first | AGENTS.md, docs/ARCHITECTURE.md (sections 3 and 5), docs/PLATFORMS.md (Android column), docs/DECISIONS.md (D28, D29), docs/API.md (**M** commands) |

### Before you start (developer)
1. Install Android Studio. In Settings → Languages & Frameworks → Android SDK install: an Android SDK Platform (the version Tauri's docs recommend), SDK Platform-Tools, SDK Build-Tools, SDK Command-line Tools, and the NDK (Side by side).
2. Add to `~/.zshrc` (Claude Code will give you exact paths after checking your folders): `JAVA_HOME` pointing to Android Studio's bundled JDK, `ANDROID_HOME`, `NDK_HOME`, and `$ANDROID_HOME/platform-tools` on `PATH`. Open a new Terminal.
3. On the phone: Settings → About phone → tap Build number 7 times → Developer options → USB debugging on. Connect by USB and accept the prompt. `adb devices` must list the phone.
4. Run `./scripts/doctor.sh`: Android items must be ✓ (Rust Android target is added in this prompt).

### Goal
The same Tauri project builds an Android app that contains only client features (no server, discovery responder, backup or export code), runs on the phone, shows the mobile sign-in screen, and fits the 30 MB budget.

### Tasks

#### Task 1 — Toolchain
- Read the Tauri 2 Android prerequisites page for the installed version. Add the Rust targets it lists (at least `aarch64-linux-android`; also `armv7-linux-androideabi`, `x86_64-linux-android` only if needed for emulator or older phones).
- Update `rust-toolchain.toml` targets and `scripts/doctor.sh`.

#### Task 2 — Init
- `npm run tauri android init`. Commit `src-tauri/gen/android` (except build outputs, which should be ignored by the generated `.gitignore`; check).
- Report the minimum Android version (minSdk) Tauri sets; set it to that or API 26 (Android 8), whichever is **higher**, in the Tauri config (verify key name) — not by hand-editing generated Gradle files unless Tauri documents that.
- App id stays `in.vidya.school`, app name "Vidya".

#### Task 3 — Feature split
- `src-tauri/Cargo.toml` features: `default = ["desktop-server"]`, `desktop-server = [deps: vidya-server, vidya-backup, vidya-export, tauri-plugin-single-instance, tauri-plugin-autostart, tauri-plugin-dialog, tray-icon ...]`, `mobile-client = [deps: vidya-client]`. Make optional dependencies `optional = true`.
- Find how to pass `--no-default-features --features mobile-client` to Android builds with the Tauri CLI (check `npm run tauri android build -- --help` for a features flag) and put it in npm scripts: `android:dev`, `android:build:apk`, `android:build:aab`.
- Gate modules and commands with `#[cfg(feature = "desktop-server")]` / `#[cfg(feature = "mobile-client")]`. Two `generate_handler!` lists selected by feature. Update `scripts/check-api-drift.mjs` to understand **D**/**M** markers: Rust desktop list ⊆ D+unmarked, mobile list ⊆ M+unmarked.
- `ServerPermit`, `LicenseService::server_permit` and every office-only service are compiled only with `desktop-server`.
- A CI step on ubuntu-latest: `cargo check -p vidya-app --no-default-features --features mobile-client --target aarch64-linux-android` (with NDK set up via a well-known action; read its README) — at minimum `cargo check` with the target, to catch desktop code leaking into mobile. Add `cargo tree` assertions in the same step that `vidya-server`, `vidya-backup`, `vidya-export`, `axum` and `rcgen` are **not** in the mobile tree.

#### Task 4 — Frontend entry
- Vite: build mode `mobile` (`vite build --mode mobile`) through `src/main.jsx` (P1.1), selected in the Tauri Android `beforeBuildCommand`/`beforeDevCommand` (use Tauri's platform-specific configuration file for Android; verify its file name and merge behaviour).
- `MobileApp.jsx` shows the mobile shell: sign-in screen (username with `@school`, password), bottom tab bar. Nothing else connected yet.
- Viewport meta, safe areas (notch), and large touch targets from UI_GUIDE.md.
- `scripts/check-bundle-split.mjs` passes: no desktop-only view (Backup, Settings, Users, Devices, Setup, Activation) in the mobile bundle.

#### Task 5 — Size and run
- `npm run android:dev` runs on the connected phone.
- `npm run android:build:apk` for arm64 only (check the split-per-ABI or target option); run `npm run size` and record APK and AAB sizes in `docs/SIZE.md`. Target 20 MB; hard limit 30 MB. If over 20 MB, list the largest contributors (`cargo bloat` for the Android target if it works, otherwise explain) and propose reductions; if over 25 MB stop for a decision.
- Add the Android arm64 APK size check to CI (build in the ubuntu job if the build time is acceptable, otherwise weekly scheduled; state which).

#### Task 6 — Android manifest basics (through Tauri config or documented manifest edits)
- Permissions: `INTERNET`, `ACCESS_NETWORK_STATE`, `ACCESS_WIFI_STATE`, `CHANGE_WIFI_MULTICAST_STATE`. Check Android's current documentation for any local-network permission required by the target SDK Tauri uses; report what you found with the page link.
- `android:allowBackup="false"` and data extraction rules that exclude everything, so school data never goes to Google's Android backup.
- `usesCleartextTraffic` false.

### Done when (agent)
- [ ] `npm run verify` passes; the Android `cargo check` CI step and tree assertions are green
- [ ] APK size recorded (≤ 30 MB, target 20 MB)
- [ ] minSdk and permission findings reported with sources

### Done when (developer)
- [ ] The Vidya sign-in screen opens on your phone from `npm run android:dev`
- [ ] The APK installs by copying it to the phone (allow "install unknown apps" for your file manager); Settings → Apps → Vidya shows its size under 30 MB

---

## P8.2 — Android secure storage and local database

| | |
|---|---|
| Phase | 8 Android |
| Depends on | P8.1 |
| Size | medium |
| Runs on | MacBook + phone |
| Read first | AGENTS.md, docs/PLATFORMS.md (Android), src-tauri/AGENTS.md |

### Goal
On the phone, the SQLCipher database opens with a key protected by Android Keystore, secrets are stored the same way, and the device's Ed25519 key is created and kept safely.

### Tasks

#### Task 1 — Tauri mobile plugin in Kotlin (`src-tauri/plugins/vidya-keystore/` or the structure Tauri's plugin guide prescribes)
- Read the Tauri 2 "mobile plugin" guide for the installed version and follow its structure exactly.
- Kotlin: an AES-256-GCM key in `AndroidKeyStore` with alias `vidya-wrap-key`, no user authentication required (the app must sync in the background of an open app), `setRandomizedEncryptionRequired(true)`.
- Commands (called from Rust, not from JavaScript): `wrap(plain: bytes) -> blob`, `unwrap(blob) -> bytes`, `deleteKey()`. Blob = IV + ciphertext.
- Restrict the plugin's permission so the webview **cannot** call it (only Rust).
- If the Keystore key is lost (for example after certain device resets), `unwrap` fails: return a specific error.
- Use only Android platform APIs (no extra Kotlin libraries), to keep the APK small.

#### Task 2 — `platform/android.rs`
- `data_dir()`: Tauri app data dir (private storage) + `/data`.
- `load_or_create_db_key()`: `key.blob` in data dir → unwrap; if missing create 32 random bytes, wrap, write atomically.
- `store_secret`/`load_secret`/`delete_secret`: blobs in `data/secrets/`.
- Other methods (including `volume_info`, `removable_drives`, keep awake): Unsupported.
- Start sequence (P2.4) works on Android; failure kind `SecureStorage` shows "This phone's secure storage could not be opened. Remove and reinstall Vidya, then sign in again." (the data is on the office computer).

#### Task 3 — Database in client mode
- `Services` built in `Mode::Client`; device id created on first start.
- `vidya-db` compiles for Android with vendored OpenSSL; if the Android build fails on OpenSSL, report the exact error and stop. Record the APK size change in `docs/SIZE.md`.

#### Task 4 — Device key
`DeviceKeyStore` in `vidya-client`: Ed25519 secret stored via `store_secret("device-key")`; public key available; created on first launch.

#### Task 5 — Wipe
`wipe_local_data()`: close the pool, delete the database files and secrets, delete the Keystore key, reset in-memory state. Used by P8.4 when the device is removed.

#### Task 6 — Tests
- Rust unit tests with the fake platform.
- An instrumented check the developer runs: debug command `debug_storage_selftest` (**M**, debug builds only) that wraps/unwraps, opens the DB, writes and reads a row, reports timings.

### Done when (agent)
- [ ] `npm run verify` passes; Android CI check green
- [ ] APK size row added to `docs/SIZE.md`

### Done when (developer)
- [ ] Run the storage self-test on the phone (the agent tells you how to trigger it): all steps OK
- [ ] Force stop and reopen the app: the database still opens

---

## P8.3 — Phone discovery, sign-in and approval

| | |
|---|---|
| Phase | 8 Android |
| Depends on | P8.2 |
| Size | medium |
| Runs on | MacBook (server) + phone on the same Wi-Fi |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (sections 7 and 8), docs/API.md (`discover_server`, `remote_sign_in`) |

### Goal
A teacher or accountant types `sierra@vaani` and a password on the phone, the app finds the school's licensed office computer on the Wi-Fi, shows the matching code, waits for the principal, pins the certificate and downloads the first snapshot.

### Tasks

#### Task 1 — Multicast lock
Extend the Kotlin plugin with `acquireMulticastLock()` / `releaseMulticastLock()` (WifiManager multicast lock, tag "vidya"). Rust acquires it only during mDNS discovery and releases it afterwards.

#### Task 2 — Commands (mobile)
- `discover_server(school_code)`: runs `vidya-client::discover`; on success remembers address in `sync_state.server_address`; tries the remembered address first (`GET /health` with fingerprint check and `licensed: true`) before discovering.
- `remote_sign_in(username, password)`: requires `@school` in the username on first sign-in (then remembered). Flow: discover → `POST /auth/sign-in` → if waiting, return `{ status: "waiting_for_approval", matchCode }` and start polling in Rust every 2 seconds for up to 10 minutes, emitting `sync-changed` with the approval state; when approved, pin the full fingerprint in `sync_state`, store the session, cache the password hash in `auth_cache` (Argon2id of the typed password computed on the phone), run `initial_snapshot`, then return signed in.
- `MustChangePassword` path: phone shows the password change screen and calls a mobile `set_first_password` that goes to `POST /auth/first-password`.
- After pinning: any certificate mismatch → error `sync.error.not_your_school` ("This is not your school's office computer.").
- `MultipleServers` from discovery (no pin yet) → error `sync.error.multiple_servers` ("Two office computers answered for this school. Ask the principal to switch Vidya off on the old computer.").

#### Task 3 — Screens (`src/views/mobile/`, React)
- `MobileSignIn.jsx`: username, password, show/hide, "Sign in".
- `Looking.jsx`: "Looking for your school's office computer…" with a spinner; failure screen with checks: on school Wi-Fi (not mobile data), office computer on with Vidya open, not a guest network; "Try again".
- `Approval.jsx`: large match code "482 913", text "Ask the principal to allow this phone on the office computer. The code there must match.", countdown, Cancel.
- `Downloading.jsx`: progress by snapshot pages.
- Errors from `AppError.message`.
- The flow state lives in a `useReducer` in `MobileApp.jsx`, driven by the command result and `sync-changed` events.

#### Task 4 — Tests
- Rust: sign-in state machine with a fake server client (waiting → allowed → snapshot; refused; expired; mismatch; multiple servers).
- React Testing Library: approval screen renders the code with a space; failure screen shows the checklist.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Mac app as server: sign in as sierra on the phone; codes match; Allow; the phone downloads data
- [ ] Refuse on another attempt; the phone says it was refused
- [ ] Windows VM (bridged) as server: the phone finds it and signs in
- [ ] Put the phone on mobile data only: the looking screen fails with the helpful checklist

---

## P8.4 — Phone sync, offline work and removal

| | |
|---|---|
| Phase | 8 Android |
| Depends on | P8.3 |
| Size | medium |
| Runs on | MacBook (server) + phone |
| Read first | AGENTS.md, docs/SYNC_PROTOCOL.md (section 6), docs/PERMISSIONS.md (rules 8 and 10) |

### Goal
Staff work normally with no connection, changes sync automatically when back on school Wi-Fi, the top bar always shows the sync state, rejected changes are explained, and removed phones wipe themselves.

### Tasks

#### Task 1 — Background sync controller (Rust, mobile)
- Triggers: after every local write (debounce 500 ms), app foreground (Tauri window focus or lifecycle event; verify), WebSocket `upTo`, network change to Wi-Fi (Kotlin plugin callback using ConnectivityManager network callback), "Sync now", every 30 seconds while in foreground without WebSocket.
- Sync is attempted only when the active network is Wi-Fi or Ethernet (not cellular), so no mobile data is used; the state shows `offline` with "Not on school Wi-Fi" otherwise.
- One cycle at a time (mutex); results emitted as `sync-changed` `SyncStatusDto { state: synced|syncing|waiting|offline|sign_in_again|removed, lastSyncAt, waiting, rejected }`.
- `data-changed` emitted after pulls so open views refresh (the `useQuery` `refreshOn` option from P7.5).

#### Task 2 — Offline sign-in
- Sign-in on the phone when the office computer is not reachable: verify against `auth_cache` for that username; success creates a local session only; lockout counting locally (5 tries → must connect to the office computer).
- On the next successful sync, if the server says the user is inactive, locked, reset or signed out → clear `auth_cache` for that user and sign out with the server's message.
- Idle timeout on phones: 15 minutes.

#### Task 3 — Status bar and rejected changes
- Mobile top bar `SyncPill` component: "Synced 2 min ago", "Syncing…", "3 changes waiting", "Not on school Wi-Fi", "Sign in again".
- Warning banner when the oldest unsynced change is older than 7 days, or the clock offset is over 5 minutes ("This phone's clock is wrong. Set date and time to automatic.").
- "Not saved" bottom sheet listing rejected changes with plain reasons (e.g. "Attendance for VI-A on 12 Sep was not saved: this class is no longer assigned to you.") and a "Got it" button that clears them.

#### Task 4 — Online-only admission
Accountant "Add student": enabled only when the last successful server contact was < 60 seconds ago; otherwise disabled with "Connect to school Wi-Fi to add a new admission. Editing existing students works offline." Submitting calls `POST /students` and inserts the returned student locally.

#### Task 5 — Removal
HTTP 410 at any time → `wipe_local_data()` → screen "This phone was removed by the principal. Its Vidya data has been deleted." → back to sign-in.

#### Task 6 — Tests
- Rust controller tests with fake transport: offline queueing, backoff, rejection handling, 410 wipe, 401 sign-in-again, no attempt on cellular.
- React Testing Library: `SyncPill` text for each state.

### Done when (agent)
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Airplane mode on: sierra marks attendance and enters marks; the pill says changes waiting
- [ ] Wi-Fi back: synced within seconds; the office computer shows them
- [ ] Wi-Fi off with mobile data on: the pill says "Not on school Wi-Fi" and no sync is attempted
- [ ] Principal removes sierra's V-B section on the office computer while the phone is offline and she marks V-B attendance; after sync the phone lists it as not saved
- [ ] Remove the phone on the office computer; the phone wipes itself

---

## P8.5 — Phone screens, PDFs and privacy protections

| | |
|---|---|
| Phase | 8 Android |
| Depends on | P8.4, P5.2 |
| Size | large |
| Runs on | MacBook (server) + phone |
| Read first | AGENTS.md, src/AGENTS.md, docs/UI_GUIDE.md, docs/spike/PRINTING.md, docs/DECISIONS.md (D25, D28), docs/PERMISSIONS.md |

### Goal
Complete teacher and accountant experiences on a phone, comfortable one-handed, with receipts and report cards shared as PDFs, and sensitive screens protected from screenshots, inside the APK size budget.

### Tasks

#### Task 1 — Teacher screens (`src/views/mobile/teacher/`)
- Home: today's sections with "Take attendance" state, marks progress per exam.
- Attendance: large cards (≥ 58 px), tap to cycle P/A/L, "All present", sticky Save bar with counts; read-only past days.
- Marks: one student at a time **or** table mode toggle; numeric keyboard (`inputMode="numeric"`), "AB" button, Next moves to the next cell, error cells highlighted after save.
- Students (own sections): list and detail without fees; tap mobile number to call (`telHref` from `src/core/links.js` through opener).
- Report card view and "Share PDF".
Reuse desktop components and hooks where the layout allows; mobile-only layouts get their own components, not copies of desktop logic.

#### Task 2 — Accountant screens (`src/views/mobile/accountant/`)
- Home: collected today, receipts today, top dues.
- Fees: search, collect with Full balance and One term, receipt screen with "Share PDF" and "Share on WhatsApp" (Android share sheet; the user chooses the app).
- Day book for this phone's receipts and all receipts known locally.
- Students: list, detail with fees, edit fields, Add student (online only).
- No backup, settings, users or devices screens on phones (office computer only).

#### Task 3 — PDFs on Android
Implement the D25 Android method (from P5.1): render the same React document components from P5.2 into the print root, produce a PDF file in the app's cache folder, open the Android share sheet through the Kotlin plugin (`FileProvider` content URI with a narrowly scoped `file_paths.xml`). Delete shared PDFs older than 1 day at start. Commands `export_receipt_pdf` and `export_report_card_pdf` (add the second to API.md).

#### Task 4 — Screenshot protection
Kotlin plugin `setSecure(on)`: adds/clears `FLAG_SECURE` on the activity window. Rust calls it when the router enters fees, receipts, marks, report cards, students detail, credentials; clears it on other screens (home and attendance are allowed). The router calls it from a `useEffect` on view change. Verify it blocks screenshots and the recent-apps preview.

#### Task 5 — No personal data in logs
Release Android builds: log level warn, no request bodies, no names. Add a test that the log configuration for mobile release is warn.

#### Task 6 — Accessibility and small screens
Test at 360×640 and with the system font size set to largest: nothing cut off, buttons reachable. Hindi text wraps correctly.

#### Task 7 — Tests
React Testing Library for each mobile view with mocked commands: renders teacher DTO without fee keys; save buttons disabled while saving; share button calls the export command.

### Done when (agent)
- [ ] `npm run verify` passes
- [ ] arm64 APK size recorded in `docs/SIZE.md` (hard limit 30 MB, target 20 MB)

### Done when (developer)
- [ ] As a teacher on the phone: attendance and marks for a whole section in under 3 minutes, one-handed
- [ ] As an accountant: collect a fee and share the receipt PDF on WhatsApp to yourself; Hindi names correct
- [ ] Try a screenshot on the receipt screen: blocked
- [ ] Largest font size: screens still usable

---

## P9.1 — Full Hindi

| | |
|---|---|
| Phase | 9 Language |
| Depends on | P8.5 |
| Size | medium |
| Runs on | MacBook + phone |
| Read first | AGENTS.md, docs/UI_GUIDE.md ("Translation", "Wording rules"), docs/PRODUCT.md (glossary), src/locales/en.json, crates/vidya-core/locales/en.json |

### Before you start (developer)
Find a Hindi-speaking reviewer, ideally a school teacher or clerk, who can spend 3–4 hours checking a spreadsheet.

### Goal
Every screen, message, error, printout and installer text available in simple everyday Hindi, reviewed by a person, with Hindi amounts and dates in words where schools expect them.

### Tasks

#### Task 1 — Complete coverage check
- Make `scripts/check-i18n.mjs` strict now: any JSX text child or user-visible attribute string (`title`, `placeholder`, `aria-label`, `alt`) not coming from `t()` fails, including single words (allow only punctuation, numbers, symbols like `₹ % / · —`, and lines marked `i18n-ignore`); any `t('…')` key missing from either locale fails. Add a Rust test that scans `crates/**/src/**/*.rs` for message keys used with `DomainError`/`ServiceError` constructors and checks both core locale files.
- Fix every gap.

#### Task 2 — Draft translations
- Translate every key in `src/locales/hi.json` and `crates/vidya-core/locales/hi.json`.
- Style: simple school Hindi as spoken in offices (उपस्थिति, फ़ीस, रसीद, अंक, कक्षा, छात्र, अभिभावक, शिक्षक, प्रधानाचार्य, बकाया, जमा, रद्द करें, बैकअप, पेन ड्राइव). Avoid heavy Sanskritised words. Keep English where schools normally use it (UPI, Aadhaar, APAAR, UDISE, RTE, PDF, Excel, Wi-Fi).
- Keep `{placeholders}` exactly; the test fails if placeholder names differ between en and hi.
- Numbers stay 0–9 with Indian grouping.

#### Task 3 — Review file
Generate `locales/hi-review.csv` (key, English, Hindi, screen or context, reviewed, reviewer note), UTF-8 with BOM so Excel opens Hindi correctly. Add `scripts/apply-hi-review.mjs` that reads the reviewed CSV back and updates `hi.json` for rows marked reviewed, reporting changed keys.

#### Task 4 — Hindi words and dates
- `vidya-core::words::amount_in_words_hi(n)`: a complete table for 0–99 (Hindi number words are irregular; write the table as a constant array with a comment that it must be checked by the reviewer, and include it in `hi-review.csv` as 100 rows), plus सौ, हज़ार, लाख, करोड़. Receipt text "रुपये … मात्र".
- `format_date_hi` with Hindi month names (जनवरी … दिसंबर) and `date_in_words_hi` for TCs.
- Tests with vectors the **reviewer confirms**; until confirmed, mark them in KNOWN_ISSUES.

#### Task 5 — Per-user language and print language
- Language already stored per user; ensure phones take the user's language after sync and the sign-in screen has its own switch stored on the device.
- Print language setting en/hi/both applied to all document components; "both" shows "Receipt / रसीद" style labels.
- Excel exports: headers follow the user's language.

#### Task 6 — Installers
NSIS installer: add Hindi language (verify Tauri's NSIS language option names). macOS: add `hi` localisation for the app name and Info.plist descriptions (verify how Tauri supports localised plist strings; if not supported, record in KNOWN_ISSUES). Record installer size change in `docs/SIZE.md`.

#### Task 7 — Font size check
Hindi text is often longer: check every screen at 1000×680 desktop and 360×640 mobile in Hindi; fix overflow with wrapping, never by shrinking below 13 px.

### Done when (agent)
- [ ] `npm run verify` passes with strict i18n checks
- [ ] `hi-review.csv` generated

### Done when (developer)
- [ ] Reviewer completes the CSV; run the apply script; commit
- [ ] Switch to Hindi on desktop and phone: every screen, error and printout is Hindi
- [ ] Print a receipt in "both" languages

---

## P10.1 — Size gate and Windows installer

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P9.1 |
| Size | medium |
| Runs on | MacBook; GitHub Actions; Windows VM and a real Windows PC |
| Read first | AGENTS.md, docs/PLATFORMS.md, docs/SIZE.md, docs/DECISIONS.md (D24, D28) |

### Before you start (developer)
Decide how you will sign Windows builds. Options to research (the agent will summarise current requirements): an OV/EV code signing certificate on a hardware token or cloud HSM from a certificate authority, or Microsoft's cloud signing service if you are eligible. Signing needs the certificate to be usable from GitHub Actions.

### Goal
Every shipped artefact confirmed within the 30 MB budget with a final audit, and a Windows installer that installs cleanly, keeps data on uninstall, sets firewall rules and permissions, and is signed.

### Tasks

#### Task 1 — Final size audit
- `cargo install cargo-bloat --locked`. Run it for the desktop release build on macOS and in CI for Windows, and for the Android client build; list the 20 largest crates for each.
- Frontend: size of `dist/` by file; fonts: subset Noto Sans Devanagari and Noto Sans to the characters Vidya needs (all Devanagari, Basic Latin, Latin-1, ₹ and common punctuation) using a font subsetting tool run once by the developer, not at build time; document the command and keep WOFF2 output.
- Check duplicate crate versions (`cargo tree -d`) and unify where possible; confirm only one TLS crypto provider and one compression library are in each build.
- Report the full `docs/SIZE.md` history as a before/after table for: Windows installer, Windows installed folder, macOS arm64 `.dmg` and installed `.app`, macOS x64 `.dmg` and installed `.app`, Android arm64 APK and AAB.
- **Hard limit 30 MB for every row (D28).** Aim for at least 5 MB of headroom so future versions fit. If any row is over 25 MB, list what could be removed or made smaller (with estimated savings) and stop for a decision.
- The release workflow (P10.3) runs `scripts/check-size.mjs` on every artefact before creating the release.

#### Task 2 — ProgramData permissions
The NSIS install hook creates `C:\ProgramData\Vidya` and grants the local Users group modify rights on it (use `icacls` with the well-known SID for Users so it works on Hindi-language Windows; verify syntax on Microsoft Learn), so any Windows account on the office PC can run Vidya on the same data. The uninstall hook must **not** delete `C:\ProgramData\Vidya` (data and local backups).

#### Task 3 — Upgrade behaviour
- Installing a new version over an old one: the installer closes a running Vidya (check the Tauri NSIS option or hook), keeps data, and the app's start sequence makes the safety backup before migrations (P6.1), then syncs it to connected backup drives (P6.2).
- Downgrade protection: if the database `user_version` is newer than the app, show the "newer version" screen (P2.4) instead of starting.

#### Task 4 — Signing hook
- Configure Tauri's Windows signing so it runs a **custom sign command** read from an environment variable (verify the config key for the installed version). Without the variable, builds stay unsigned for development.
- Document in `RELEASE.md` how to set up the chosen signing option in GitHub Actions secrets, without putting any secret in the repository.
- Verify signatures in CI with `signtool verify /pa` (or the method your signing option documents).

#### Task 5 — Installer polish
Start menu and desktop shortcuts, app name "Vidya", publisher name from a build-time variable, English and Hindi installer languages (P9.1), license/EULA page from `legal/EULA.txt` (placeholder text flagged in KNOWN_ISSUES for the lawyer), per-machine install requiring admin. The installer shows the installed size correctly in "Apps & features" (set the NSIS estimated size if Tauri does not).

#### Task 6 — Windows test script
Write `docs/TESTING_WINDOWS.md`: steps for the VM and for a real Windows 10 and Windows 11 PC — install, first run activation, firewall rules present, second Windows user account can open the same school, sleep during school hours, printing to a USB printer, "Back up now" as the accountant, adding `D:\` and a pen drive as backup drives and plugging the pen drive in later, uninstall keeps data, reinstall opens data, upgrade over the previous version, SmartScreen with signed build, installer download size and installed size under 30 MB.

### Done when (agent)
- [ ] Size report with before/after numbers; every artefact ≤ 30 MB
- [ ] `RELEASE.md` Windows section written
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] `docs/TESTING_WINDOWS.md` passed in the VM
- [ ] Passed on a real Windows PC (borrowed, second-hand, or the pilot school's)

---

## P10.2 — Mac signing and notarization

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P10.1 |
| Size | small |
| Runs on | MacBook; GitHub Actions macos-latest |
| Read first | AGENTS.md, docs/PLATFORMS.md, docs/DECISIONS.md (D28), docs/SIZE.md |

### Before you start (developer)
1. Join the Apple Developer Program.
2. Create a "Developer ID Application" certificate (Xcode → Settings → Accounts → Manage Certificates, or the developer website). Keep the exported `.p12` and its password outside the repository.
3. Create an app-specific password for your Apple ID, or an App Store Connect API key for notarization (the agent will explain which the installed Tauri version supports best).

### Goal
Signed and notarized Apple Silicon and Intel `.dmg` files (or one universal `.dmg` if D28 allows it) that open on any Mac without security warnings, built locally or in CI, each within 30 MB.

### Tasks
1. Read Tauri's macOS code signing and notarization guide for the installed version. List the exact environment variables it uses.
2. Hardened runtime on. Entitlements file with only what is needed: network server and client. Decide on App Sandbox: explain to the developer that sandboxing affects writing backups to backup drives and pen drives, DiskArbitration access and Keychain access, and recommend **no sandbox** for Developer ID distribution unless every feature is proven to work sandboxed. Record the decision as D26 after approval.
3. Info.plist keys from P7.2 are present in the signed app.
4. Local signing: `npm run tauri build -- --target aarch64-apple-darwin` and `--target x86_64-apple-darwin` (or `universal-apple-darwin` if `docs/SIZE.md` shows it fits with headroom) with the variables set from the developer's Keychain; notarize; staple; verify with `spctl --assess --type open --context context:primary-signature -v` on each .dmg and `codesign --verify --deep --strict` on each app (check the right verification commands in Apple's documentation). Run the size check on the signed outputs (signing adds a little).
5. CI: job on `macos-latest` for version tags: import the certificate from base64 secret into a temporary keychain, build both architectures, notarize, staple, size-check, upload the `.dmg` files named `Vidya-<version>-apple-silicon.dmg` and `Vidya-<version>-intel.dmg`. Delete the temporary keychain at the end even on failure.
6. `RELEASE.md` macOS section: creating certificates, secrets to add, how to rotate them, and how to tell schools which file to download (Apple menu → About This Mac → Chip "Apple M…" means Apple Silicon; "Intel" means Intel).
7. Test: each notarized .dmg downloaded through a browser (so it is quarantined) opens on a Mac that has never run Vidya; local network permission prompt appears with the right text; data folder survives replacing the app with a newer version.

### Done when (agent)
- [ ] CI notarized builds work on a test tag (e.g. `v0.9.0-rc1`), each under 30 MB
- [ ] `RELEASE.md` macOS section written

### Done when (developer)
- [ ] Download the CI Apple Silicon `.dmg` in Safari, open it, drag to Applications, open Vidya: no "cannot be opened" warning
- [ ] If you can borrow an Intel Mac, repeat with the Intel `.dmg`

---

## P10.3 — Android signing and release pipeline

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P10.2 |
| Size | medium |
| Runs on | MacBook; GitHub Actions |
| Read first | AGENTS.md, docs/PLATFORMS.md, docs/DECISIONS.md (D28), RELEASE.md |

### Goal
One tagged release produces a signed Windows installer, notarized Mac `.dmg` files, a signed arm64 APK and AAB, with checksums and a size report, in a draft GitHub release. The release fails if any file is over 30 MB.

### Tasks

#### Task 1 — Android release keystore
- Explain to the developer how to create an upload keystore with `keytool` (exact command, key alias `vidya`, validity 25+ years), and to enrol in Play App Signing (Google holds the app signing key; the upload key can be reset if lost). Also explain that APKs shared directly (outside Play) are signed with the upload key, so **losing it means directly installed phones cannot update** without uninstalling.
- Store: keystore file base64, store password, key alias, key password as GitHub secrets. Never in the repo.
- Configure signing through the Gradle project Tauri generated, reading values from environment variables or a git-ignored `keystore.properties` (follow Tauri's Android signing guide for the installed version).
- Enable R8 shrinking and resource shrinking for release if Tauri's Gradle project supports it without breaking the Kotlin plugin (keep rules for the plugin classes); record the APK size change.

#### Task 2 — Versioning
- One version source: `package.json` version, synced into `tauri.conf.json` and Cargo workspace version by `scripts/set-version.mjs <x.y.z>`.
- Android `versionCode` derived from the version (e.g. `major*10000 + minor*100 + patch`), checked to increase.
- The app shows its version in Settings → About (desktop) and the account sheet (phone).

#### Task 3 — Release workflow `.github/workflows/release.yml`
On tags `v*`:
1. `verify` job on macOS and Windows (`npm run verify`).
2. `windows` job: signed NSIS installer (P10.1).
3. `macos` job: notarized `.dmg` files (P10.2).
4. `android` job on `ubuntu-latest`: Java, Android SDK and NDK (well-known actions; read READMEs), Rust Android targets, signed arm64 APK and AAB.
5. `release` job: download all artefacts, run `node scripts/check-size.mjs` on all of them (fail over 30 MB), write `SIZES.md` and a SHA-256 checksums file, create a draft GitHub release with all files and a changelog section from `CHANGELOG.md`.
- The Provider Tool is never built here (assert the workflow does not reference `provider/`).
- Secrets list with where to get each value in `RELEASE.md`.

#### Task 4 — Release checklist
`RELEASE.md` final section: before tagging (all PROGRESS checks, TESTING docs passed, CHANGELOG, `docs/SIZE.md` updated), tagging, after the workflow (download, verify checksums, install on VM, real PC, Mac, phone), publishing (make the release non-draft only after checks), sending schools the installer (no automatic updates; D24).

### Done when (agent)
- [ ] A test tag produces all files, the size report and checksums in a draft release
- [ ] `RELEASE.md` complete

### Done when (developer)
- [ ] Install each file from the draft release on its platform
- [ ] Keystore backed up in two safe places (not the MacBook alone)

---

## P10.4 — Security and privacy review

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P10.3 |
| Size | large |
| Runs on | MacBook |
| Read first | AGENTS.md, every document in docs/, .claude/agents/security-reviewer.md |

### Goal
Find and fix security problems before a real school uses Vidya, and give schools a clear privacy notice.

### Tasks

#### Task 1 — Systematic review
Run the security-reviewer subagent separately on each area and write `docs/SECURITY_REVIEW.md` with a findings table (id, severity, area, file:line, issue, fix, status):
1. Every Tauri command: permission, office-computer-only rule, validation, role-shaped output.
2. Every HTTP route: signed device check, `Origin::Phone` always applied, rate limit, body limits, error bodies free of internals.
3. SQL: search for any string-built SQL.
4. Frontend: no `dangerouslySetInnerHTML`, `innerHTML` or similar anywhere; no user data in `href`/`src` except `telHref`; CSP; capabilities; no dev commands in release (`load_sample_school`, `debug_*`, env public key override, `VIDYA_DEV_ALLOW_DEMO_SERVER`). Check the production bundle text for these names.
5. Secrets: repository history (`git log -p` search for key-like strings), logs, crash output, exports, temp files.
6. Crypto: key sizes, nonces, zeroization, Argon2 parameters, randomness sources.
7. Licensing and the server permit: editing the license table, swapping the public key in the binary (note: cannot be fully prevented; confirm tampering detection behaves as designed), copying the data folder to another computer (server must not start), any code path that starts the server, discovery or sync endpoints without a `ServerPermit`, two office computers for one school.
8. Sync: forged roles or origins, replay, pinning, revoked devices, teachers receiving fee data through any path (snapshot, pull, WebSocket, error messages, alerts), phones pushing office-only changes, any connection to a non-private address.
9. Backups and backup drives: plaintext leftovers, restore into another school, damaged files, destinations receiving anything except encrypted files and the manifest, manifest contents, deletion of files not in the manifest, drive letters reused by another volume, staff running backups when switched off.
10. Android: FLAG_SECURE coverage, allowBackup, exported components, FileProvider paths, logs, no sync over cellular.
11. Dependencies: `cargo deny check advisories`, `npm audit --omit=dev`.

#### Task 2 — Fix
Fix every critical and high finding with a test; medium where reasonable; list the rest with reasons in KNOWN_ISSUES. Re-run the size check after fixes.

#### Task 3 — Privacy in the product
- Settings → Privacy page (principal, and a short version for staff): what personal data Vidya stores (students, parents, staff), where (office computer, approved phones by role, encrypted backups on the office computer and on backup drives the school chooses), who can see what, that nothing is sent to the software provider or any internet service, how to export and how to delete.
- "Delete student permanently" (principal, office computer only, only if the student has no receipts, marks for past sessions or TC; otherwise explain that accounting and school records must be kept and offer "mark as left"). Deletion removes the student and related attendance and marks, writes a change-log event without personal data, and syncs a delete to phones. Explain on screen that older backups still contain the student until they are removed by retention.
- `legal/PRIVACY_NOTICE.md` draft for schools, noting India's Digital Personal Data Protection Act, 2023, and marked "Draft – to be reviewed by a lawyer". Do not claim legal compliance.

### Done when (agent)
- [ ] `docs/SECURITY_REVIEW.md` has no open critical or high items
- [ ] `npm run verify` passes

### Done when (developer)
- [ ] Read the review summary
- [ ] Send the privacy notice and EULA drafts to a lawyer

---

## P10.5 — Load test and real-device testing

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P10.4 |
| Size | medium |
| Runs on | MacBook, Windows PC, phones, pilot school |
| Read first | AGENTS.md, docs/TESTING_STRATEGY.md, docs/TESTING_WINDOWS.md (written in P10.1) |

### Goal
Prove Vidya is fast with a large school and survives a real school day, and give the developer a complete manual test script.

### Tasks

#### Task 1 — Large school generator
`vidya-testkit::LargeSchool::build(seed)`: 2,000 students, 30 sections, 60 staff, 5 sessions of attendance (every school day), 4 exams per session with marks, 20,000 receipts, 500 cancellations. A debug command or example writes it to a data folder for manual testing.

#### Task 2 — Performance tests
- Service-level timings (release build, `cargo test --release -- --ignored perf` style or a bench example): every read service used by a screen < 300 ms on the MacBook; report numbers.
- Frontend: time from navigation to rendered content for each screen < 1 second on the Windows VM with the large school (developer measures with a timing overlay added in debug builds using `performance.now()` around the first render after data arrives). Check the large tables (fee register, students, marks) with React's Profiler in a debug build and fix slow renders with memoisation or paging, not new libraries.
- Sync: teacher snapshot for 2 sections and accountant snapshot for 2,000 students over Wi-Fi; report seconds (target < 10 s teacher, < 30 s accountant) and bytes.
- Backups: time and size of a backup of the large school, and time to sync it to a USB 2.0 pen drive.
- Database size after generation and backup file size.
- Fix hotspots (indexes, queries, pagination) with before/after numbers.

#### Task 3 — Manual test script `docs/TESTING.md`
Write step-by-step scenarios, each marked VM / real Windows PC / Mac / phone:
1. Fresh install, activation, setup wizard, printing slips (Windows and Mac separately).
2. Three phones approved (two teachers, one accountant).
3. A full school day: attendance for all sections on phones, fee collection on PC and phone, marks entry, report cards.
4. Wi-Fi drop in the middle of saving attendance and fees.
5. Office computer switched off during the day, then on.
6. Phone clock set 2 hours wrong.
7. Teacher's section changed while her phone is offline.
8. Phone removed by principal.
9. Power cut during a backup and during a backup drive sync (pull the plug of the VM / force quit, or pull out the pen drive mid-copy).
10. Backup drives: accountant presses "Back up now" and "Sync backup drives"; pen drive left unplugged for a week then plugged in; restore from the pen drive onto a second computer, Windows to Mac and Mac to Windows.
11. Copy the data folder to another computer: Vidya shows the "different computer" screen and phones cannot sync with it.
12. Session change at year end.
13. Principal forgets password; reset code.
14. Office PC replaced: replacement activation, restore from a backup drive, phones reconnect; old PC switched off (and what phones show if it is not).
15. Upgrade to a new version with phones still on the old version (the phone must show "Update Vidya on this phone" if the protocol version differs; implement the version check if missing).
Each step lists expected results.

#### Task 4 — Pilot school plan
`docs/PILOT.md`: two-week pilot checklist (installation day, choosing a backup pen drive or external disk with the principal, training for principal, accountant and teachers in 30 minutes each, daily "Back up now" check, feedback form questions, what to measure: time to collect a fee, time to take attendance, problems per day).

### Done when (agent)
- [ ] Performance numbers reported and targets met or explained
- [ ] `docs/TESTING.md` and `docs/PILOT.md` written

### Done when (developer)
- [ ] `docs/TESTING.md` run end to end on real devices
- [ ] Pilot school running for two weeks; problems fixed with `/fix`

---

## P10.6 — Play Store listing and review access

| | |
|---|---|
| Phase | 10 Release |
| Depends on | P10.5 |
| Size | medium |
| Runs on | MacBook |
| Read first | AGENTS.md, docs/PRODUCT.md, legal/PRIVACY_NOTICE.md, RELEASE.md, docs/DECISIONS.md (D28, D29, D30) |

### Before you start (developer)
- Google Play Console **organisation** account (needs a D-U-N-S number for your business).
- A website where you can publish a privacy policy page.

### Goal
Everything needed to publish the Vidya phone app on Google Play. (There is no Google sign-in or Drive verification in Edition 2, because Vidya uses no Google services.)

### Tasks

#### Task 1 — Current requirements (research with sources)
Read Google's current Play Console help pages and list, with links and dates checked: target API level requirement, new developer account testing requirements (closed testing with testers for a period, if still required), data safety form sections, privacy policy requirements, permission declarations relevant to Vidya (network, multicast, local network access), app content questionnaires (target audience: staff of schools, not children), app size limits for AAB and install size, and any policy about apps that require external hardware or software to function. Write `docs/PLAY_STORE.md`.

#### Task 2 — Store listing
English and Hindi: app name, short description (≤ 80 characters), full description (clearly: "Vidya for Android works with the Vidya software installed on your school's office computer, over the school Wi-Fi. It does not work on its own and does not use the internet."), feature list, contact email, category Education.

#### Task 3 — Data safety answers
Based on the actual code (cite files): data types stored on device, whether any data is collected by the developer (none), sharing (none), encryption in transit (HTTPS on the local network) and at rest, deletion (removing the phone or uninstalling deletes local data).

#### Task 4 — Reviewer access
Play reviewers cannot reach a school's office computer. Implement a **demo build flavour**, compiled only when `VIDYA_REVIEW_DEMO=1`: the Android app includes a built-in in-process sample server (using `vidya-testkit` sample school and the real services in server mode on `127.0.0.1`, with a demo-only permit type that cannot exist in normal builds) with fixed demo logins shown on the sign-in screen. It must never be in normal release builds (CI assertion that checks both the feature flags and the absence of server crates in the release `cargo tree` for Android). Report the demo APK size separately; it does not ship to schools but must still be under Play's limits. Explain whether to upload the demo flavour to a separate testing track or to provide it to review through Play's app access instructions, according to the requirements found in Task 1.

#### Task 5 — Privacy policy page
`legal/privacy-policy.html`: simple static page based on the privacy notice, for the developer's website. Marked draft until the lawyer reviews it.

#### Task 6 — Screenshots plan
List of 8 phone screenshots and 4 desktop images to make with the **sample school only** (no real names): teacher attendance, marks, accountant fee collection, receipt share, sync status, approval code, and the desktop home, fee register and backup drives card.

#### Task 7 — Build
Signed release AAB from the release workflow; upload instructions for the internal testing track.

### Done when (agent)
- [ ] `docs/PLAY_STORE.md`, listing texts and data safety answers written with sources
- [ ] Demo flavour builds and is absent from normal release builds (CI check)

### Done when (developer)
- [ ] Internal testing track installed on your phones
- [ ] Privacy policy published after lawyer review